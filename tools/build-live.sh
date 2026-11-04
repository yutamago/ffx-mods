#!/usr/bin/env bash
# Builds demaster_live.dll (i686-pc-windows-msvc: FFX.exe is 32-bit) on
# Linux/WSL with cargo-xwin, as tools/build-windows.sh builds demaster.exe.
# The live music engine (docs/music.md, "Phase 2"); the C# mod loads it
# from the mod folder when audio.music_engine = "live".
# Usage: tools/build-live.sh [extra cargo args]
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$repo/tools/shims:$PATH"
export XWIN_ARCH=x86
export RUSTC_WRAPPER=
target=i686-pc-windows-msvc
cache="${XDG_CACHE_HOME:-$HOME/.cache}"
mkdir -p "$cache"
eval "$(flock "$cache/ffx-xwin.lock" cargo xwin env --target "$target" 2>/dev/null)"
export RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static"
cd "$repo"
cargo build --release --target "$target" -p demaster-live "$@"
echo "built: $repo/target/$target/release/demaster_live.dll"
