import datetime
from typing import final

__all__ = ["Runtime"]


@final
class Runtime:
    """Shared Tokio runtime whose workers start when constructed.

    Clients and active work keep it alive; the last owner releases it automatically.
    No-steal clients keep a fixed worker, without CPU pinning.
    """

    def __init__(
        self,
        *,
        workers: int | None = None,
        work_steal: bool = True,
        thread_name: str | None = None,
        max_blocking_threads: int | None = None,
        thread_keep_alive: datetime.timedelta | None = None,
    ) -> None:
        """Thread counts must be positive; thread_keep_alive is a nonnegative timedelta.

        workers defaults to available CPU parallelism, or 1 if unavailable.
        thread_name defaults to the package name, "wreq-python".
        Thread names cannot contain NUL characters.
        Blocking-pool settings apply to each worker runtime in no-steal mode.
        None preserves Tokio's blocking-pool defaults.
        """
        ...
