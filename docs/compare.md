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
| `tools/compare/scenes.toml` | One `[[screen]]` per PC replay: the PS2 state and script, which PS2 shot pairs with which PC shot, and the regions. |
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
"Text" below is always the PS2 menu font (font M) as the Demaster draws it.

**Common to every menu screen.**

- **Text is drawn at half size.** Every string of the PS2 font M (help
  bar, names, options, list entries, labels) is about half as wide and half
  as tall as the PS2's, from the same start point: the help line
  "Ausrüstung wechseln." spans x 34..199, 16 lines high, on the PS2 and x
  34..114, 7 lines, on the Demaster (exact view). The numbers (HP, MP,
  S.LV, counts, AP) and the shape-drawn labels match in size. Most of the
  difference numbers come from this. Seen through `tools/calllist`, which
  makes the mod's calls; to check in Fahrenheit.
- **The menu sequence bands are missing**: the dark blue bands the PS2's
  sub-screens slide in and leave standing (main menu: a vertical band at
  x 366..376 left of the options; Items, Abilities, Overdrive, Status,
  Customize: a horizontal band under the bar or party plate; Config: one
  under the help bar and one at the right edge). Known (`docs/ui.md`, "Not
  done": the PS2 starts menu scenes the PC never starts).
- Plates, plate shadows, circles, scroll bars, portraits, the gradient
  background and the help bar's shape sit within 1-2 pixels of the PS2's.
- List plates (Equip lists, Customize) end about 16 pixels further right
  than the PS2's.

**Per screen.**

| Screen | Exact | Matches | Differs | Intended |
|---|---|---|---|---|
| Main menu | 10.4 | party plates, HP/MP, S.LV, option plates, footer box, Gil | text half size; the vertical band; location name small | play time a second on (the PS2 ran on after the load); `Items`/`Abilitys` (PC wording) |
| Items | 15.7 | the list (rows, icons, counts, circles, scroll bar) to 2 pixels | the sub-command bar: PS2 spans nearly the full width with wide gaps, the Demaster's is about 480 of 640 wide with its entries closer (it sizes to the half-size text); the band under the bar | |
| Abilities | 10.4 | party plate, list row | text; band under the party plate | |
| Overdrive | 11.2 | party plate, Prüfen/Wählen bar, mode plates, list rows | text; band | |
| Status | 14.2 | face, Overdrive/Type plates, weapon/armour rows, the 8 stat rows, values | text; band; the `!` prompt's button | PS2 pad ✕ vs the PC's key glyph (the harness plays with the keyboard; the PC's glyph) |
| Config | 55.4 | the choice windows' x, the brushes, the cursor | **row layout**: PS2 rows from y 70 every 42 lines, the Demaster's from y 60 every 40 (the PC's layout words, `0x008cc050`); text; the bands | first row: Sound (Stereo/Mono) on the PS2, the PC's soundtrack row (Remaster/Original) in its place |
| Equip | 6.7 (list 10.5) | best match: everything but the text, to 1-2 pixels | text; list plates longer; the left brush stub of the Weapon/Armour rows | |
| Customize | 11.3 / 13.8 | lists, item cost box (46 -> 42), shield and ability plates | text; list plates longer; band | |
| Title | 51.0 | the menu, logo and bar pictures (the PS2 layers) | the Demaster places the picture higher and stretches it: the logo's top about 60 lines higher, the bottom bar about 25 (the PS2's 512x448 picture is mapped differently); the PC's own water band where the PS2 shows the PlayOnline banner and the water; the PC's small cursor | the water band (`docs/ui.md`) |
| Battle | 27.3 | the same battle (monsters, party); the command window's colours | the HUD stays HD: help bar, CTB list, party list (smaller, other places; `docs/ui.md` "Not done"); the command window smaller with half-size text, without the PS2's "EKSTASE!!" tab and "BEFEHL" label; the 3D camera frames the scene a little wider | |

## Not done

- More scenes: sphere grid, dialogue windows, the battle sub-menus, shops,
  field maps and models (the PS2 states are cheap to add: walk there with a
  script and `keep`; the PC side needs a replay at the same place).
- The loading screen and the splash videos (not reachable in the harness).
- A Fahrenheit run of the same scenes (the comparison uses the calllist
  replays, which make the mod's calls in the harness).
