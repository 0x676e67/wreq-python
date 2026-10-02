import asyncio

import pytest
import wreq
from wreq import redirect

from cancellation_test import local_server

client = wreq.Client(redirect=redirect.Policy.limited(10))


@pytest.mark.asyncio
@pytest.mark.parametrize("fail", [False, True])
async def test_custom_redirect_callback(fail):
    def callback(attempt):
        if fail:
            raise ValueError("redirect callback failed")
        return attempt.stop()

    async with (
        local_server() as (url, connections),
        wreq.Client(proxies=[], redirect=redirect.Policy.custom(callback)) as client,
    ):
        task = asyncio.create_task(client.get(url))
        _, writer = await asyncio.wait_for(connections.get(), 5)
        writer.write(
            b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\n\r\n"
        )
        await writer.drain()
        if fail:
            with pytest.raises(
                wreq.exceptions.RequestError,
                match="ValueError: redirect callback failed",
            ):
                await asyncio.wait_for(task, 5)
        else:
            response = await asyncio.wait_for(task, 5)
            assert response.status.is_redirection()
            await response.close()


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_request_disable_redirect():
    response = await client.get(
        "https://google.com",
        redirect=redirect.Policy.none(),
    )
    assert response.status.is_redirection()
    assert response.url == "https://google.com/"


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_request_enable_redirect():
    response = await client.get(
        "https://google.com",
        redirect=redirect.Policy.limited(),
    )
    assert response.status.is_success()
    assert response.url == "https://www.google.com/"


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_client_request_disable_redirect():
    client = wreq.Client(redirect=redirect.Policy.none())
    response = await client.get("https://google.com")
    assert response.status.is_redirection()
    assert response.url == "https://google.com/"


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_client_request_enable_redirect():
    response = await client.get("https://google.com")
    assert response.status.is_success()
    assert response.url == "https://www.google.com/"


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_client_redirec_history():
    url = "https://google.com/"
    client = wreq.Client(redirect=redirect.Policy.limited())
    response = await client.get(url)
    assert response.status.is_success()
    assert response.url == "https://www.google.com/"

    history = response.history
    assert len(history) == 1
    assert history[0].url == "https://www.google.com/"
    assert history[0].previous == url
