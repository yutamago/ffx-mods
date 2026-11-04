# PS2 vs Demaster: the comparison tooling

`tools/compare` puts the PS2 original, running in PCSX2, next to the
Demaster (the PC game with the mod, through the ffx-rust harness) for the
same scene, from the same saves, in the same language. For each scene it
writes two PNGs, PS2 | Demaster | difference, and numbers per screen region:

- **exact** (`<replay>-<shot>-exact.png`): PCSX2's software renderer at the
  native resolution (512x416) against the Demaster's 4:3 part scaled down to
  that size. For colours, blending and layout: the software renderer is the
  reference for what the GS drew.
- **detail** (`<replay>-<shot>-detail.png`): PCSX2's hardware renderer
  (Vulkan, 2x: 1024x832) scaled to the Demaster's 4:3 part (960x720)
  against the Demaster at full resolution. For textures, edges and small
  text. Both renderers draw the same frame: the PS2 side keeps a
  single-frame GS dump of each shot and replays it with the other renderer.

Output: `/mnt/c/shots/compare/` (`ps2/`, `ps2hw/`, `pc/` the raw frames,
`report.tsv` the numbers). Nothing of it goes into git.

## Pieces

| Piece | What |
|---|---|
| `tools/pcsx2ctl` (`tools/build-pcsx2ctl.sh`, Windows exe through cargo-xwin) | Drives a running PCSX2: PINE (TCP `127.0.0.1:28011`) for EE memory, save states and status; `WM_KEYDOWN`/`WM_KEYUP` posted to PCSX2's display window for the pad and the hotkeys (no focus needed, the user's keyboard is not touched); screenshots through PCSX2's screenshot hotkey (the GS output at the internal resolution); GS dumps through the single-frame dump hotkey; `wincap` captures the display window only (`PrintWindow`), never the desktop. Runs scripts like the replays (`wait`, `press`, `shot`, `gsdump`, `keep`, `state`, `waitfor`, `pressuntil`, `read`, `write`, `dump`). |
| `tools/compare/setup.py` | Writes the settings `PCSX2.ini` needs (pad bindings for the posted keys, F12 screenshot as PNG at the internal resolution, F11 GS dump, the folder memory card, software renderer at native) and the memory card from the PC saves (below). `setup.py renderer <dir> sw\|vk\|dx12 [scale]` switches the renderer. |
| `tools/compare/pcsx2.sh` | `start [state]` (boots the disc, or `PCSX2_ISO=<dump>` a GS dump; writes the PID to `compare.pid`), `stop` (`taskkill /PID`, never by name), `ctl <pcsx2ctl args>`. |
| `tools/compare/states.txt` | pcsx2ctl script that makes the save states from a fresh boot: `title`, `field-kami0100`, `field-kami-outside`, `battle-kami`, `field-cdsp0000` (in `C:\_workspaces\pcsx2\sstates\compare\`). |
| `tools/compare/scenes.toml` | One `[[screen]]` per PC replay: the PS2 state and script, which PS2 shot pairs with which PC shot, the regions, and `pc_env` (environment for the PC run: `FFX_HOOK_DISABLE = "replacements"` makes the harness run the game's own code instead of ffx-rust's ports, so a replay's code patches and jumps, standing in for the mod's hooks, take effect; the title and battle scenes need it). |
| `tools/compare/compare.py` | `ps2`, `hw`, `pc`, `diff`, `all` (below). |
| `tools/replays/compare-title.txt`, `compare-battle-kami.txt` | PC replays for the title screen and for the same battle as the PS2 state. |

## Running it

```bash
tools/build-pcsx2ctl.sh                                    # once
uv run python -I tools/compare/setup.py /mnt/c/_workspaces/pcsx2 \
  "/mnt/c/Users/<user>/Documents/Square Enix/FINAL FANTASY X&X-2 HD Remaster/FINAL FANTASY X" 1 5
tools/compare/pcsx2.sh start                               # once: the save states
tools/compare/pcsx2.sh ctl run '<Windows path of tools\compare\states.txt>'
tools/compare/pcsx2.sh stop
target/release/demaster ui /tmp/dm-ffx.iso ~/steam-games/FFX-X2-HD/data/FFX_Data.vbf \
  ~/ffx-work/demaster/scratch/compare-ui                   # the mod files the PC side uses
uv run --with pillow --with numpy python -I tools/compare/compare.py all [replay...]
```

`compare.py ps2` plays the PS2 scripts (one PCSX2, software renderer) and
keeps each shot and its GS dump; `hw` replays the dumps with the hardware
renderer (`--hw vk|dx12|dx11`, `--hw-scale N`, one PCSX2 launch per dump,
the ini back to software afterwards); `pc` runs the PC replays through
`tools/calllist` (`--mod`, `--lang de`); `diff` writes the PNGs and
`report.tsv`. Each step runs one game at a time and refuses to start the PC
game while a PCSX2 runs. A run of all twelve shots takes about 15 minutes,
most of it the PC side.

### Same saves on both sides

The PC save file is the PS2 one: SCES_504.92 writes 0x64f8 bytes (0x40
header + the 0x64b8-byte game state, CRC-16 at header `+0x1a` and as a word
at `+0x64f4`), and the PC's `ffx_NNN` files hold exactly that, zero-padded
to 0x6900, with the same CRC (PS2 `0x002510a8`: CCITT polynomial 0x1021,
start 0xffff, result inverted, table entry 0xff left 0; every PC save passes
it). `setup.py` copies the first 0x64f8 bytes of `ffx_001` and `ffx_005`
into a PCSX2 folder memory card (`memcards/ffx-compare.ps2/`, with a
formatted 8 MB superblock written by `setup.py`, which PCSX2 needs before it
lists the folders) as `BESCES-50492FF090601/BESCES-50492FF090601` and
`...05`. The game lists slot n as file n + 1 and loads them like its own
saves (no icon needed). The PS2 then has the same party, items, equipment,
location and play time as the PC replays' `load direct 1` / `5`.

PC language: German (`FFX_LANG=de`, the replays' `menu_us` atlas paths
turned into `menu_de`), so both sides show the same texts. The PC's German
wording differs from the PAL disc's in places (`Items`/`Item`,
`Abilitys`/`Ability`, `Hintergrundmusik`): that is the PC's localisation.

### Driving PCSX2: what was tried and what works

- **PINE** works from WSL too (mirrored networking), but pcsx2ctl does it
  on the Windows side. Save states through PINE take slot numbers only;
  `keep`/`state` copy named states through slot 10.
- **Input**: PCSX2 2.8 has no command-line input playback (input
  recordings `.p2m2` need the GUI's menu). Keys posted to the display
  window go through Qt to PCSX2's keyboard bindings without focus.
  **The first key after a start is lost**: `pcsx2.sh start` spends it on an
  unbound key.
- **Screenshots**: the screenshot hotkey with `ScreenshotSize = 2`
  (internal resolution, uncorrected) gives the PS2 frame exactly
  (512x416 for the menus, title and battle); with the hardware renderer at
  N x, 512N x 416N. Window capture (`wincap`) gives the scaled, filtered
  window picture; not used for comparisons.
- **GS dumps**: the single-frame dump hotkey writes `<name>.gs.zst` (and a
  PNG preview, removed); `pcsx2-qt.exe -batch -- <dump>` replays it with the
  ini's renderer and scale, and the screenshot hotkey works there.
- Quirks found: the title menu shows no cursor until a first press, which
  only brings the cursor; loading a save state re-inserts the memory card,
  so a state taken in the save list reports no card (states.txt goes
  through the title state instead); the field takes no menu input for a
  few seconds after a load (states wait 500 frames).
- PS2 state worth reading in a screen: the running menu sequence scenes
  (`read 0x0068f540 0x100`: 8 slots of 0x20, +0 state, +8 name) and the
  layout words (Config's `r_cnameplt` record at `0x00693f78`); the
  per-screen scene list in `docs/ui.md` came from such reads.
- PS2 `g_SaveData` is `0x003194d0` (PC `0x0112ca90`): `+0` the map, used by
  `waitfor` in states.txt. The battle's field, group and formation are at
  `0x00318c76` (PC `0x0112c256`, `docs/ffx-rust-findings.md`).

### Hardware renderer inaccuracies

The detail view's PS2 side is the hardware renderer, which is not
pixel-exact. Against the software render of the same dump (scaled back to
512x416): mean difference 1.7-4.1 on the menus (2-5 % of the pixels over
32), 3.7 on the title, 8.3 in battle (3D: edges and depth). Seen:

- 1-pixel seams at sprite edges: the dark shadow line under the item and
  equipment plates is drawn dashed, plate ends get a light edge line.
- Thin lines and plate ends a pixel off (half-pixel offset at 2x).
- PCSX2's fixes do not help here: "Half Pixel Offset: Normal" with "Round
  Sprite: Full" changes nothing, "Align to Native" makes it worse (8 % of
  the pixels off). The dumps are rendered without them.

So colours, blending and positions are read from the exact view only; the
detail view is for texture content, glyph shapes and edges.

## Scenes

| Replay (PC) | PS2 state | Shots | Save |
|---|---|---|---|
| `ui-mainmenu` | `field-cdsp0000` | main menu | 1, salvage ship |
| `ui-items` | `field-cdsp0000` | Items | 1 |
| `ui-abilities` | `field-cdsp0000` | Abilities, Tidus | 1 |
| `ui-overdrive` | `field-cdsp0000` | Overdrive, Tidus | 1 |
| `ui-status-ps2` | `field-cdsp0000` | Status, Tidus | 1 |
| `ui-config` | `field-cdsp0000` | Config | 1 |
| `ui-equip` | `field-kami0100` | Equip, Tidus; the weapon list | 5, Thunder Plains agency |
| `ui-customize` | `field-kami0100` | Customize: equipment list on the shield, its ability list | 5 |
| `compare-title` | `title` | title menu, cursor on Load | none |
| `compare-battle-kami` | `battle-kami` | first command window | 5, field 34 (kami00) group 0 formation 1 |

The loading screen is not in the list: the PS2 shows plain black while it
loads (from the save list to the field), and the harness's loads are too
quick to show the PC's; `ui.loading` makes the PC's black too
(`docs/ui.md`).

## Results (2026-11-03)

Numbers: mean absolute difference (0-255) of the exact view, whole frame.
"Text" is the 16x12 font (names, options, help lines, list entries).

**First run, and what it found** (common to every menu screen):

- **Text at half size.** Every 16x12 string was about half as wide and
  0.7 as high as the PS2's, from the same start point (help line
  "Ausrüstung wechseln.": PS2 x 34..199, 16 lines; Demaster x 34..114),
  at 16:9 and at 4:3 alike: the PC draws the font 0.78 times as big with
  square-pixel extents (`docs/ui.md`, "The 16x12 font's size"). Fixed in
  `Ps2Menu.ps2_sized`; now x 34..199, lines 13..27 (PS2 12..27).
- **The menu sequence bands were missing**: scenes the PS2's sub-screens
  start and the PC does not. Read in PCSX2 per screen and started by the
  mod (`docs/ui.md`, "The menu sequence scenes").
- **Config's rows** (PS2 from y 65, 42 apart; Demaster 55, 40): the PC
  reads the Japanese International's `menu_script.bin`, the PAL game its
  own disc's (`docs/ui.md`, "The menu layout data"). The `ui.menus` job
  writes the disc's.
- **List plates 16 pixels too long** (Equip, Customize, Status, Overdrive,
  Items): 0xdb was drawn 209 wide from x + 16; the PS2 ends it at x + 209
  (`0x001f4820`).
- Plates, plate shadows, circles, scroll bars, portraits, the gradient
  background and the help bar's shape sit within 1-2 pixels of the PS2's.

**Before and after** (exact view, whole frame; the PC side regenerated with
`demaster ui`, the replays starting the scenes as the mod does):

| Screen | Before | After | Still different | Intended |
|---|---|---|---|---|
| Main menu | 10.4 | 5.1 | text 1 line lower (the PC's y offset 0.78) and its glyph edges; option plates about 2 pixels right | play time a second on (the PS2 ran on after the load); `Items`/`Abilitys` (PC wording) |
| Items | 15.7 | 5.6 | the bar's entries a few pixels apart from the PS2's | |
| Abilities | 10.4 | 3.1 | glyph edges | |
| Overdrive | 11.2 | 4.1 | glyph edges | |
| Status | 14.2 | 8.2 | the face (HD portrait source, 1 pixel); glyph edges | the `!` prompt: PS2 pad ✕ vs the PC's key glyph "C" (the harness plays with the keyboard), so its text starts elsewhere |
| Config | 55.4 | 6.5 | glyph edges | first row: Sound (Stereo/Mono) on the PS2, the PC's soundtrack row (Hintergrundmusik: Remaster/Original) in its place, with its own help line |
| Equip | 6.7 | 5.4 | glyph edges; the portrait (1 pixel) | |
| Equip, weapon list | 10.5 | 7.6 | the rows about 1 line lower; the Weapon row's cursor (the PS2 hides it every other frame while the list is open) | |
| Customize | 11.3 | 5.5 | glyph edges | |
| Customize, ability list | 13.8 | 6.8 | glyph edges | |
| Title | 51.0 | 51.0 | not touched: the Demaster places the picture higher and stretches it (the logo's top about 60 lines higher, the bottom bar about 25); the PC's own water band where the PS2 shows the PlayOnline banner and the water; the PC's small cursor (since done: "Results (2026-11-04)") | the water band (`docs/ui.md`) |
| Battle | 27.3 | 26.9 | command window: text now PS2 size (command region 30.6 -> 26.4), but the "EKSTASE!!" tab and the "BEFEHL" label are missing; the HUD stays HD (help bar, CTB list, party list); the 3D camera frames the scene a little wider (since done: "Results (2026-11-04)") | |

"Glyph edges": the PS2 font is drawn from the 4x filtered PS2 pages
(`docs/font.md`), the PS2 sampled it 1:1; and the glyphs start 0.78 lines
lower (the PC's y offset for the European languages; the draw takes an
integer y). Each string leaves a thin outline in the difference.

**Open** (2026-11-03; the battle and title items are done since, below): the battle command window's "EKSTASE!!" tab and "BEFEHL" label
and the rest of the battle HUD; the title picture's placement; the PC's 16-line
line feed inside a 16x12 string (the PS2's is 20; no menu string seen
with one). All of it was measured through `tools/calllist`, which makes the
mod's calls in the harness; the mod itself was not run in Fahrenheit here.

## Results (2026-11-04): battle HUD and title

Exact view, whole frame and regions (mean absolute difference; the
detail view in brackets):

| Scene | Region | Before | After |
|---|---|---|---|
| Title | all | 51.0 (50.3) | 18.2 (17.6) |
| | banner | 69.8 | 32.6 |
| | logo | 39.1 | 3.4 |
| | menu | 30.3 | 5.1 |
| | bar | 27.8 | 4.4 |
| Battle | all | 26.9 (26.3) | 19.3 (19.3) |
| | help | 31.7 | 12.3 |
| | CTB | 47.6 | 25.2 |
| | command | 26.4 | 18.9 |
| | party | 42.0 | 21.3 |

What changed: the title picture at the PS2's place (the converter lays
the layer out for the PC's quad) and its cursor moved with the menu text
(the mod's `ui.title` hook, done here by call detours); the battle help
bar, CTB list and party list drawn the PS2 way (`@ps2hud`), the
Overdrive tab and the battle sequence scenes' footer, glow, lines and
row bands (`tab`, `footer`: their resting frame as fixed calls).
`docs/ui.md`, "Title screen and logos", "The battle sequence scenes",
"The battle HUD".

Still different:

- **Title, banner** (intended): the PC's own animated, translucent water
  band covers the top of the screen; under it the converter leaves the
  PS2 banner's rows black (the user's choice: the banner stays covered).
  The PS2 shows the banner and its water strip there.
- **Title, menu**: the cursor is the PC's small hand at the PS2 place (the
  PS2's hand-and-ball is drawn by the mod's cursor hook only for cursor
  kind 0; whether the title's cursor takes that path was not checked).
- **Battle, 3D** (most of the rest): the PC camera frames the scene a
  little wider, the models stand elsewhere; it shows through every
  region.
- **Battle, CTB**: the turn order differs (the PC puts Wakka third, the
  PS2 seventh, from the same save): the battle's state, not the draw.
- **Battle, text**: glyph edges as in the menus; the font M digits a
  little greyer than the PS2's.
- **Battle, Overdrive tab**: the PS2 shot catches the tab's blink at a
  lower alpha.

## Not done

- More scenes: sphere grid, dialogue windows, the battle sub-menus, shops,
  field maps and models (the PS2 states are cheap to add: walk there with a
  script and `keep`; the PC side needs a replay at the same place).
- The loading screen and the splash videos (not reachable in the harness).
- A Fahrenheit run of the same scenes (the comparison uses the calllist
  replays, which make the mod's calls in the harness).
