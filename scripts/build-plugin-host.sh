#!/bin/sh
# Build legacy-plugin-host for 32-bit Windows (the host OMSI's 32-bit plugin DLLs run in) and
# put it into dist/ (copy it next to the game) as legacy-plugin-host32.exe. Needs the i686-pc-windows-gnu Rust
# target and MinGW (brew install mingw-w64; rustup target add i686-pc-windows-gnu); off
# Windows the game runs it through Wine (brew install --cask wine-stable).
set -e
cd "$(dirname "$0")/.."
CARGO_TARGET_I686_PC_WINDOWS_GNU_LINKER=i686-w64-mingw32-gcc \
RUSTFLAGS="-C panic=abort -C link-arg=-Wl,--enable-stdcall-fixup" \
  cargo build --release --target i686-pc-windows-gnu -p legacy-plugin --bin legacy-plugin-host
mkdir -p dist
cp target/i686-pc-windows-gnu/release/legacy-plugin-host.exe dist/legacy-plugin-host32.exe
echo "plugin host installed as dist/legacy-plugin-host32.exe"
