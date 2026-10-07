import datetime
from enum import Enum, auto
from typing import final

__all__ = ["Runtime", "Scheduler"]


@final
class Scheduler(Enum):
    """How a runtime schedules client work."""

    WORK_STEALING = auto()
    """Worker threads share client work and steal it from each other."""

    PER_WORKER = auto()
    """Each worker runs its own single-thread runtime; a client stays on one worker."""

    CURRENT_THREAD = auto()
    """No worker threads: blocking calls drive its IO, taking turns when threads
    share it, and nothing runs between calls.

    Best with a client and runtime per thread; only blocking clients can use it.
    Upload iterators run on whichever thread drives it: blocking wreq calls from
    them raise RuntimeError, and one that waits for another call on this runtime
    never returns.
    """


@final
class Runtime:
    """Shared Tokio runtime; worker schedulers start their threads when constructed.

    Clients, responses and active work keep it alive; the last owner releases
    it automatically.
    Per-worker clients keep a fixed worker, without CPU pinning.
    """

    def __init__(
        self,
        *,
        scheduler: Scheduler = Scheduler.WORK_STEALING,
        workers: int | None = None,
        thread_name: str | None = None,
        max_blocking_threads: int | None = None,
        thread_keep_alive: datetime.timedelta | None = None,
    ) -> None:
        """Thread counts must be positive; thread_keep_alive is a nonnegative timedelta.

        workers defaults to available CPU parallelism, or 1 if unavailable, and
        must be None with CURRENT_THREAD.
        thread_name defaults to the package name, "wreq-python".
        Thread names cannot contain NUL characters.
        Blocking-pool settings apply to each worker runtime with PER_WORKER.
        None preserves Tokio's blocking-pool defaults.
        """
        ...
