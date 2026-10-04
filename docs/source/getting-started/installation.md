# Installation

!!! info "Supported Platforms"
	- **Python 3.11+ is required**
	- Linux (glibc/musl): `x86_64`, `aarch64`, `armv7`, `i686`
	- macOS: `x86_64`, `aarch64`
	- Windows: `x86_64`, `i686`, `aarch64`
	- Android: `aarch64`, `x86_64`

---

## Install from PyPI

The easiest way to install wreq is via PyPI:

```bash
pip install wreq
```

Or with [uv](https://github.com/astral-sh/uv):

```bash
uv pip install wreq
```

---

## Build from Source

To build from source, first set up the BoringSSL build environment. See the [boringssl build guide](https://github.com/google/boringssl/blob/main/BUILDING.md) for details.

Example (Ubuntu/Debian):

```bash
sudo apt install -y build-essential cmake perl pkg-config libclang-dev musl-tools git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
pip install uv maturin

uv venv
source .venv/bin/activate

# Development install
maturin develop --uv

# Build wheel
maturin build --release

# Install from local wheel
pip install target/wheels/wreq-*.whl
```

---

## Next Steps

- See the [Guides](../guide/basic.md) for usage examples
- Browse the [API Reference](../api/wreq.md) for full documentation
