#!/bin/bash

set -e

if [ -z "$1" ]; then
  echo "Usage: $0 <target> [maturin_args]"
  exit 1
fi

TARGET=$1
ARGS=$2

IMAGE="ghcr.io/0x676e67/rust-musl-cross"
VOLUME_MAPPING=("-v" "$(pwd):/home/rust/src")
MATURIN_CMD="maturin build --release --out dist $ARGS"
EXTRA_ENV=()

for var in BORING_BSSL_RUST_CPPLIB MATURIN_VERSION CFLAGS CXXFLAGS LDFLAGS RUSTFLAGS; do
  if [ -n "${!var}" ]; then
    EXTRA_ENV+=("-e" "$var=${!var}")
  fi
done

case $TARGET in
  x86_64-unknown-linux-musl | \
  aarch64-unknown-linux-musl | \
  armv7-unknown-linux-musleabihf | \
  i686-unknown-linux-musl)
    ;;
  *)
    echo "Unknown target: $TARGET"
    exit 1
    ;;
esac

echo "Building for $TARGET..."
docker pull $IMAGE:$TARGET
# The image's bundled Rust may be older than the project's MSRV.
docker run --rm "${VOLUME_MAPPING[@]}" "${EXTRA_ENV[@]}" $IMAGE:$TARGET /bin/bash -c \
  "rustup toolchain install 1.98.0 --profile minimal --target $TARGET && rustup run 1.98.0 $MATURIN_CMD"

echo "Build completed for target: $TARGET"
