# :repeat: Redirects & Error Handling

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

Handle various request exceptions:

```python
import datetime
import wreq
import asyncio
import wreq.exceptions as exceptions

wreq_errors = (
    exceptions.BodyError,
    exceptions.BuilderError,
    exceptions.ConnectionError,
    exceptions.ConnectionResetError,
    exceptions.DecodingError,
    exceptions.RedirectError,
    exceptions.TimeoutError,
    exceptions.StatusError,
    exceptions.RequestError,
    exceptions.UpgradeError,
)


async def test_timeout_error():
    print("\n--- TimeoutError (timeout) ---")
    try:
        await wreq.get(
            "https://httpbin.io/delay/10", timeout=datetime.timedelta(seconds=1)
        )
    except wreq_errors as e:
        print(f"Caught: {type(e).__name__}: {e}")
    except Exception as e:
        print(f"Other error: {type(e).__name__}: {e}")


async def test_connection_error():
    print("\n--- ConnectionError (refused) ---")
    try:
        await wreq.get("http://127.0.0.1:9999")
    except wreq_errors as e:
        print(f"Caught: {type(e).__name__}: {e}")
    except Exception as e:
        print(f"Other error: {type(e).__name__}: {e}")


async def main():
    await test_timeout_error()
    await test_connection_error()


if __name__ == "__main__":
    asyncio.run(main())
```
