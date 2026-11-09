# FFX mods

All of Yutamago's [Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit)
mods for FINAL FANTASY X HD Remaster (Steam, Windows). Most bring back parts
of the original PS2 game, with the PC version's game logic underneath (the
PS2 mods); others change the game in their own way. Install any of them,
alone or together; each is a separate zip.

| Mod | What it changes | PS2 disc | Installed size | Needs |
|---|---|---|---|---|
| [PS2 music engine](#ps2-music-engine) (`yuta.ps2-music-engine`) | the original soundtrack played by the PS2's own sound driver inside the game | yes | ~60 MB | — |
| [PS2 soundtrack](#ps2-soundtrack) (`yuta.ps2-soundtrack`) | the original soundtrack rendered from the PS2 music by the PS2 sound driver; new games start with it | yes | ~890 MB | — |
| [PS2 movies](#ps2-movies) (`yuta.ps2-movies`) | the PS2 movies (FMVs), 4:3, with the PS2 sound | yes | ~1.6 GB | PS2 aspect ratio recommended |
| [PS2 characters](#ps2-characters) (`yuta.ps2-characters`) | PS2 models and textures for the party, NPCs, monsters, aeons, weapons, objects | yes | ~860 MB | — |
| [PS2 maps](#ps2-maps) (`yuta.ps2-maps`) | PS2 textures of the field and battle maps, the PS2 painted backgrounds | yes | ~2.3 GB | PS2 aspect ratio recommended |
| [PS2 effects](#ps2-effects) (`yuta.ps2-effects`) | PS2 textures of spells, summons and the other effects | yes | ~1.3 GB | — |
| [PS2 lighting](#ps2-lighting) (`yuta.ps2-lighting`) | characters lit the PS2 way | optional | ~80 MB | — |
| [No HD post-effects](#no-hd-post-effects) (`yuta.no-hd-post-effects`) | no HD glow, no shadow maps (the PS2's ground shadows) | no | <1 MB | — |
| [PS2 interface](#ps2-interface) (`yuta.ps2-interface`) | PS2 menus, windows, font, portraits, title screen, button glyphs; a clean start-up | yes | ~180 MB | PS2 aspect ratio recommended |
| [PS2 aspect ratio](#ps2-aspect-ratio) (`yuta.ps2-aspect-ratio`) | the 4:3 picture with the PS2 camera | no | <1 MB | — |
| [PS2 game feel](#ps2-game-feel) (`yuta.ps2-game-feel`) | PS2 run speed, battle menu without input delay, battle subtitles | no | <1 MB | — |
| [Bug fixes](#bug-fixes) (`yuta.bug-fixes`) | NPC mouths, the Farplane video, Lulu's Fury spins | no | <1 MB | — |
| [Modern Minimap](#modern-minimap) (`yuta.modern-minimap`) | a round field minimap, centred on you and turned with the camera | no | <1 MB | — |
| [FPS unlock](#fps-unlock) (`yuta.fps-unlock`) | the game drawn at your monitor's refresh rate (or uncapped) at its own speed; slow motion and fast-forward | no | <1 MB | — |
| [Debug Revival](#debug-revival) (`yuta.debug-revival`) | the developers' debug tools left in the game, back on: script debugger, debug menus and panels, battle cheats, free camera, map jumps, developer saves (F9) | no | ~35 MB (two rebuilt rooms, made in the game) | — |
| **[PS2 bundle](#ps2-bundle)** (`yuta.ps2-bundle`) | all twelve PS2 mods above (not Modern Minimap, FPS unlock, Debug Revival) in one zip with one install step | yes | ~7.3 GB | — |

Every mod needs the shared library `yuta.ffx-mods` (in every zip; its
manifest dependency). Sizes are the files `install.cmd` builds with the
default settings, measured with the German PAL disc; the program files add
5 MB for mods with PS2 files, under 0.1 MB for the others.

**No game data is included.** The mods with PS2 files build them on your
computer from your own Final Fantasy X PS2 disc (any region): an image of
it (`.iso`), or the disc itself in a disc drive. After that they need
neither.

The [PS2 bundle](#ps2-bundle) (formerly FFX Demaster) is every PS2 mod
here in one zip, installed and built in one step (one pass over the disc).

## Install

You need:
- FINAL FANTASY X/X-2 HD Remaster on Steam (Windows);
- [Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit/releases)
  1.0.0-alpha12 or later, installed as in its
  [installation guide](https://github.com/fahrenheit-crew/fahrenheit/wiki/Installation-guide);
- for the mods with PS2 files: your own Final Fantasy X PS2 disc, as an
  image or in a disc drive.

For each mod you want:

1. Download `yuta.<mod>-<version>.zip` from the
   [releases](https://github.com/yutamago/ffx-mods/releases) and unpack it
   into your Fahrenheit folder (the one with Fahrenheit's `bin` folder). It
   adds `mods\yuta.<mod>\` and `mods\yuta.ffx-mods\` (the shared library;
   unpacking it again with another mod is fine).
2. Double-click `mods\yuta.<mod>\install.cmd`. It adds `yuta.ffx-mods` and
   the mod to `mods\loadorder` and makes the mod's `settings.toml` from
   `settings.default.toml`. For a mod with PS2 files it then asks where
   your disc is (the disc drives with the disc in them are listed; or pick
   an image file), finds the game in the Steam folders (or asks for its
   folder), and builds the mod's PS2 files into its folder. From a
   terminal: `install.cmd -Iso 'D:\PS2\FFX.iso'`, `install.cmd -Drive E`,
   `-Game <game folder>`, `-NoDisc` (PS2 lighting only), `-Force`
   (rebuild everything), `-Disable <mod>` / `-Enable <mod>` (below).
   Running it again is safe: it redoes only what
   changed. (The PS2 bundle has one `install.cmd` for all its mods, in
   `mods\yuta.ps2-bundle\`: it takes the same options and builds every
   installed mod from one pass over the disc.)
3. Start the game through Fahrenheit (`bin\fhstage0.exe <path to FFX.exe>`).

In the game, **F8** opens one window for every installed mod of the
family: for the mods with PS2 files whether their files match their
settings (and the command to rebuild them), and which code patches are on.
It opens by itself when something is wrong.

**Settings:** each mod has `settings.toml` in its folder, with a comment
above each setting; the defaults are the PS2's ([`docs/config.md`](docs/config.md)
describes every setting). `install.cmd` makes it from
`settings.default.toml` (the defaults, in the zip) when there is none; the
mod uses the defaults until then. After changing one, run that mod's
`install.cmd` again if it has PS2 files, then restart the game.

**Updating:** unpack the newer zip over your install and run
`install.cmd` again. The zips hold only `settings.default.toml`, never
`settings.toml`, so your settings stay; settings that are new in that
version are added to your `settings.toml` with their comments and
defaults (a setting the file lacks has its default anyway). The build
redoes only what changed.

**Switching a mod off:** `install.cmd -Disable <mod>` (in any mod's
folder; `-Disable ps2-movies,ps2-maps` for several) takes the mod out of
`mods\loadorder` and lists it in `mods\ffx-mods-off.txt`; `install.cmd`
then never adds it back or builds it, until `install.cmd -Enable <mod>`
(or deleting its line there and running `install.cmd`). Deleting a mod's
line in `mods\loadorder` by hand works too: the next `install.cmd` puts the
mod on that list instead of adding it back. The mod's folder and its
PS2 files stay, so switching it on again rebuilds only what changed
meanwhile. (Fahrenheit reads `mods\loadorder` line by line: a commented
line there would be taken for a mod name, hence the separate list.)

**Uninstall:** delete `mods\yuta.<mod>\` and its line in `mods\loadorder`
(the PS2 files it built are in its folder and go with it). When no mod of
the family is left, delete `mods\yuta.ffx-mods\` and its line too.

## PS2 bundle

The PS2 bundle (`yuta.ps2-bundle`, formerly
[FFX Demaster](https://github.com/yutamago/ffx-demaster)) is the HD
Remaster with the look and sound of the PS2 game: the twelve PS2 mods in
one zip, `yuta.ps2-bundle-<version>.zip`, installed and built in one step.

| In the bundle | |
|---|---|
| PS2 files | [PS2 music engine](#ps2-music-engine), [PS2 soundtrack](#ps2-soundtrack), [PS2 movies](#ps2-movies), [PS2 characters](#ps2-characters), [PS2 maps](#ps2-maps), [PS2 effects](#ps2-effects), [PS2 lighting](#ps2-lighting), [PS2 interface](#ps2-interface) |
| code only | [No HD post-effects](#no-hd-post-effects), [PS2 aspect ratio](#ps2-aspect-ratio), [PS2 game feel](#ps2-game-feel), [Bug fixes](#bug-fixes) |

- PS2 disc: needed. Size: ~7.3 GB of PS2 files with the default settings
  (German PAL disc), plus ffmpeg for the movies (168 MB, downloaded if none
  is found); the zip is 2.6 MB. The first build takes about an hour, half
  of it the movies.
- The zip holds each mod's folder as in its own zip (without its own
  `demaster.exe`), the shared library `mods\yuta.ffx-mods\` with the one
  `demaster.exe`, and `mods\yuta.ps2-bundle\`: the bundle's README, its
  installer and a small mod whose manifest lists the twelve (its
  `Dependencies`); in the game its section of the F8 window shows which of
  them are on.

**Install:** unpack the zip into your Fahrenheit folder and double-click
`mods\yuta.ps2-bundle\install.cmd`. It adds `yuta.ffx-mods`, every mod and
then `yuta.ps2-bundle` to `mods\loadorder`, asks where your disc is, finds
the game, and builds every mod's PS2 files from one pass over the disc,
each into its own folder (options as for a mod's `install.cmd`, above).
Start the game through Fahrenheit. Each mod keeps its own
`settings.toml`; after changing one, run the bundle's `install.cmd` again
(it rebuilds only what changed). To update, unpack the newer zip over the
install and run the bundle's `install.cmd`: your `settings.toml` files
stay (see Updating, above).

**Switching one mod off:** `mods\yuta.ps2-bundle\install.cmd -Disable <mod>`
(`-Enable <mod>` switches it on again); it stays off at every later
`install.cmd` (Switching a mod off, above). F8's "PS2 bundle" section
shows it as "off: switched off". Deleting its folder `mods\yuta.<mod>\`
as well gives its disk space back. A part of a mod is a setting in its
`settings.toml` (`"hd"`, or `false` for a fix). The other mods (Modern
Minimap, FPS unlock) install next to the bundle from their own zips.

## The mods

### PS2 music engine

The original soundtrack played the PS2 way: the PS2 game's own sound driver
(a port of its `iopsound.irx` on a model of the PS2's SPU2 sound chip) runs
inside the game and plays the music from the disc's sequences and
instruments, driven by the game's music commands (fades, pauses, tempo, as
the scripts ask). It plays while the game's soundtrack option (Config >
Soundtrack) is "Original"; choose that in the game. The three tracks with
streamed parts, and anything the engine can't play, stay the PC's
recordings of the original soundtrack.

- PS2 disc: needed (`install.cmd` copies the disc's music sequences,
  instrument banks and driver tables into `music\`).
- Size: ~57 MB of disc files, plus `demaster_live.dll` (0.5 MB).
- Setting: `audio.music_engine = "live"` (default here) or `"hd"`. Its log:
  `music\live.log`.

### PS2 soundtrack

The original soundtrack as the PS2 played it: every track rendered again
from the disc's music sequences and instruments by a port of the PS2
game's sound driver on a model of the SPU2, in place of the PC's
recordings in the game's "Original" soundtrack (`audio.music`). New games
start with the soundtrack option on "Original" (`audio.soundtrack`; the PC
starts on the arranged one); loaded saves keep their choice. With PS2
music engine too, that engine plays live and this bank is what it falls
back on (the three tracks with streamed parts).

- PS2 disc: needed. Size: ~890 MB (one sound bank). The build takes about
  five minutes.

### PS2 movies

The PS2 movies (FMVs) with the PS2's sound, in place of the HD's re-made
ones (`movies`). They are built for the picture the game shows: 4:3 (960 x
720) when PS2 aspect ratio is installed with its 4:3 picture, else 16:9
(1280 x 720) with the 4:3 picture between black bars. The converter reads
that from PS2 aspect ratio's settings; installing, removing or switching
it later shows PS2 movies as "rebuild needed" in the F8 window. The four
movies the PC plays with the 3D scene drawn over them (the Farplane among
them) stay HD.

- PS2 disc: needed. Needs ffmpeg to encode (not included): if none is
  found, `install.cmd` downloads an LGPL build of FFmpeg (168 MB) into the
  mod's `tools\` and checks its SHA-256 ([`docs/movies.md`](docs/movies.md)).
- Size: ~1.6 GB. The first build takes 30 to 35 minutes.

### PS2 characters

The PS2 models and textures: the party, NPCs, monsters and bosses, aeons,
weapons and objects. Models the HD kept from the PS2 get their PS2
textures back; the remodelled ones (the party, many NPCs) get the PS2
meshes and textures. Each kind is a setting (`models.*`). It also keeps
PS2 textures at full size at every texture quality of the game's graphics
options (`render.texture_quality`; the game's lower qualities would shrink
them), for its own files and those of every other mod of the family, and
can show unfiltered texels (`render.texture_filter = "nearest"`, off by
default).

- PS2 disc: needed. Size: ~860 MB (5,200 files). The build takes a few
  minutes.

### PS2 maps

The PS2 textures of the field maps (`maps.field`) and the battle maps
(`maps.battle`), cut out of the PS2's graphics memory as the PS2 game
loaded them, and the PS2's painted backgrounds of the fixed-camera rooms
(`maps.backgrounds`). The rooms the HD re-framed for 16:9 get their (4:3)
PS2 paintings only with PS2 aspect ratio's 4:3 picture; like PS2 movies,
the mod is rebuilt when that changes ([`docs/maps.md`](docs/maps.md),
[`docs/screen.md`](docs/screen.md)).

- PS2 disc: needed. Size: ~2.3 GB. The build takes about 15 minutes.

### PS2 effects

The PS2 textures of the effects: spells, summons, overdrives and the
particles of the maps and monsters (`effects.textures`,
[`docs/effects.md`](docs/effects.md)).

- PS2 disc: needed. Size: ~1.3 GB. The build takes a few minutes.

### PS2 lighting

Characters lit the PS2 way: the PS2's light colours and directions at its
level, its hard falloff instead of the HD's soft one, no HD fill light,
point lights only where the PS2 used them. The character shaders are
patched copies of the game's own, so that part needs no disc. With the
disc, the six cutscenes whose lights the HD retuned (the opening's Tidus
close-up among them) also get the PS2's light colours back; without it
they stay HD, and `install.cmd` (with "no disc" or `-NoDisc`) says so.

- PS2 disc: optional. Size: ~80 MB (the event lights: 0.5 MB of that).

### No HD post-effects

Two things the HD draws that the PS2 didn't: its glow, the blurred bright
parts added over the whole picture (`render.glow`), and its shadow maps
(`render.shadows`): without them every character has one soft ground
shadow under it, as on the PS2 ([`docs/render.md`](docs/render.md)).

- PS2 disc: not needed. Code only.

### PS2 interface

The PS2 interface: message and dialogue windows, battle command windows
and the battle HUD, the main menu, the Items and Abilities screens, the
hand cursor, the item icons, the PS2 font and its letter widths (English,
French, Italian, German, Spanish), character portraits, the PS2 title
screen, PlayStation button glyphs. A clean start-up too: no splash videos,
no loading screen, a black window at start instead of a white one.
Optional, off by default: the PS2's "SQUARESOFT presents" screen at boot
(`ui.boot_logo`). The PC-only screens (Esc menu, settings, save/load) and
the menus not done yet stay HD.

The PS2 laid all this out for a 4:3 screen. Alone, the mod draws it the
way the HD draws its own PS2-placed menus, spread over the 16:9 width;
with PS2 aspect ratio it lands where the PS2 had it. So PS2 aspect ratio
is recommended, not required (not a manifest dependency).

- PS2 disc: needed. Size: ~180 MB.

### PS2 aspect ratio

The PS2's 4:3 picture with black bars at the sides and the PS2 camera
projection (the HD widened the view for 16:9). The menus, windows and
battle HUD the game places in PS2 coordinates land where the PS2 had them;
the opening's staff credits get the PS2 size; the Flash screens (Esc menu,
settings, save/load) are drawn inside the 4:3 picture. The HD movies are
16:9: while one plays full screen the picture goes back to 16:9 for it
(the movies of PS2 movies are 4:3 and stay so).

- PS2 disc: not needed. Code only.

### PS2 game feel

How the PS2 version plays where the PC changed it, each a setting: Tidus
runs at the PS2's speed (`timing.run_speed`; the PC's is 12 % faster), the
battle menu takes input at once (`ui.battle_menu_delay`; the PC ignores the
pad for 0.3 to 0.45 s after each battle window opens), scripted battle
dialogue is always subtitled (`ui.battle_subtitles`).

- PS2 disc: not needed. Code only.

### Bug fixes

Bugs of the PC version, each a setting: NPC mouths that stay open after
they talk (`fixes.npc_mouths`), the Farplane background video that never
plays (`fixes.farplane_video`), Lulu's Fury spins that often don't count
(`fixes.lulu_fury`).

- PS2 disc: not needed. Code only.

### Modern Minimap

The field minimap becomes round, always centred
on you and turned with the camera, so that up on the map is where the
camera looks. Your character is an arrow showing where they face; the
walkable area sits over a grid fixed to the world, in the game's minimap
colours on a small CRT screen (`minimap.style = "plain"` leaves the screen
effects out, `"game"` brings the game's own minimap back). The game's
markers keep their shapes and colours; those beyond the edge sit on the rim
and fade in as you come nearer, and where you are headed stays on the rim.
It shows and hides when the game's minimap would, and the fog of war still
clears as you walk. Size and corner are settings (`minimap.size`,
`minimap.corner`). [`docs/minimap.md`](docs/minimap.md) explains it.

- PS2 disc: not needed. Code only.

### FPS unlock

The 30 fps limit goes, the game's speed stays.
The game draws at your monitor's refresh rate, at a rate you choose, or
uncapped, with vsync on or off; the world keeps its 30 ticks a second, and
the camera, the characters (their bones, cloth and hair) and the map's
moving objects (animated parts, lifts, gates, boats) are drawn between
ticks. A time scale gives slow motion and fast-forward (×0.05 to
×100) without drawing more frames. Voice-synced scenes, movies and loads
stay at their own speed.

- Its settings are in the F8 window (a change applies at once and is saved
  into its `settings.toml`); **Alt+F9** switches vsync on and off;
  **Ctrl+F9** switches between fast-forward (×10) and ×1, **Shift+F9**
  between slow motion (×0.25) and ×1, **Ctrl+Shift+F9** back to ×1,
  **Ctrl+PgUp/PgDn** step the scale. A counter in the top-right corner shows
  the frame rate, the world's ticks a second, vsync and the scale.
- Effects, the 2D layers and blob shadows still move at 30 Hz (the effects
  at the game's speed, drawn on every frame); fast-forward costs CPU (every
  tick is simulated) and the counter says when it falls short.
  [`docs/fps-unlock.md`](docs/fps-unlock.md) explains how it works and its
  limits.
- PS2 disc: not needed. Code only.

### Debug Revival

The debug tools the developers left in `FFX.exe`, turned back on. **F9**
opens its window: an event-script debugger (every running script with its
disassembly, stacks and variables; pause, step, breakpoints), the
developers' PS2 debug menu drawn by the game's own code, the PC version's
ten developer panels, the battle debug switches and cheats with any battle
on demand, and in the field walking through walls, game speed, random
encounters, a free camera, jumps to any map and the developers' ~110 saves
for every part of the story. A **Debug flag** switches the game into the
developers' debug mode while it runs.

- Maps that would crash the game are refused; five unfinished areas get a
  stand-in script so you can walk around in them, and two of them, rooms in
  Bevelle that survive only as PS2 backgrounds in an older format, are
  rebuilt from your game files the first time the game runs with the mod
  (they show from the next start; ~35 MB in its folder). About 60 test
  maps with no text in your language show their Japanese text instead of
  crashing.
- Back up your saves first: the cheats and two of the panels change what
  your next save writes, and jumping out of story order can break your
  progress. Debug items that would hang the game, clear the story flags or
  reset your Steam achievements are blocked.
- No settings. Formerly its own mod `yuta.debugrevival`
  (github.com/yutamago/ffx-debug-revival-mod): delete
  `mods\yuta.debugrevival\` and its line in `mods\loadorder` when you
  install this one. [`docs/debug-revival/`](docs/debug-revival/README.md)
  explains every tab and the research behind it.
- PS2 disc: not needed.

## Documents

- [`docs/config.md`](docs/config.md): every setting, and which mod has it.
- [`docs/differences.md`](docs/differences.md): every difference between
  the two versions and how it is restored.
- [`docs/patches.md`](docs/patches.md), [`docs/render.md`](docs/render.md),
  [`docs/screen.md`](docs/screen.md), [`docs/ui.md`](docs/ui.md),
  [`docs/models.md`](docs/models.md), [`docs/music.md`](docs/music.md):
  how each change works and how it was checked.
- [`docs/minimap.md`](docs/minimap.md): the Modern Minimap.
- [`docs/fps-unlock.md`](docs/fps-unlock.md): FPS unlock, and how FFX.exe
  times its frames (what a tick is, the limiter, fast-forward).
- [`docs/debug-revival/`](docs/debug-revival/README.md): Debug Revival, its
  tools, and the research behind every address it uses.
- [`docs/disc.md`](docs/disc.md): how the PS2 disc stores its files.
- [`docs/ci.md`](docs/ci.md): CI and releases.

## Build

| Path | What |
|---|---|
| `converter/` | `demaster`, the converter (Rust; library and program). Uses the ffx-rust crates (`github.com/yutamago/ffx-rust`, public, pinned in `Cargo.toml`). |
| `live/` | `demaster_live.dll`, the live music engine (Rust, 32-bit like FFX.exe) |
| `common/` | `yuta.ffx-mods`, the shared C# library (a Fahrenheit mod): settings, build status, code patch helpers, the F8 window |
| `mods/<name>/` | the mods (C#, .NET 10, `Fahrenheit.Sdk/1.0.0-alpha12`), each with its `settings.default.toml` (the defaults; the player's `settings.toml` is made from it) and players' `README.txt` |
| `tests/` | C# tests (`tests/fps-unlock`: FPS unlock's portable part, built from its sources) |
| `tools/install/` | `install.ps1` and `install.cmd`, in every mod's zip |
| `tools/package.cs` | one zip per mod, and the PS2 bundle's |

```bash
cargo build --release -p demaster                     # Linux: target/release/demaster
cargo test -p demaster -p demaster-live               # converter tests, incl. that every mod's settings agree
tools/build-windows.sh                                # demaster.exe (cargo-xwin, static CRT)
tools/build-live.sh                                   # demaster_live.dll (32-bit)
dotnet build -c Release ffx-mods.slnx                 # artifacts/deploy/rel/mods/yuta.*/
dotnet test -c Release --no-build ffx-mods.slnx       # C# tests (FFX_GAME_DIR=<game folder> for those on FFX.exe)
dotnet run tools/package.cs -- dist 0.1.0             # dist/yuta.<mod>-0.1.0.zip, dist/yuta.ps2-bundle-0.1.0.zip
```

The converter builds one mod at a time: `demaster build --mod <name>
--mod-dir <mod folder> --iso <disc image or drive>` (`demaster mods` lists
the mods, their settings and jobs; `demaster jobs --mod <name>` the jobs).
A `demaster.exe` inside a mod's folder builds that mod. Without a mod it
builds every job into one folder from `demaster.toml` (development).

The disc can be a drive letter (`E:`): on Windows the converter reads the
drive's raw volume (`\\.\E:`) in whole 2048-byte sectors at sector offsets
into aligned memory (`converter/src/disc.rs`, `SectorReader`); images are
read the same way.

CI (`.github/workflows/build.yml`) builds the mods and runs their tests,
runs the converter tests on Linux, builds `demaster.exe` and `demaster_live.dll` on Windows,
packages every mod's zip and the PS2 bundle's and checks that they hold no game data, on every
push and pull request; a tag `v*` publishes a release with the zips
(`release.yml`); no secrets are needed ([`docs/ci.md`](docs/ci.md)).

## Contributing: adding a mod

A new mod is a folder `mods/<name>/` (its Fahrenheit id is `yuta.<name>`):

1. **Project.** `yuta.<name>.csproj` with `Sdk="Fahrenheit.Sdk/1.0.0-alpha12"`,
   `AssemblyName` `yuta.<name>`, `RootNamespace`, `AssemblyTitle` and
   `Description` (no `;` in it: it goes into the manifest).
   `mods/Directory.Build.props` adds the rest: the version, a reference to
   the shared library `common/` (not copied: Fahrenheit loads it once from
   `mods/yuta.ffx-mods` and shares it), its global usings, and
   `yuta.ffx-mods` as the manifest's dependency; `settings.default.toml`
   and `README.txt` are copied into the build. Add the project to
   `ffx-mods.slnx`.
2. **Module.** `Mod.cs`: an `FhModule` with `[FhLoad(FhGameId.FFX)]`, its
   `SETTINGS` (`SettingInfo`: key, who applies it, default), its `PATCHES`
   (`CodePatch`: the setting, its "off" value, a description and the apply
   function) and a `ModInfo`; `init` calls `ModHost.start(this, _logger,
   mod_context, INFO)`. That reads the mod's `settings.toml`, checks its
   build stamp, applies the patches and puts the mod into the one F8
   window. An apply function gets the `ModHost` (its log, its folder; it
   converts to the `FhModule` for `Native.fn<T>(va).hook(m, ...)`).
   Copy `mods/ps2-game-feel/` for a code-only mod.
3. **Shared code** goes into `common/` (`Native` for FFX.exe addresses and
   byte patches, `Fn` for function shapes, `ModFiles` for the PS2 files of
   every installed mod), never copied into a mod. A hook on a function
   another mod may hook too must call the next one (`chain_from`). A mod
   whose settings change while the game runs draws them in its part of the
   F8 window (`ModInfo.Section`; `FamilyWindow.toggle` for a key of its own)
   and writes them back with `ModSettings.write` (as FPS unlock does).
4. **Settings.** Each setting is in `converter/src/config.rs` (`Config`,
   `SETTINGS`), the mod's entry in `converter/src/mods.rs`, its `Mod.cs`,
   its `settings.default.toml` (with comments; `[source]` only if it has
   PS2 files; the zip ships it, `install.cmd` makes the player's
   `settings.toml` from it)
   and `docs/config.md`. `cargo test -p demaster` checks they agree, and
   that no setting or job belongs to two mods.
5. **PS2 files.** A mod that builds files lists its jobs in its `mods.rs`
   entry (`jobs`, `disc`: needed, optional or none); the jobs are
   `jobs::Job`s in `jobs::registry()` (`docs/` and the existing jobs show
   how). A job may read only its own mod's settings, writes with
   `out.put(...)` into the mod's `efl/x` and says `needs_disc() = false`
   if it builds from the PC game alone. `demaster build --mod <name>` and a
   `demaster.exe` in the mod's folder then build exactly those jobs, and
   `tools/install/install.ps1` (shipped as the mod's `install.cmd`) runs
   it: it finds the mod by its manifest, asks for the disc only when the
   mod has disc jobs (`demaster mods`), and adds the mod to `mods\loadorder`.
6. **Package and CI.** Add the mod to `MODS` in `tools/package.cs` (whether
   it ships `demaster.exe`) and to the zip check in
   `.github/workflows/build.yml`; a PS2 mod also goes into the PS2 bundle
   (its id in `FfxModsDependencies` of `mods/ps2-bundle/yuta.ps2-bundle.csproj`,
   in the converter's order, and in the bundle's `README.txt`); describe it in this README (what it
   changes for the player, disc, size, dependencies) and in its
   `README.txt`. Tests of a mod's portable code go in `tests/<name>/`
   (in `ffx-mods.slnx`; CI runs them).
7. **Test in the game** before calling it done (`CLAUDE.md`, "Testing in
   the game"): alone, and with the other mods.

Never commit disc images, extracted files or anything from the game's
archives.
