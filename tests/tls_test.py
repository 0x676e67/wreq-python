import pytest
import wreq
from wreq.emulation import Emulation
from wreq.tls import CertStore, chromium_root_store


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_badssl():
    client = wreq.Client(tls_verify=False)
    resp = await client.get("https://self-signed.badssl.com/")
    async with resp:
        assert resp.status.is_success()


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_badssl_invalid_cert():
    url = "https://self-signed.badssl.com/"
    client = wreq.Client(tls_verify=False, tls_info=True)
    resp = await client.get(url)
    async with resp:
        assert resp.status.is_success()
        tls_info = resp.tls_info
        assert tls_info is not None

        peer_der_cert = tls_info.peer_certificate()
        assert peer_der_cert is not None

        assert isinstance(peer_der_cert, bytes)
        assert len(peer_der_cert) > 0

        cert_store = CertStore(der_certs=[peer_der_cert])
        assert cert_store is not None

        client = wreq.Client(tls_verify=cert_store)
        resp = await client.get(url)
        async with resp:
            assert resp.status.is_success()


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_alps_new_endpoint():
    url = "https://google.com"
    client = wreq.Client(emulation=Emulation.Chrome133)
    resp = await client.get(url)
    async with resp:
        text = await resp.text()
        assert text is not None


def test_chromium_root_store():
    # The binding is always registered; on a build without the chromium-pki feature
    # it raises rather than being absent, and on a feature build it returns a store.
    try:
        store = chromium_root_store()
    except RuntimeError as exc:
        assert "chromium-pki" in str(exc)
    else:
        assert isinstance(store, CertStore)
