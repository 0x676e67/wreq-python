//! HTTPS loopback echo server for the Python client benchmarks.

#![deny(unsafe_code)]

use std::{
    convert::Infallible,
    env,
    error::Error,
    io::{self, Write},
    sync::Arc,
    time::Duration,
};

use btls::{
    asn1::Asn1Time,
    bn::BigNum,
    ec::{EcGroup, EcKey},
    hash::MessageDigest,
    nid::Nid,
    pkey::PKey,
    ssl::{AlpnError, Ssl, SslAcceptor, SslMethod, SslVersion, select_next_proto},
    x509::{X509, X509Name, extension::SubjectAlternativeName},
};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode, body::Incoming, server::conn, service::service_fn};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use tokio::{net::TcpListener, sync::oneshot, task::JoinSet, time::timeout};
use tokio_btls::SslStream;

type BoxError = Box<dyn Error + Send + Sync>;

#[derive(Clone, Copy)]
enum Protocol {
    Http1,
    Http2,
}

impl Protocol {
    fn name(self) -> &'static str {
        match self {
            Self::Http1 => "h1",
            Self::Http2 => "h2",
        }
    }

    fn alpn(self) -> &'static [u8] {
        match self {
            Self::Http1 => b"http/1.1",
            Self::Http2 => b"h2",
        }
    }

    fn alpn_wire(self) -> &'static [u8] {
        match self {
            Self::Http1 => b"\x08http/1.1",
            Self::Http2 => b"\x02h2",
        }
    }
}

fn main() -> Result<(), BoxError> {
    let mut protocol = Protocol::Http1;
    let mut workers = 4;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--protocol" => {
                protocol = match args.next().as_deref() {
                    Some("h1") => Protocol::Http1,
                    Some("h2") => Protocol::Http2,
                    _ => return Err(io::Error::other("--protocol requires h1 or h2").into()),
                };
            }
            "--workers" => {
                workers = args
                    .next()
                    .ok_or_else(|| io::Error::other("--workers requires a positive integer"))?
                    .parse::<usize>()?;
                if workers == 0 {
                    return Err(io::Error::other("--workers must be greater than zero").into());
                }
            }
            _ => return Err(io::Error::other(format!("unknown argument: {arg}")).into()),
        }
    }

    let acceptor = Arc::new(tls_acceptor(protocol)?);
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()?
        .block_on(run(protocol, workers, acceptor))
}

/// Generates a fresh, self-signed certificate for this loopback benchmark only.
fn tls_acceptor(protocol: Protocol) -> Result<SslAcceptor, BoxError> {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
    let key = PKey::from_ec_key(EcKey::generate(&group)?)?;
    let mut name = X509Name::builder()?;
    name.append_entry_by_text("CN", "localhost benchmark")?;
    let name = name.build();

    let mut cert = X509::builder()?;
    cert.set_version(2)?;
    let serial = BigNum::from_u32(1)?.to_asn1_integer()?;
    cert.set_serial_number(&serial)?;
    cert.set_subject_name(&name)?;
    cert.set_issuer_name(&name)?;
    cert.set_pubkey(&key)?;
    let not_before = Asn1Time::days_from_now(0)?;
    let not_after = Asn1Time::days_from_now(1)?;
    cert.set_not_before(&not_before)?;
    cert.set_not_after(&not_after)?;
    let san = SubjectAlternativeName::new()
        .dns("localhost")
        .ip("127.0.0.1")
        .build(&cert.x509v3_context(None, None))?;
    cert.append_extension(&san)?;
    cert.sign(&key, MessageDigest::sha256())?;

    let mut builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;
    builder.set_certificate(&cert.build())?;
    builder.set_private_key(&key)?;
    builder.check_private_key()?;
    builder.set_min_proto_version(Some(SslVersion::TLS1_3))?;
    builder.set_max_proto_version(Some(SslVersion::TLS1_3))?;
    builder.set_alpn_select_callback(move |_, client| {
        select_next_proto(protocol.alpn_wire(), client).ok_or(AlpnError::ALERT_FATAL)
    });
    Ok(builder.build())
}

async fn run(
    protocol: Protocol,
    workers: usize,
    acceptor: Arc<SslAcceptor>,
) -> Result<(), BoxError> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let metadata = serde_json::json!({
        "url": format!("https://{}", listener.local_addr()?),
        "protocol": protocol.name(),
        "workers": workers,
    });
    println!("{metadata}");
    io::stdout().flush()?;

    // A separate thread avoids a blocking stdin task holding Tokio shutdown open.
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    std::thread::Builder::new()
        .name("benchmark-stdin".into())
        .spawn(move || {
            let result = io::stdin().read_line(&mut String::new());
            let _ = shutdown_tx.send(result);
        })?;

    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            result = &mut shutdown_rx => {
                result??;
                break;
            }
            accepted = listener.accept() => {
                let (socket, _) = accepted?;
                // Match the Rust benchmark: do not delay small echo responses.
                socket.set_nodelay(true)?;
                let acceptor = acceptor.clone();
                connections.spawn(async move {
                    let ssl = Ssl::new(acceptor.context())?;
                    let mut stream = SslStream::new(ssl, socket)?;
                    timeout(Duration::from_secs(10), std::pin::Pin::new(&mut stream).accept()).await??;
                    if stream.ssl().selected_alpn_protocol() != Some(protocol.alpn()) {
                        return Err(io::Error::other("client did not negotiate the required ALPN").into());
                    }

                    let io = TokioIo::new(stream);
                    match protocol {
                        Protocol::Http1 => {
                            conn::http1::Builder::new()
                                .timer(TokioTimer::new())
                                .keep_alive(true)
                                .serve_connection(io, service_fn(echo))
                                .await?;
                        }
                        Protocol::Http2 => {
                            conn::http2::Builder::new(TokioExecutor::new())
                                .timer(TokioTimer::new())
                                .keep_alive_interval(Duration::from_secs(30))
                                .serve_connection(io, service_fn(echo))
                                .await?;
                        }
                    }
                    Ok::<_, BoxError>(())
                });
            }
            Some(result) = connections.join_next(), if !connections.is_empty() => {
                if let Err(error) = result? {
                    eprintln!("connection error: {error}");
                }
            }
        }
    }

    drop(listener);
    connections.abort_all();
    while let Some(result) = connections.join_next().await {
        match result {
            Ok(Err(error)) => eprintln!("connection error: {error}"),
            Err(error) if !error.is_cancelled() => return Err(error.into()),
            _ => {}
        }
    }
    Ok(())
}

async fn echo(request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    match request.into_body().collect().await {
        Ok(body) => Ok(Response::new(Full::new(body.to_bytes()))),
        Err(error) => {
            eprintln!("request body error: {error}");
            let mut response =
                Response::new(Full::new(Bytes::from_static(b"request body read failed")));
            *response.status_mut() = StatusCode::BAD_REQUEST;
            Ok(response)
        }
    }
}
