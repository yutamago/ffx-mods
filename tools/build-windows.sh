#!/usr/bin/env bash
# Builds demaster.exe (x86_64-pc-windows-msvc) on Linux/WSL with cargo-xwin,
# the way ffx-rust builds its Windows binaries (../ffx-decomp/tools/build-renderer.sh).
# Needs: rustup target add x86_64-pc-windows-msvc; cargo install cargo-xwin.
# Usage: tools/build-windows.sh [extra cargo args]
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
# lld-link -> rustc's rust-lld (Ubuntu's lld-link can be too old)
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86_64
export RUSTC_WRAPPER=
target=x86_64-pc-windows-msvc
cache="${XDG_CACHE_HOME:-$HOME/.cache}"
mkdir -p "$cache"
# Only setting up cargo-xwin's MSVC CRT cache races between parallel builds,
# so that runs under ffx-rust's lock; the build itself doesn't.
eval "$(flock "$cache/ffx-xwin.lock" cargo xwin env --target "$target" 2>/dev/null)"
# cargo-xwin sets RUSTFLAGS (empty), which overrides .cargo/config.toml's
# target rustflags: link the CRT statically here too, so demaster.exe needs
# no Visual C++ runtime. (With --target, RUSTFLAGS skips host build scripts.)
export RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static"
cd "$repo"
cargo build --release --target "$target" -p demaster "$@"
echo "built: $repo/target/$target/release/demaster.exe"
