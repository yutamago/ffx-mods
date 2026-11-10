# The interface items still open (ps2-interface)

The parts of the interface that still look like the HD version, and how to
pick them up later. Status 2026-11-09: none started. A first agent run
(2026-11-04..06, stopped by the usage limit) got only as far as two replay
edits that were never run (below, "Where it stopped").

Everything here goes into the **ps2-interface** mod (`mods/ps2-interface`,
settings `ui.*`; the converter's `ui.menus` job, `converter/src/jobs/interface.rs`,
`jobs/sphere_grid.rs`). Read first: `docs/ui.md` (each item's section, named below),
`docs/compare.md` (how screens are compared with the PS2 and the numbers so
far), `docs/debugger.md` (PCSX2 with the debug server), `docs/plan.md` (row
"PS2-drawn menu textures..." and its Todo).

## The items

| # | Item | Now | Goal | Docs |
|---|---|---|---|---|
| 1 | Sphere grid panels | PS2 textures are in; the panels around the grid are the PC's (HD title plate, face at (x - 95, 80), name at y 158, HD digits) | The PS2 panel (`0x00260378`, SLPM_675.13): two translucent brown-to-gold bands, the S.LV label and frame lines from page 14400 (anm sprite list `0x0033b050`), the 91 x 98 face at (x - 130, 90), the name at y 59, the level at (x - 22, 198) with the PS2 digit draw `0x00257818`; the menu panels' PS2 draws; the bright node links | ui.md "The sphere grid", "The interface around the grid" |
| 2 | Libra (Scan) window, enemy HP gauges | the PC's | First check whether the PS2 has them and draws them differently (PS2 battle state with Libra / a scanned enemy); port only what differs | none yet |
| 3 | Wakka's Overdrive reels | the PC's own drawing | the PS2's reels | ui.md "The Overdrive mini-game windows" |
| 4 | World map | ported (picture, location plates, marker scene), never seen on screen | see it in the game, then compare with the PS2 | ui.md "The world map"; compare.md (`ui-worldmap`) |
| 5 | Overdrive mini-games | Tidus's replay runs (bar, hit zone, timer at PS2 places; a gap after the button glyph in the prompt); Auron, Lulu, Wakka replays never run | run all four; compare Tidus's bar with the PS2 | ui.md "The Overdrive mini-game windows"; compare.md |

The sphere grid's camera distance is NOT an item any more: the "PS2 shows
the grid 1.6x closer" of the first comparison was the 16:9 grid squeezed
into 4:3, fixed by `render.aspect` (ps2-aspect-ratio,
`RenderPatches.cs` `ABMAP_ASPECT_LOAD` `0x00652a7a`).

## How each screen has been done so far

1. Find the PS2 draw (PCSX2 debugger, SLPM_675.13 / SCES_504.92 symbols in
   ffx-rust) and its FFX.exe twin (ffx-rust `symbols/ffx.sym`, the Ghidra
   dump `~/ffx-work/decomp`).
2. Hook the twin in the mod (`mods/ps2-interface`, helpers from the old
   `Ps2Menu.cs`) and draw what the PS2 drew, at PS2 places.
3. Mirror the draw in `tools/calllist` and write a harness replay
   (`tools/replays/ui-*.txt`) that reaches the screen.
4. Compare with the PS2 in PCSX2 (`tools/compare`, a PS2 save state at the
   same screen): the number before and after, remaining differences
   explained in `docs/compare.md`. Look at the PNGs yourself.
5. Test in the game through Fahrenheit (the mod installed), not only the
   harness.

## Where it stopped

Branch `wip/ui-last-items` in **ffx-demaster** (commit `cac6926`, pushed,
"untested, interrupted"):
- `tools/replays/ui-spheregrid.txt`: applies the 4:3 camera write as the
  mod does (the replay had lagged behind the mod).
- `tools/replays/ui-worldmap.txt`: a new way to open the world map: open
  the main menu (`call 0x00821750 0x17`, which registers menu 0x17 =
  `TkMenuCtrlWorldMap` but activates only the main menu), then
  `call 0x008aa100 0x17 0` (`TkMenuMainActiveMenu(0x17, 0)`, as the game's
  `TkMenuCtrlItem` does in its state 7), wait for the fade-in (`+0x64` =
  0x80). Never run.
Start from those two edits.

## Before starting

- **Where the test tools live:** the harness replays, `tools/shotrun`,
  `tools/calllist`, `tools/compare`, `tools/pcsx2ctl` stay in ffx-demaster,
  now the research and test-tool home of the PS2 mods (the bundle moved
  here as `mods/ps2-bundle` on 2026-11-09); `wip/ui-last-items` is a
  branch there.
- **PS2 states:** item 5 needs a PS2 battle with a full Overdrive gauge
  (e.g. set the gauge byte through PINE in the `battle-kami` state, and the
  same in the PC replay; the PS2 actor's gauge address is not looked up
  yet). Items 2 and 4 need a save near those screens.
- **Disk:** C: is nearly full. PS2 states and screenshots add up; delete
  them when the item is done.

## Rules for the run

- One game at a time; the lock `flock /home/angelo/ffx-work/demaster/fh-test.lock`
  around Fahrenheit runs, and wait until no other FFX.exe runs (other
  sessions start games without the lock). End only PIDs you started; never
  kill FFX or PCSX2 by name. One PCSX2 at a time. Capture only the game
  window.
- Bump `Job::version` of `ui.menus` when its output changes; settings in
  the places the settings test checks; new FFX.exe / PS2 facts in
  `docs/ffx-rust-findings.md`.
- Done per item = seen in the game and compared with the PS2 where a PS2
  state exists, with `docs/ui.md`, `docs/compare.md` and `docs/plan.md`
  updated.
