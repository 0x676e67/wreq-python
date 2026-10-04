"""
Custom redirect policy example.

Demonstrates how to use custom redirect policies with Python callbacks.
"""

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
