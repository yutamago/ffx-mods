# PS2 debugger: PCSX2 + DebugServer + MCP

A PCSX2 build with a debug server, so Claude Code can set breakpoints and
watchpoints, read registers and memory, disassemble, step and backtrace in the
running PS2 game, through the `pcsx2` MCP server.

```
Claude Code ──stdio (MCP)──> pcsx2-mcp (node, WSL) ──TCP 127.0.0.1:21512──> DebugServer in pcsx2-qt.exe
                                                  └──TCP 127.0.0.1:28011──> PINE (save states, game info)
```

WSL runs with mirrored networking (`.wslconfig` `networkingMode=Mirrored`), so
the bridge in WSL reaches Windows' 127.0.0.1 directly.

## Paths and ports

| What | Where |
|---|---|
| Debug PCSX2 (portable, Windows) | `C:\_workspaces\pcsx2-debug` (`pcsx2-qt.exe`, `portable.txt`, `inis\PCSX2.ini`) |
| Its BIOS, memory cards, save states, disc | shared with the stock install: `C:\_workspaces\pcsx2\{bios,memcards,sstates}`, `C:\_workspaces\pcsx2\iso\ffx-de.iso` |
| Stock PCSX2 2.8.2 (comparison tooling, untouched) | `C:\_workspaces\pcsx2` (`docs/compare.md`) |
| MCP bridge (WSL) | `~/tools/pcsx2-mcp` (`node dist/index.js`; built, dev dependencies pruned) |
| Patch source and diff (GPL) | `C:\_workspaces\pcsx2-debug\source` |
| DebugServer | TCP `127.0.0.1:21512`, loopback only, newline-delimited JSON |
| PINE | TCP `127.0.0.1:28011` (slot 28011, as the stock ini) |
| Log | `C:\_workspaces\pcsx2-debug\logs\emulog.txt`: `[DebugServer] Listening on 127.0.0.1:21512` at start, `Client connected` per bridge |

The ini is the stock `PCSX2.ini` (pad bindings, PINE on) with the folders
pointed at the stock install. The build has no `updater.exe` (the official
updater would replace it with a stock build).

## Build

Upstream PCSX2 **v2.8.2** (`fd9d310c`, same as the stock install), Release,
MSVC 14.51 + CMake/Ninja, dependencies from v2.8.2's
`build-dependencies.bat` (Qt 6.11.1; FFmpeg DLLs copied from the stock 2.8.2,
they are only used for video capture), plus `DebugServer.cpp/.h` from
[hkmodd/PCSX2-MCP](https://github.com/hkmodd/PCSX2-MCP) (`8f9bf45e`) ported to
2.8.2:

- `MipsStackWalk::Walk` without the thread stack-top argument (2.8.2 has none),
  `Read8/Read32/Write8` (2.8.2's `MemoryInterface`), `NOMINMAX` before winsock.
- Breakpoint/watchpoint changes, step, resume and memory/register writes run on
  the CPU thread (`Host::RunOnCPUThread`, as PCSX2's own debugger does: adding
  a breakpoint resets the recompilers). Commands other than `status` answer
  "No game running" without a VM.
- Started in `VMManager::Internal::CPUThreadInitialize` next to PINE, stopped
  (thread joined) in `CPUThreadShutdown`; logs through PCSX2's `Console` (so
  the line is in emulog), `SO_EXCLUSIVEADDRUSE`.

The build tree was deleted after the build; `source\` holds the patch
(`pcsx2-v2.8.2-debugserver.patch`) if it ever has to be redone.

## Use

Start the game in the debug build (one PCSX2 at a time: both are
`pcsx2-qt.exe`, and `tools/compare` looks for that name):

```bash
PCSX2=/mnt/c/_workspaces/pcsx2-debug tools/compare/pcsx2.sh start [state]   # prints the PID
PCSX2=/mnt/c/_workspaces/pcsx2-debug tools/compare/pcsx2.sh stop            # taskkill /PID, never by name
```

The `pcsx2` MCP server is registered for this project in local scope
(`claude mcp add --scope local pcsx2 -- node /home/angelo/tools/pcsx2-mcp/dist/index.js`,
in `~/.claude.json`, not in git); a Claude session started after that has
the tools. Call `pcsx2_connect` first.

| Tools | What |
|---|---|
| `pcsx2_connect`, `pcsx2_status`, `pcsx2_game_info` | connect (DebugServer, then PINE), PC / paused / cycles, title and serial |
| `pcsx2_read_memory`, `pcsx2_write_memory`, `pcsx2_read_string`, `pcsx2_find_pattern`, `pcsx2_memory_diff` | EE memory (addresses as hex strings, e.g. `"0x003194d0"`; up to 4096 bytes per read) |
| `pcsx2_read_registers`, `pcsx2_write_register`, `pcsx2_evaluate` | all register categories (GPR 128-bit, CP0, FPR, VU0...), expressions (`sp + 0x10`, symbols) |
| `pcsx2_disassemble` | PCSX2's disassembler |
| `pcsx2_set_breakpoint` / `remove` / `list`, `pcsx2_set_watchpoint` / `remove` / `list`, `pcsx2_clear_all_breakpoints` | breakpoints with optional condition; watchpoints `read`/`write`/`readwrite`/`onchange`, action `break`/`log`/`both`, range `address`..`end` |
| `pcsx2_pause`, `pcsx2_continue`, `pcsx2_step`, `pcsx2_step_over` | execution |
| `pcsx2_get_backtrace`, `pcsx2_get_threads`, `pcsx2_get_modules` | stack walk of the running thread, EE/IOP BIOS threads, IOP modules |
| `pcsx2_save_state`, `pcsx2_load_state` | slots 0-9 through PINE (shared `sstates`; `tools/compare` uses slot 10 and `sstates\compare\`) |

`ee` is the default CPU; most tools take `cpu: "iop"`. A breakpoint or
watchpoint hit pauses the VM: poll `pcsx2_status` until `Paused: true`, then
read registers / backtrace, `pcsx2_continue` to go on.

Notes from the first session (FFX at the start of the game):

- `pcsx2_pause` usually lands in the BIOS idle thread (PC `0x00081fc0`, all
  registers zero, no backtrace): the game threads are waiting. Break in game
  code instead; all 8 game threads wait in `WaitSema` (stub `0x002f8460`,
  return `0x002f8468`), so a breakpoint on `0x002f8468` stops in a game thread
  with a 2-frame backtrace (caller `0x002d2068`).
- `pcsx2_list_watchpoints` shows 0 hits for a `break` watchpoint that did
  break; the pause and the PC are the evidence.
- The backtrace is PCSX2's heuristic stack walk: short, from the running
  thread only.

Example session (end to end, 2026-11-04): read `g_SaveData` `0x003194d0`;
breakpoint on `0x002f8468` hit, backtrace `0x002f8460 <- 0x002d20b0`;
write watchpoint on `0x00787aa0..0x00787ac0` (the stack below that
thread's `sp`) broke at `0x0011ada8` `sd ra,(sp)`; step; save and load
slot 9.
