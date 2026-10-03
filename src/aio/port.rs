use std::{
    cell::RefCell,
    io::{Read, Write},
    sync::{
        Arc, Mutex, MutexGuard, PoisonError, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

use pyo3::{PyTraverseError, PyVisit, intern, prelude::*, sync::PyOnceLock};

use super::coroutine::Slot;

/// Wakes queued for one event loop, delivered by a single bell per batch.
pub(crate) struct Port {
    queue: Mutex<Vec<Arc<Slot>>>,
    bell: Bell,
    /// Cleared once the loop drops its drain or keeper, or a scheduled drain cannot be
    /// queued; later wakes are discarded.
    open: AtomicBool,
}

enum Bell {
    /// A socket pair whose read end the loop watches. The read end is `None`
    /// when Python owns it, as with a proactor loop.
    Socket { tx: Socket, rx: Option<Socket> },
    /// A loop that cannot watch the socket: the waking thread schedules the drain,
    /// and a [`Keeper`] holds the port until the loop closes.
    Loop(Py<PyAny>),
}

#[cfg(unix)]
type Socket = std::os::unix::net::UnixStream;

#[cfg(windows)]
type Socket = std::net::TcpStream;

/// Resolves the asyncio futures of queued coroutines on the loop thread.
#[pyclass(frozen, module = "wreq")]
struct Drain(Arc<Port>);

/// Owns a scheduled-drain port through a renewing timer, which the loop discards when
/// closed. It reports the port's loop reference so an unclosed loop can be collected.
#[pyclass(module = "wreq")]
struct Keeper(Option<Arc<Port>>);

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

        // A loop drops its drain or keeper when closed or freed, so a reused address
        // never matches.
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
                return Self::scheduled(event_loop);
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
        Self::scheduled(event_loop)
    }

    /// Open a port whose wakes schedule drains on `event_loop`.
    fn scheduled(event_loop: &Bound<'_, PyAny>) -> PyResult<Arc<Port>> {
        let port = Self::new(Bell::Loop(event_loop.clone().unbind()));
        let keeper = Bound::new(event_loop.py(), Keeper(Some(port.clone())))?;
        Keeper::arm(&keeper, event_loop)?;
        Ok(port)
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
    fn __repr__(&self) -> &'static str {
        match self.0.bell {
            Bell::Socket { .. } => "<wreq.Drain socket>",
            Bell::Loop(_) => "<wreq.Drain scheduled>",
        }
    }

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
            // Resume the task awaiting the waiter, unless it was already cancelled.
            let done = waiter
                .bind(py)
                .call_method0(intern!(py, "done"))
                .and_then(|done| done.is_truthy());
            let released = match done {
                Ok(false) => waiter
                    .bind(py)
                    .call_method1(intern!(py, "set_result"), (py.None(),))
                    .map(drop),
                done => done.map(drop),
            };
            if let Err(err) = released {
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

// ===== impl Keeper =====

impl Keeper {
    /// Seconds between renewals; any delay works, as only a closing loop drops the timer.
    const RENEW_SECS: f64 = 3600.0;

    fn arm(keeper: &Bound<'_, Keeper>, event_loop: &Bound<'_, PyAny>) -> PyResult<()> {
        let py = keeper.py();
        event_loop
            .call_method1(intern!(py, "call_later"), (Self::RENEW_SECS, keeper))
            .map(drop)
    }
}

#[pymethods]
impl Keeper {
    fn __call__(slf: &Bound<'_, Self>) -> PyResult<()> {
        let event_loop = match slf.borrow().0.as_deref() {
            Some(Port {
                bell: Bell::Loop(event_loop),
                ..
            }) => event_loop.clone_ref(slf.py()),
            _ => return Ok(()),
        };
        Self::arm(slf, event_loop.bind(slf.py()))
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        // The keeper is the only object reporting this reference, so it is counted once.
        if let Some(Port {
            bell: Bell::Loop(event_loop),
            ..
        }) = self.0.as_deref()
        {
            visit.call(event_loop)?;
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        if let Some(port) = self.0.take() {
            port.close();
        }
    }
}

impl Drop for Keeper {
    fn drop(&mut self) {
        self.__clear__();
    }
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


class Watch:
    # Pending receives reference the watch only through bound methods, so a closed
    # loop frees it, and with it the drain, without waiting for the cycle collector.

    def __init__(self, loop, sock, drain):
        self.loop = loop
        self.sock = sock
        self.drain = drain

    def arm(self):
        self.loop._proactor.recv(self.sock, 4096).add_done_callback(self.ready)

    def ready(self, fut):
        if fut.cancelled() or fut.exception() or not fut.result():
            self.sock.close()
            return
        try:
            self.arm()
        except Exception:
            self.sock.close()
            return
        self.drain()


def watch(loop, fileno, drain):
    sock = None
    try:
        sock = socket.socket(fileno=fileno)
        # A first receive that fails lets the caller fall back to scheduled drains.
        Watch(loop, sock, drain).arm()
    except BaseException:
        if sock is None:
            socket.close(fileno)
        else:
            sock.close()
        raise
",
                c"wreq/_proactor.py",
                c"wreq._proactor",
            )?
            .getattr("watch")
            .map(Bound::unbind)
        })
        .map(|watch| watch.bind(py))
}
