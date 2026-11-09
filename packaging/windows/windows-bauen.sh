#!/usr/bin/env bash
# Baut unter Linux die Windows-Fassung (x86_64, MSVC-ABI) und packt sie als ZIP.
# Der Setup-Installer entsteht anschließend unter Windows mit Inno Setup:  iscc packaging\windows\werkbank.iss
# Braucht: rustup target add x86_64-pc-windows-msvc, cargo install cargo-xwin, clang, zip.
set -euo pipefail
cd "$(dirname "$0")/../.."
LLD="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin"
LLVM=$(ls -d /usr/lib/llvm-*/bin 2>/dev/null | sort -V | tail -1)
export RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static"
export PATH="$LLD:$LLVM:$PATH" CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=rust-lld XWIN_ACCEPT_LICENSE=1
cargo xwin build --release --target x86_64-pc-windows-msvc -p werkbank-app
EXE=target/x86_64-pc-windows-msvc/release/werkbank-app.exe
V=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
OUT=target/windows
rm -rf "$OUT/Werkbank" && mkdir -p "$OUT/Werkbank"
cp "$EXE" assets/handbuch.md assets/lizenzen.txt LICENSE.de.md LICENSE.en.md "$OUT/Werkbank/"
(cd "$OUT" && rm -f "Werkbank-$V-windows-x64.zip" && zip -qr "Werkbank-$V-windows-x64.zip" Werkbank)
ls -lh "$OUT"/*.zip
