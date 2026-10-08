# Installation

wreq requires Python 3.11 or newer. Wheels are published for Linux (glibc and
musl), macOS, Windows and Android. Available architectures depend on the
platform and Python build; pip selects a compatible wheel when one is available.

## Install from PyPI

```bash
python -m pip install wreq
```

Or, inside a virtual environment, use uv:

```bash
uv pip install wreq
```

If no compatible wheel is available, installation needs the source-build tools
below. Use [Quick start](quickstart.md) to check the installed client.

## Build from source

Install Rust 1.98 or newer, a C/C++ compiler, CMake, Perl and libclang. Consult
the [BoringSSL build guide](https://github.com/google/boringssl/blob/main/BUILDING.md)
for additional platform requirements. The source build also needs network
access to download Cargo dependencies, including the Git dependencies in the
repository's lockfile.

For Ubuntu or Debian, start with:

```bash
sudo apt-get update
sudo apt-get install -y build-essential cmake perl pkg-config libclang-dev nasm git
```

After installing Rust and uv, clone the repository and create a virtual
environment:

```bash
git clone https://github.com/0x676e67/wreq-python.git
cd wreq-python
uv venv
source .venv/bin/activate
uv pip install maturin

# Build and install the current extension into this environment.
maturin develop --uv --release --locked
```

On Windows PowerShell, activate the environment with
`.venv\Scripts\Activate.ps1`. Rebuild the extension after changing Rust code.

To produce a distributable wheel instead:

```bash
maturin build --release --locked --out dist
uv pip install dist/wreq-*.whl
```

Use the wheel's exact filename when your shell does not expand `*`.

## Next steps

Follow [Quick start](quickstart.md), read the [guides](../guide/basic.md), or
browse the [API reference](../api/wreq.md).
