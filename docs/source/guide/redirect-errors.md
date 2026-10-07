# Redirects & Error Handling

!!! info "On this page"
    - Custom redirect policy
    - Error handling

### Custom Redirect Policy

Control redirect behavior with custom policies:

```python
import asyncio
from wreq import Client, redirect
from wreq.redirect import Attempt, Action


def custom_policy(attempt: Attempt) -> Action:
    """Custom redirect policy that blocks example.com redirects."""
    print(
        f"Redirect to: {attempt.next} (status: {attempt.status}) (headers: {attempt.headers})"
    )

    # Block redirects to example.com
    if "example.com" in attempt.next:
        return attempt.stop()

    # Limit redirect chain length
    if len(attempt.previous) > 5:
        return attempt.error("Too many redirects")

    # Allow other redirects
    return attempt.follow()


async def main():
    # Create a client with custom redirect policy
    policy = redirect.Policy.custom(custom_policy)
    client = Client(redirect=policy)

    # Test with a URL that redirects
    async with client.get("http://httpbin.io/redirect/3") as response:
        print(f"Final URL: {response.url}")
        print(f"Status: {response.status}")


if __name__ == "__main__":
    asyncio.run(main())
```

### Error Handling

Every wreq error derives from `wreq.Error`; catch a subclass to handle one failure:

```python
import asyncio
import datetime
import wreq


async def main():
    try:
        await wreq.get(
            "https://httpbin.io/delay/10", timeout=datetime.timedelta(seconds=1)
        )
    except wreq.TimeoutError as e:
        print(f"Timed out: {e}")
    except wreq.ConnectionError as e:
        print(f"Could not connect: {e}")
    except wreq.Error as e:
        print(f"Request failed: {type(e).__name__}: {e}")


if __name__ == "__main__":
    asyncio.run(main())
```

`RequestError` groups transport failures, so it catches connection errors and
timeouts alike. `ConnectionError`, `ConnectionResetError` and `TimeoutError` also
derive from the builtins of the same name. See
[`wreq.exceptions`](../api/exceptions.md) for the full hierarchy.
