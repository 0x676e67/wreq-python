import asyncio
import datetime
import wreq


async def fetch(label, url, **kwargs):
    print(f"\n--- {label} ---")
    try:
        await wreq.get(url, **kwargs)
    except wreq.Error as e:
        # Every wreq error derives from `wreq.Error`.
        print(f"Caught: {type(e).__name__}: {e}")


async def main():
    await fetch("BuilderError (bad URL)", "htt://httpbin.org/status/404")
    await fetch(
        "TimeoutError (timeout)",
        "https://httpbin.io/delay/10",
        timeout=datetime.timedelta(seconds=1),
    )
    await fetch("ConnectionError (refused)", "http://127.0.0.1:9999")


if __name__ == "__main__":
    asyncio.run(main())
