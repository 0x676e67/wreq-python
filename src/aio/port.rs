use std::{
    cell::RefCell,
    io::{Read, Write},
    sync::{
        Arc, Mutex, MutexGuard, PoisonError, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

use pyo3::{intern, prelude::*, sync::PyOnceLock};

use super::coroutine::Slot;

/// Wakes queued for one event loop, delivered by a single bell per batch.
pub(crate) struct Port {
    queue: Mutex<Vec<Arc<Slot>>>,
    bell: Bell,
    /// Cleared once the loop drops its drain; later wakes are discarded.
    open: AtomicBool,
}

enum Bell {
    /// A socket pair whose read end the loop watches. The read end is `None`
    /// when Python owns it, as with a proactor loop.
    Socket { tx: Socket, rx: Option<Socket> },
    /// A loop that cannot watch the socket: the waking thread schedules the drain.
    Loop(Py<PyAny>),
}

#[cfg(unix)]
type Socket = std::os::unix::net::UnixStream;

#[cfg(windows)]
type Socket = std::net::TcpStream;

/// Resolves the asyncio futures of queued coroutines on the loop thread.
#[pyclass(frozen)]
struct Drain(Arc<Port>);

thread_local! {
    /// Ports of the loops that ran on this thread, keyed by loop address.
    static PORTS: RefCell<Vec<(usize, Weak<Port>)>> = const { RefCell::new(Vec::new()) };
}

// ===== impl Port =====

impl Port {
    /// Return the running loop and its port, opening the port on first use.
    pub(super) fn current(py: Python<'_>) -> PyResult<(Bound<'_, PyAny>, Arc<Port>)> {
        static GET_RUNNING_LOOP: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
        let event_loop = GET_RUNNING_LOOP
            .get_or_try_init(py, || {
                py.import("asyncio")?
                    .getattr("get_running_loop")
                    .map(Bound::unbind)
            })?
            .bind(py)
            .call0()?;

        // A loop drops its drain when closed or freed, so a reused address never matches.
        let key = event_loop.as_ptr() as usize;
        let cached = PORTS.with_borrow_mut(|ports| {
            ports.retain(|(_, port)| port.upgrade().is_some_and(|port| port.is_open()));
            ports
                .iter()
                .find(|(id, _)| *id == key)
                .and_then(|(_, port)| port.upgrade())
        });
        let port = match cached {
            Some(port) => port,
            None => {
                let port = Self::open(&event_loop)?;
                PORTS.with_borrow_mut(|ports| ports.push((key, Arc::downgrade(&port))));
                port
            }
        };
        Ok((event_loop, port))
    }

    /// Open a port for `event_loop`, preferring a socket the loop watches itself.
    fn open(event_loop: &Bound<'_, PyAny>) -> PyResult<Arc<Port>> {
        let py = event_loop.py();
        if let Ok((tx, rx)) = socket_pair() {
            #[cfg(windows)]
            if event_loop.hasattr(intern!(py, "_proactor"))? {
                use std::os::windows::io::IntoRawSocket;

                let port = Self::new(Bell::Socket { tx, rx: None });
                let drain = Drain(port.clone());
                let watched = proactor_watch(py)
                    .and_then(|watch| watch.call1((event_loop, rx.into_raw_socket(), drain)));
                if watched.is_ok() {
                    return Ok(port);
                }
                return Ok(Self::new(Bell::Loop(event_loop.clone().unbind())));
            }

            let fd = raw(&rx);
            let port = Self::new(Bell::Socket { tx, rx: Some(rx) });
            let drain = Drain(port.clone());
            // Loops without reader support fall back to scheduling drains.
            if event_loop
                .call_method1(intern!(py, "add_reader"), (fd, drain))
                .is_ok()
            {
                return Ok(port);
            }
        }
        Ok(Self::new(Bell::Loop(event_loop.clone().unbind())))
    }

    fn new(bell: Bell) -> Arc<Port> {
        Arc::new(Port {
            queue: Mutex::default(),
            bell,
            open: AtomicBool::new(true),
        })
    }

    #[inline]
    pub(super) fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }

    #[inline]
    fn lock(&self) -> MutexGuard<'_, Vec<Arc<Slot>>> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Queue a woken slot, ringing the bell when the queue was empty.
    pub(super) fn push(self: &Arc<Self>, slot: Arc<Slot>) {
        let first = {
            let mut queue = self.lock();
            if !self.is_open() {
                return;
            }
            queue.push(slot);
            queue.len() == 1
        };
        if first {
            self.ring();
        }
    }

    fn ring(self: &Arc<Self>) {
        match &self.bell {
            // A full socket already holds an unread ring. A lost ring would leave the
            // queue non-empty, so later pushes never ring: retry interrupted writes.
            Bell::Socket { tx, .. } => loop {
                match (&*tx).write(&[1]) {
                    Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                    _ => break,
                }
            },
            Bell::Loop(event_loop) => {
                Python::try_attach(|py| {
                    let drain = Drain(self.clone());
                    let scheduled = event_loop
                        .bind(py)
                        .call_method1(intern!(py, "call_soon_threadsafe"), (drain,));
                    // A closed loop never runs the drain.
                    if scheduled.is_err() {
                        self.close();
                    }
                });
            }
        }
    }

    /// Discard queued wakes, breaking the cycle between queued slots and the port.
    fn close(&self) {
        let queue = {
            let mut queue = self.lock();
            self.open.store(false, Ordering::Release);
            std::mem::take(&mut *queue)
        };
        drop(queue);
    }
}

// ===== impl Drain =====

#[pymethods]
impl Drain {
    fn __call__(&self, py: Python<'_>) -> PyResult<()> {
        let port = &self.0;
        // Clear the bell before taking the queue so no ring is lost; a short read
        // means the bell is empty.
        if let Bell::Socket { rx: Some(rx), .. } = &port.bell {
            let mut buf = [0; 64];
            while matches!((&*rx).read(&mut buf), Ok(n) if n == buf.len()) {}
        }

        let mut slots = std::mem::take(&mut *port.lock()).into_iter();
        while let Some(slot) = slots.next() {
            let Some(waiter) = slot.take_waiter() else {
                continue;
            };
            if let Err(err) = release(waiter.bind(py)) {
                // A raising callback, such as a signal handler, ends this drain. Requeue
                // this wake and the rest; the next drain skips futures already done.
                slot.set_waiter(waiter);
                port.lock().splice(..0, std::iter::once(slot).chain(slots));
                port.ring();
                return Err(err);
            }
        }
        Ok(())
    }
}

impl Drop for Drain {
    fn drop(&mut self) {
        // A socket port's drain is registered with the loop and owns the port's lifetime.
        if let Bell::Socket { .. } = self.0.bell {
            self.0.close();
        }
    }
}

/// Resume the task awaiting `waiter`, unless it was already cancelled.
fn release(waiter: &Bound<'_, PyAny>) -> PyResult<()> {
    let py = waiter.py();
    if !waiter.call_method0(intern!(py, "done"))?.is_truthy()? {
        waiter.call_method1(intern!(py, "set_result"), (py.None(),))?;
    }
    Ok(())
}

#[cfg(unix)]
fn socket_pair() -> std::io::Result<(Socket, Socket)> {
    let (tx, rx) = Socket::pair()?;
    tx.set_nonblocking(true)?;
    rx.set_nonblocking(true)?;
    Ok((tx, rx))
}

/// Connect a loopback pair, accepting only the connection made here.
#[cfg(windows)]
fn socket_pair() -> std::io::Result<(Socket, Socket)> {
    use std::net::{Ipv4Addr, TcpListener};

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let tx = Socket::connect(listener.local_addr()?)?;
    let local = tx.local_addr()?;
    let rx = loop {
        let (rx, peer) = listener.accept()?;
        if peer == local {
            break rx;
        }
    };
    tx.set_nodelay(true)?;
    tx.set_nonblocking(true)?;
    rx.set_nonblocking(true)?;
    Ok((tx, rx))
}

#[cfg(unix)]
fn raw(socket: &Socket) -> i64 {
    use std::os::fd::AsRawFd;
    i64::from(socket.as_raw_fd())
}

#[cfg(windows)]
fn raw(socket: &Socket) -> std::os::windows::io::RawSocket {
    use std::os::windows::io::AsRawSocket;
    socket.as_raw_socket()
}

/// Watch a socket with overlapped receives on a proactor loop, like its own
/// self-pipe; the receive is re-armed before each drain runs.
#[cfg(windows)]
fn proactor_watch(py: Python<'_>) -> PyResult<&Bound<'_, PyAny>> {
    static WATCH: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
    WATCH
        .get_or_try_init(py, || {
            PyModule::from_code(
                py,
                c"import socket

def watch(loop, fileno, drain):
    sock = socket.socket(fileno=fileno)

    def rearm(fut=None):
        if fut is not None and (fut.cancelled() or fut.exception() or not fut.result()):
            sock.close()
            return
        try:
            loop._proactor.recv(sock, 4096).add_done_callback(rearm)
        except Exception:
            sock.close()
            return
        if fut is not None:
            drain()

    rearm()
",
                c"wreq/_proactor.py",
                c"wreq._proactor",
            )?
            .getattr("watch")
            .map(Bound::unbind)
        })
        .map(|watch| watch.bind(py))
}
