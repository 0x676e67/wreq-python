import ssl

import pytest
import wreq
from wreq.emulation import Emulation
from wreq.tls import CertStore, ExtensionType, TlsOptions


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

        assert type(peer_der_cert) is bytes
        assert len(peer_der_cert) > 0

        cert_store = CertStore(der_certs=[bytes(peer_der_cert)])
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


def test_tls_options_trust_anchors_and_grease_sigalgs():
    opts = TlsOptions(grease_sigalgs_enabled=True, trust_anchors=b"")
    assert opts is not None


def test_extension_type_trust_anchors():
    opts = TlsOptions(extension_permutation=[ExtensionType.TRUST_ANCHORS])
    assert opts is not None


def test_cert_store_accepts_bytes_like():
    certs = ssl.create_default_context().get_ca_certs(binary_form=True)
    if not certs:
        pytest.skip("no CA certificates loaded")
    der = certs[0]
    pem = ssl.DER_cert_to_PEM_cert(der).encode()
    CertStore(der_certs=[wreq.Bytes(der), memoryview(der)])
    CertStore.from_der_certs([wreq.Bytes(der)])
    CertStore.from_pem_stack(wreq.Bytes(pem))


def test_trust_anchors_accept_bytes_like():
    # Any buffer is taken as raw bytes, not as a sequence of items.
    for ids in (wreq.Bytes(b"\x01\x02"), memoryview(b"\x01\x02\x03\x04").cast("I")):
        assert TlsOptions(trust_anchors=ids) is not None
