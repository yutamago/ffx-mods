# Findings to port into ffx-rust

New facts about FFX.exe, the PS2 ELF/IOP modules and the PS2 data found while
building the demaster. Add a line as you find something; the orchestrator
ports them into ffx-rust (`symbols/ffx.sym`, `docs/formats/*.md`) and marks
them ported.

| Status | What (address, name, format) | Evidence | Our doc |
|---|---|---|---|
| new | FFX.exe `0x008f9a40` `TOMkpWindowPlain`(x, y, w, h): the PS2 message window, meswin shapes 200-203 (layout `0x00c5b460` entries 0-3: (160,56) 16x24, (176,56) 8x24, (160,80) 16x8, (176,80) 8x8), colours `col_mssgwind0` `0x025d16b4` (corner, left edge, gradient start) and `col_mssgwind1` `0x025d16b0` (gradient end), horizontal gradient on shapes 201/203 | decompile; PS2 twin SLPM_675.13 `0x001fa550`, same structure; drawn in the game | `docs/ui.md` "The PS2 windows" |
| new | FFX.exe `0x00907060` `yiWindowDrawMenu` (the kind-0 draw) is the remaster's replacement. The PS2 kind-0 draw (SLPM_675.13 `0x00208758`, handler table `0x00323710`, 6 words a kind) is scissor + `TOMkpWindowPlain` + text scissor (x0+12, y0+4, x1-12, y1-4), i.e. FFX.exe's `yiWindowDrawPlate` `0x00907390`; PS2 kinds 1/2 (`0x00208870`) draw no plate | MIPS disassembly | `docs/ui.md` |
| new | `TOMkpShapeXYWHUV`/`UVC2` shape ids: -1 = `0x3e80` (16000), -2 = `0x3e82` (UV x 1/2048, 1/1024: `battle_kuang`), -3 = `0x3e83`, -4 = `0x3e84` `stonetexture` (UV / 256), -6 = `0x3e86`; 0..398 meswin, 400.. battle, 600.. icon | decompile `0x00903bb0`, `0x00903ee0` | `docs/ui.md` |
| new | `TODrawMenuPlateXYWHType` (`0x008f5f70`) type 10 = the HD message box: shadow `TODrawCrossBoxXYWHC2` (+6, +6, `0x401a1a1a`), stone body (shape -4, alpha 0x40), gradient `TODrawCrossBoxXYWHC2Mode2` `0x608f2c45`/`0x60501826` (sphere grid: `0x60340614`/`0x60300410`), meswin frame (shape 200 with explicit UVs); type 9 the same with `0x668f4b2d`/`0x6617080a`; callers yiWindowDrawMenu, TOMenuDrawYesNoWindow, TkBtlEndDrawLegendNotice | decompile | `docs/ui.md` |
| new | `TOSeqInitParam` colours at run time (PC, English; bytes R G B A): `col_mssgwind0/1` `19 00 31 6b` / `34 00 66 6b`; `col_complt0/1` `4f 28 78 80` / `58 4a 82 80`; `col_nameplt0/1` `24 3e 53 80` / `11 18 40 80`; `col_helpplt` `09 09 09 80`; `col_bsoplt0/1` `3a3a3a80`/`27272780`; `col_bitemplt0/1` `51422380`/`3c3f1280`; `col_babplt0/1` `4a1e5580`/`420d3980`; `col_lmtplt0/1` `1f385580`/`141c3880` | harness `read 0x025d1600 0xb8` | `docs/ui.md` table |
| new | SLPM_675.13 callers of the PS2 `TOMkpWindowPlain` `0x001fa550`: the 5 window-kind draws `0x00208758`, `0x00208a68`, `0x00208ce8`, `0x00208f60`, `0x002091e0`; the yes/no window `0x002362c0` (= PC `TOMenuDrawYesNoWindow`); unidentified `0x00223880`, `0x002238e0`, `0x002239b0` (the last two through a table at `0x00223cd8`), `0x0023d758`. `0x001fa0f0`, `0x001fa210`, `0x001fa448` read the same colours and have no reference | MIPS disassembly, data pointer search | `docs/ui.md` |
