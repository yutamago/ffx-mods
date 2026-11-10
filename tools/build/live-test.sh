#!/usr/bin/env bash
# Plays a replay through ffx-rust's harness (tools/shotrun) with the live
# music engine injected into the game (docs/mods/music.md, "Phase 2"; no C#
# mod runs in the harness):
#   tools/build/live-test.sh <replay> <music dir> <Windows-side work dir (/mnt/c/...)> [mod dir...]
# <music dir> is a converter output's music/ (job audio.live). The DLL,
# live-inject.exe and the music files are copied to the work dir (Windows
# processes can't load them from WSL); FMOD's mix goes to <work>/mix.wav
# (FFX_AUDIO_WAV), the engine's own output to <work>/live.wav
# (DEMASTER_LIVE_WAV), its log to <work>/music/live.log.
# Needs tools/build/build-live.sh first; ends only the FFX.exe it started.
# LIVE_NO_INJECT=1 runs without the engine (a baseline).
set -euo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
replay="$(realpath "$1")"; music="$2"; work="$3"; shift 3
rel="$repo/target/i686-pc-windows-msvc/release"
mkdir -p "$work/music"
cp "$rel/demaster_live.dll" "$rel/live-inject.exe" "$work/"
cp "$music"/*.bgm "$music"/*.wd "$music"/*.irx "$work/music/"
rm -f "$work/music/live.log" "$work/mix.wav" "$work/mix.wav.marks" "$work/live.wav"
winwork="$(wslpath -w "$work")"
pids() { tasklist.exe /FI "IMAGENAME eq FFX.exe" /FO CSV /NH 2>/dev/null | tr -d '\r' | awk -F'","' '/FFX.exe/{print $2}' | sort; }
before="$(pids)"
export FFX_AUDIO_WAV="$work/mix.wav" DEMASTER_LIVE_WAV="$work/live.wav"
export WSLENV="${WSLENV:+$WSLENV:}FFX_AUDIO_WAV/p:DEMASTER_LIVE_WAV/p"
# ffx-rust next to the repo (FFX_RUST for a worktree), and a shotrun whose
# path dependency on it resolves (SHOTRUN: its Cargo.toml)
ffx_rust="$(realpath "${FFX_RUST:-$repo/../ffx-decomp}")"
shotrun="${SHOTRUN:-$repo/tools/shotrun/Cargo.toml}"
cd "$ffx_rust"
FFX_GAME_DIR="${FFX_GAME_DIR:-$HOME/steam-games/FFX-X2-HD}" cargo run --release -q \
  --manifest-path "$shotrun" -- "$replay" "$winwork\\trace.txt" "$@" &
run=$!
# Other agents start games too: the game is ours only if exactly one new
# FFX.exe appeared (checked again after the start-up wait). With two, we
# can't tell which is ours, so nothing is injected or killed.
newpids() { comm -13 <(echo "$before") <(pids); }
pid=""
for _ in $(seq 1 300); do
  pid="$(newpids | head -1)"
  [ -n "$pid" ] && break
  sleep 1
done
if [ -z "$pid" ]; then echo "no new FFX.exe"; wait $run; exit 1; fi
sleep 20
if [ "$(newpids | wc -l)" -ne 1 ]; then
  echo "several new FFX.exe ($(newpids | tr '\n' ' ')): can't tell which is ours; not touching any"
  wait $run; exit 1
fi
echo "game PID $pid"
# LIVE_NO_INJECT=1: a baseline run with the game's own FMOD music (the
# replay's waitfor on "LIVE" must go: tools/replays/music-live.txt with its
# first line removed)
[ -n "${LIVE_NO_INJECT:-}" ] || "$work/live-inject.exe" "$pid" "$winwork\\demaster_live.dll" "$winwork" || { taskkill.exe /PID "$pid" /F; exit 1; }
wait $run
