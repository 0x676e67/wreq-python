import asyncio

import pytest
import wreq
from wreq.cookie import Cookie

client = wreq.Client()


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_get_cookie():
    jar = wreq.Jar()
    url = "http://localhost:8080/cookies"
    cookie = Cookie("test_cookie", "12345", domain="localhost", path="/cookies")
    jar.add(cookie, url)
    cookie = jar.get("test_cookie", url)
    assert cookie is not None
    assert cookie.name == "test_cookie"
    assert cookie.value == "12345"
    assert cookie.domain == "localhost"
    assert cookie.path == "/cookies"

    jar.clear()

    jar.add("test_cookie=12345; Path=/cookies; Domain=localhost", url)
    cookie = jar.get("test_cookie", url)
    assert cookie is not None
    assert cookie.name == "test_cookie"
    assert cookie.value == "12345"
    assert cookie.domain == "localhost"
    assert cookie.path == "/cookies"

    client = wreq.Client(cookie_provider=jar)
    response = await client.get(url)
    assert response.status.is_success()
    assert "test_cookie" in await response.text()


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_get_all_cookies():
    jar = wreq.Jar()
    url = "http://localhost:8080/cookies"
    cookie1 = Cookie("test_cookie1", "12345", domain="localhost", path="/cookies")
    cookie2 = Cookie("test_cookie2", "67890", domain="localhost", path="/cookies")
    jar.add(cookie1, url)
    jar.add(cookie2, url)

    cookies = jar.get_all()
    assert len(cookies) == 2
    cookie_names = [cookie.name for cookie in cookies]
    assert "test_cookie1" in cookie_names
    assert "test_cookie2" in cookie_names

    client = wreq.Client(cookie_provider=jar)
    response = await client.get(url)
    assert response.status.is_success()
    body = await response.text()
    assert "test_cookie1" in body
    assert "test_cookie2" in body


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_remove_cookie():
    jar = wreq.Jar()
    client = wreq.Client(cookie_provider=jar)
    url = "http://localhost:8080/cookies"
    cookie = Cookie("test_cookie", "12345", domain="localhost", path="/cookies")
    jar.add(cookie, url)

    # Verify the cookie is set
    response = await client.get(url)
    assert response.status.is_success()
    assert "test_cookie" in await response.text()

    # Remove the cookie
    jar.remove("test_cookie", url)

    # Verify the cookie is removed
    cookie = jar.get("test_cookie", url)
    assert cookie is None

    response = await client.get(url)
    assert response.status.is_success()
    assert "test_cookie" not in await response.text()


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_clear_cookies():
    jar = wreq.Jar()
    client = wreq.Client(cookie_provider=jar)
    url = "http://localhost:8080/cookies"
    cookie1 = Cookie("test_cookie1", "12345", domain="localhost", path="/cookies")
    cookie2 = Cookie("test_cookie2", "67890", domain="localhost", path="/cookies")

    jar.add(cookie1, url)
    jar.add(cookie2, url)

    # Verify cookies are set

    response = await client.get(url)
    assert response.status.is_success()
    body = await response.text()
    assert "test_cookie1" in body
    assert "test_cookie2" in body

    # Clear all cookies
    jar.clear()

    # Verify all cookies are cleared
    assert jar.get("test_cookie1", url) is None
    assert jar.get("test_cookie2", url) is None

    response = await client.get(url)
    assert response.status.is_success()
    body = await response.text()
    assert "test_cookie1" not in body
    assert "test_cookie2" not in body


@pytest.mark.asyncio
async def test_client_cookie_jar_accessor():
    url = "http://localhost:8080/cookies"

    # 1) If a cookie_provider is passed, client.cookie_jar should return it (shared storage).
    jar = wreq.Jar()
    client = wreq.Client(cookie_provider=jar)
    assert client.cookie_jar is not None
    client.cookie_jar.add("test_cookie=12345; Path=/cookies; Domain=localhost", url)
    resp = await client.get(url)
    assert resp.status.is_success()
    assert "test_cookie" in await resp.text()

    # 2) If cookie_store=True is used without explicit provider, client.cookie_jar should exist.
    client2 = wreq.Client(cookie_store=True)
    assert client2.cookie_jar is not None
    client2.cookie_jar.add("test_cookie=abc; Path=/cookies; Domain=localhost", url)
    resp2 = await client2.get(url)
    assert resp2.status.is_success()
    assert "test_cookie" in await resp2.text()

    # 3) If both cookie_provider and cookie_store=True are set, the provider must win.
    jar3 = wreq.Jar()
    client3 = wreq.Client(cookie_provider=jar3, cookie_store=True)
    assert client3.cookie_jar is not None
    client3.cookie_jar.add("test_cookie=zzz; Path=/cookies; Domain=localhost", url)
    resp3 = await client3.get(url)
    assert resp3.status.is_success()
    assert "test_cookie" in await resp3.text()


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_cookie_storage_from_server():
    """Test that cookies set by the server are actually stored in the jar."""
    jar = wreq.Jar()
    client = wreq.Client(cookie_provider=jar)

    # Request httpbin to set a cookie (returns 302 redirect, which is expected)
    set_url = "http://localhost:8080/cookies/set?session_id=test123&user=alice"
    response = await client.get(set_url)

    async with response:
        # Verify cookies are stored in the jar
        url = "http://localhost:8080"
        session_cookie = jar.get("session_id", url)
        user_cookie = jar.get("user", url)

        assert session_cookie is not None, "session_id cookie should be stored in jar"
        assert session_cookie.name == "session_id"
        assert session_cookie.value == "test123"

        assert user_cookie is not None, "user cookie should be stored in jar"
        assert user_cookie.name == "user"
        assert user_cookie.value == "alice"

    # Verify that cookies are sent back in subsequent requests
    response = await client.get("http://localhost:8080/cookies")
    assert response.status.is_success()
    body = await response.text()
    assert "session_id" in body
    assert "test123" in body
    assert "user" in body
    assert "alice" in body

    # Verify all cookies in the jar
    all_cookies = jar.get_all()
    assert len(all_cookies) >= 2
    cookie_names = [cookie.name for cookie in all_cookies]
    assert "session_id" in cookie_names
    assert "user" in cookie_names


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_cookie_value_update_from_server():
    """Test that cookie values can be updated by server responses."""
    jar = wreq.Jar()
    client = wreq.Client(cookie_provider=jar)
    url = "http://localhost:8080"

    # First request: set initial cookie value
    set_url1 = "http://localhost:8080/cookies/set?user_id=initial_value"
    response1 = await client.get(set_url1)

    async with response1:
        # Verify initial cookie is stored
        user_id_cookie = jar.get("user_id", url)
        assert user_id_cookie is not None, "user_id cookie should be stored in jar"
        assert (
            user_id_cookie.value == "initial_value"
        ), "Initial cookie value should be 'initial_value'"

    # Verify cookie is sent in subsequent request
    cookies_url = "http://localhost:8080/cookies"
    response2 = await client.get(cookies_url)
    body1 = await response2.text()
    assert "user_id" in body1
    assert "initial_value" in body1

    # Second request: update cookie value
    set_url2 = "http://localhost:8080/cookies/set?user_id=updated_value"
    response3 = await client.get(set_url2)

    async with response3:
        # Verify cookie value is updated
        user_id_cookie_updated = jar.get("user_id", url)
        assert (
            user_id_cookie_updated is not None
        ), "user_id cookie should still be in jar"
        assert (
            user_id_cookie_updated.value == "updated_value"
        ), "Cookie value should be updated to 'updated_value'"

    # Verify updated cookie is sent in subsequent request
    response4 = await client.get(cookies_url)
    body2 = await response4.text()
    assert "user_id" in body2
    assert "updated_value" in body2, "Updated cookie value should be sent to server"
    assert "initial_value" not in body2, "Old cookie value should not be present"

    # Verify only one user_id cookie exists in jar
    all_cookies = jar.get_all()
    user_id_cookies = [c for c in all_cookies if c.name == "user_id"]
    assert (
        len(user_id_cookies) == 1
    ), "Should have exactly one user_id cookie after update"
    assert user_id_cookies[0].value == "updated_value"


@pytest.mark.asyncio
async def test_cookie_dict_sends_one_header():
    requests = asyncio.Queue()

    async def handle(reader, writer):
        requests.put_nowait(await reader.readuntil(b"\r\n\r\n"))
        writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        await writer.drain()
        writer.close()

    server = await asyncio.start_server(handle, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    try:
        resp = await client.get(url, cookies={"a": "1", "b": "2"})
        await resp.close()
        head = (await asyncio.wait_for(requests.get(), 5)).decode().lower()
    finally:
        server.close()
        await server.wait_closed()

    cookies = [line for line in head.split("\r\n") if line.startswith("cookie:")]
    assert cookies == ["cookie: a=1; b=2"]
