import asyncio
from wreq import Client, Response
from wreq.emulation import Emulation, Profile, Platform
from wreq.tls import TlsOptions, TlsVersion, AlpnProtocol
from wreq.http2 import Http2Options, PseudoId, PseudoOrder
from wreq.header import HeaderMap, OrigHeaderMap


async def print_response_info(resp: Response):
    """Helper function to print response details

    Args:
        resp: Response object from the request
    """
    async with resp:
        print("\n=== Response Information ===")
        print(f"Status Code: {resp.status}")
        print(f"Version: {resp.version}")
        print(f"Response URL: {resp.url}")
        print(f"Headers: {resp.headers}")
        print(f"Content-Length: {resp.content_length}")
        print(f"Remote Address: {resp.remote_addr}")
        print(f"Content: {await resp.text()}")
        print("========================\n")


async def request_firefox():
    """Test request using Firefox browser Emulation

    Demonstrates basic browser Emulation with custom header order
    """
    print("\n[Testing Firefox Emulation]")
    client = Client(
        emulation=Emulation.Firefox135,
    )
    resp = await client.get("https://tls.peet.ws/api/all")
    await print_response_info(resp)
    return client


async def request_chrome_android(client: Client):
    """Test request using Chrome on Android Emulation

    Demonstrates advanced Emulation with OS specification

    Args:
        client: Existing client instance to update
    """
    print("\n[Testing Chrome on Android Emulation]")
    resp = await client.get(
        "https://tls.peet.ws/api/all",
        emulation=Emulation(
            profile=Profile.Chrome134,
            platform=Platform.Android,
        ),
        # Disable client default headers
        default_headers=False,
    )
    await print_response_info(resp)


async def request_advanced_configuration():
    """Test request using advanced configuration similar to the Rust example

    Demonstrates:
    1. Custom TLS configuration with specific cipher suites and curves
    2. Custom HTTP/2 configuration with specific settings
    3. Custom headers with Twitter Android user agent
    4. Original header order preservation
    """
    print("\n[Testing Advanced Configuration]")

    # TLS options configuration - similar to Rust example
    tls_options = TlsOptions(
        grease_enabled=True,
        enable_ocsp_stapling=True,
        curves_list=":".join(["X25519", "P-256", "P-384"]),
        cipher_list=":".join(
            [
                "TLS_AES_128_GCM_SHA256",
                "TLS_AES_256_GCM_SHA384",
                "TLS_CHACHA20_POLY1305_SHA256",
                "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
                "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384",
                "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256",
                "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256",
                "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384",
                "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256",
            ]
        ),
        sigalgs_list=":".join(
            [
                "ecdsa_secp256r1_sha256",
                "rsa_pss_rsae_sha256",
                "rsa_pkcs1_sha256",
                "ecdsa_secp384r1_sha384",
                "rsa_pss_rsae_sha384",
                "rsa_pkcs1_sha384",
                "rsa_pss_rsae_sha512",
                "rsa_pkcs1_sha512",
                "rsa_pkcs1_sha1",
            ]
        ),
        alpn_protocols=[AlpnProtocol.HTTP2, AlpnProtocol.HTTP1],
        min_tls_version=TlsVersion.TLS_1_2,
        max_tls_version=TlsVersion.TLS_1_3,
    )

    # HTTP/2 options configuration
    http2_options = Http2Options(
        initial_stream_id=3,
        initial_window_size=16777216,
        initial_connection_window_size=16711681 + 65535,
        headers_pseudo_order=PseudoOrder(
            PseudoId.METHOD,
            PseudoId.PATH,
            PseudoId.AUTHORITY,
            PseudoId.SCHEME,
            PseudoId.PROTOCOL,
        ),
    )

    # Default headers
    headers = HeaderMap()
    headers.insert(
        "User-Agent",
        "TwitterAndroid/10.89.0-release.0 (310890000-r-0) G011A/9 (google;G011A;google;G011A;0;;1;2016)",
    )
    headers.insert("Accept-Language", "en-US")
    headers.insert("Accept-Encoding", "br, gzip, deflate")
    headers.insert("Accept", "application/json")
    headers.insert("Cache-Control", "no-store")
    headers.insert("Cookie", "ct0=YOUR_CT0_VALUE;")

    # Original headers to preserve case and order
    orig_headers = OrigHeaderMap()
    orig_headers.insert("Cookie")
    orig_headers.insert("Content-Length")
    orig_headers.insert("User-Agent")
    orig_headers.insert("ACCEPT-LANGUAGE")
    orig_headers.insert("ACCEPT-ENCODING")

    # Create client with advanced configuration
    client = Client(
        tls_options=tls_options,
        http2_options=http2_options,
        headers=headers,
        orig_headers=orig_headers,
    )

    # Make request to TLS fingerprinting service
    resp = await client.post("https://pingly.us.kg/api/all")
    await print_response_info(resp)

    return client


async def main():
    """Main function to run the Emulation examples

    Demonstrates different browser Emulation scenarios:
    1. Firefox with custom header order
    2. Chrome on Android with OS specification
    3. Advanced configuration with custom TLS, HTTP/2, and headers
    """
    # First test with Firefox
    client = await request_firefox()

    # Then update and test with Chrome on Android
    await request_chrome_android(client)

    # Test with advanced configuration
    await request_advanced_configuration()


if __name__ == "__main__":
    # Run the async main function
    asyncio.run(main())
