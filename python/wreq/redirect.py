from typing import Callable, Sequence, final

from .header import HeaderMap
from . import StatusCode

__all__ = ["Policy", "Attempt", "Action", "History"]


@final
class Policy:
    """
    Represents the redirect policy for HTTP requests.

    Clients do not follow redirects by default. Use limited() to enable
    redirect following with a limit (10 when max is omitted).
    """

    @staticmethod
    def limited(max: int | None = None) -> "Policy":
        """
        Create a Policy with a maximum number of redirects.

        An error will be returned if the max is reached.

        Args:
            max: Maximum number of redirects to follow

        Returns:
            Policy: A redirect policy with the specified limit

        Example:
            ```python
            import wreq
            from wreq import Client, redirect
            policy = redirect.Policy.limited(5)
            client = Client(redirect=policy)
            ```
        """
        ...

    @staticmethod
    def none() -> "Policy":
        """
        Create a Policy that does not follow any redirect.

        Returns:
            Policy: A redirect policy that doesn't follow redirects

        Example:
            ```python
            from wreq import Client, redirect

            policy = redirect.Policy.none()
            client = Client(redirect=policy)
            ```
        """
        ...

    @staticmethod
    def custom(callback: Callable[["Attempt"], "Action"]) -> "Policy":
        """
        Create a custom Policy using the passed function.

        Args:
            callback: A callable that takes an Attempt and returns an Action

        Returns
            Policy: A custom redirect policy

        Example:
            ```python
            from wreq import Client, redirect

            def policy(attempt: redirect.Attempt) -> redirect.Action:
                if len(attempt.previous) > 5:
                    return attempt.error("too many redirects")
                elif "example.com" in attempt.next:
                    return attempt.stop()
                else:
                    return attempt.follow()

            policy = redirect.Policy.custom(policy)
            client = Client(redirect=policy)
            ```
        """
        ...

    def __str__(self) -> str: ...


@final
class Attempt:
    """
    A type that holds information on the next request and previous requests
    in redirect chain.
    """

    status: StatusCode
    """
    The HTTP status code of the redirect response.
    """

    headers: HeaderMap
    """
    The headers of the redirect response.
    """

    next: str
    """
    The next URI to which the client is being redirected.
    """

    previous: Sequence[str]
    """
    The list of previous URIs in the redirect chain.
    """

    def follow(self) -> "Action":
        """
        Returns an action meaning the client should follow the next URI.

        Returns:
            Action: An action to follow the redirect
        """
        ...

    def stop(self) -> "Action":
        """
        Returns an action meaning the client should not follow the next URI.

        The 30x response will be returned as the result.

        Returns:
            Action: An action to stop following redirects
        """
        ...

    def error(self, message: str) -> "Action":
        """
        Returns an action failing the redirect with an error.

        The error will be returned for the result of the sent request.

        Args:
            message: Error message

        Returns:
            Action: An action that will raise an error
        """
        ...

    def __str__(self) -> str: ...


@final
class Action:
    """
    An action to perform when a redirect status code is found.

    This class is typically created by calling methods on Attempt:
    - attempt.follow()
    - attempt.stop()
    - attempt.error(message)
    """

    ...

    def __str__(self) -> str: ...


@final
class History:
    """
    An entry in the redirect history.
    """

    status: int
    """Get the status code of the redirect response."""

    url: str
    """Get the URL of the redirect response."""

    previous: str
    """Get the previous URL before the redirect response."""

    headers: HeaderMap
    """Get the headers of the redirect response."""

    def __str__(self) -> str: ...
