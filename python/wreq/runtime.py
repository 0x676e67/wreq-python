from typing import final

__all__ = ["Runtime"]


@final
class Runtime:
    """Lazy Tokio runtime wrapper. No-steal clients keep a fixed worker, without CPU pinning."""

    def __init__(
        self,
        *,
        workers: int | None = None,
        work_steal: bool = True,
        thread_name: str = "wreq",
        max_blocking_threads: int | None = None,
        thread_keep_alive: float | None = None,
    ) -> None:
        """Thread counts must be positive; thread_keep_alive is in seconds.

        workers defaults to TOKIO_WORKER_THREADS or available parallelism.
        Blocking-pool settings apply to each worker runtime in no-steal mode.
        None preserves Tokio's blocking-pool defaults.
        """
        ...

    @property
    def workers(self) -> int: ...

    @property
    def work_steal(self) -> bool: ...

    @property
    def thread_name(self) -> str: ...

    @property
    def max_blocking_threads(self) -> int | None: ...

    @property
    def thread_keep_alive(self) -> float | None: ...

    @property
    def closed(self) -> bool: ...

    def shutdown_timeout(self, timeout: float) -> None:
        """Shut down an unused custom runtime; repeated calls are harmless.

        Release clients (including closed clients), responses, streams and tasks
        first, or RuntimeError is raised. timeout is finite nonnegative seconds,
        per worker in no-steal mode. Running blocking work can outlive the timeout.
        """
        ...
