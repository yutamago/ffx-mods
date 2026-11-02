# Findings to port into ffx-rust

New facts about FFX.exe, the PS2 ELF/IOP modules and the PS2 data found while
building the demaster. Add a line as you find something; the orchestrator
ports them into ffx-rust (`symbols/ffx.sym`, `docs/formats/*.md`) and marks
them ported.

| Status | What (address, name, format) | Evidence | Our doc |
|---|---|---|---|
| new | PS2 `musicNNN.bgm` header `u32` at 0x10 (size): the end of the last track or up to 3 bytes more (music052/127/190/191/193/194/196/240); the bytes after the last track are not always zero (music127 `c8ffc1`, 193 `88`, 196 `7f3e78`, 197 `01d4`); files padded to 8 | scan of all 112 BGMs of the German disc; studio round trip keeps both | `docs/studio.md` |
| new | PS2 BGM usage: header flag `0x10000` (automatic key-off one tick before the next command), counted loops `0x04/0x05`, ties `0x2a/0x2b`, `0x2e` only in `music240`; no sequence uses `0x70` (delta offset); every varlen is minimal; every other song ends its notes with `0x18`/`0x1a` | same scan (command counts per file) | `docs/studio.md` |
| new | PS2 `wave_wd.sec` lists no bank for music052, 190 (191 uses 190), 193-199: their sequences name WD banks that `wave_wd.dat` does not hold | `Sources::bank` on the German disc | `docs/studio.md` (Limits) |
