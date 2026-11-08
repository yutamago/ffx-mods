# FFX mods

All of Yutamago's [Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit)
mods for FINAL FANTASY X HD Remaster (Steam, Windows). Most bring back parts
of the original PS2 game, with the PC version's game logic underneath (the
PS2 mods); others change the game in their own way. Install any of them,
alone or together; each is a separate zip.

| Mod | What it changes | PS2 disc | Installed size | Needs |
|---|---|---|---|---|
| [PS2 music engine](#ps2-music-engine) (`yuta.ps2-music-engine`) | the original soundtrack played by the PS2's own sound driver inside the game | yes | ~60 MB | — |
| [PS2 characters](#ps2-characters) (`yuta.ps2-characters`) | PS2 models and textures for the party, NPCs, monsters, aeons, weapons, objects | yes | ~860 MB | — |
| [PS2 lighting](#ps2-lighting) (`yuta.ps2-lighting`) | characters lit the PS2 way | optional | ~80 MB | — |
| [PS2 interface](#ps2-interface) (`yuta.ps2-interface`) | PS2 menus, windows, font, portraits, title screen, button glyphs; a clean start-up | yes | ~180 MB | PS2 aspect ratio recommended |
| [PS2 aspect ratio](#ps2-aspect-ratio) (`yuta.ps2-aspect-ratio`) | the 4:3 picture with the PS2 camera | no | <1 MB | — |
| [PS2 game feel](#ps2-game-feel) (`yuta.ps2-game-feel`) | PS2 run speed, battle menu without input delay, battle subtitles | no | <1 MB | — |
| [Bug fixes](#bug-fixes) (`yuta.bug-fixes`) | NPC mouths, the Farplane video, Lulu's Fury spins | no | <1 MB | — |
| [Modern Minimap](#modern-minimap) (`yuta.modern-minimap`) | a round field minimap, centred on you and turned with the camera | no | <1 MB | — |

Every mod needs the shared library `yuta.ffx-mods` (in every zip; its
manifest dependency). Sizes are the files `install.cmd` builds with the
default settings, measured with the German PAL disc; the program files add
5 MB for mods with PS2 files, under 0.1 MB for the others.

**No game data is included.** The mods with PS2 files build them on your
computer from your own Final Fantasy X PS2 disc (any region): an image of
it (`.iso`), or the disc itself in a disc drive. After that they need
neither.

[FFX Demaster](https://github.com/yutamago/ffx-demaster) is the whole
package (the PS2 mods here plus the PS2 movies, maps, effects and more).

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
   the mod to `mods\loadorder`. For a mod with PS2 files it then asks where
   your disc is (the disc drives with the disc in them are listed; or pick
   an image file), finds the game in the Steam folders (or asks for its
   folder), and builds the mod's PS2 files into its folder. From a
   terminal: `install.cmd -Iso 'D:\PS2\FFX.iso'`, `install.cmd -Drive E`,
   `-Game <game folder>`, `-NoDisc` (PS2 lighting only), `-Force`
   (rebuild everything). Running it again is safe: it redoes only what
   changed.
3. Start the game through Fahrenheit (`bin\fhstage0.exe <path to FFX.exe>`).

In the game, **F8** opens one window for every installed mod of the
family: for the mods with PS2 files whether their files match their
settings (and the command to rebuild them), and which code patches are on.
It opens by itself when something is wrong.

**Settings:** each mod has `settings.toml` in its folder, with a comment
above each setting; the defaults are the PS2's ([`docs/config.md`](docs/config.md)
describes every setting). After changing one, run that mod's `install.cmd`
again if it has PS2 files, then restart the game.

**Uninstall:** delete `mods\yuta.<mod>\` and its line in `mods\loadorder`
(the PS2 files it built are in its folder and go with it). When no mod of
the family is left, delete `mods\yuta.ffx-mods\` and its line too.

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

### PS2 lighting

Characters lit the PS2 way: the PS2's light colours and directions at its
level, its hard falloff instead of the HD's soft one, no HD fill light,
point lights only where the PS2 used them. The character shaders are
patched copies of the game's own, so that part needs no disc. With the
disc, the six cutscenes whose lights the HD retuned (the opening's Tidus
close-up among them) also get the PS2's light colours back; without it
they stay HD, and `install.cmd` (with "no disc" or `-NoDisc`) says so.

- PS2 disc: optional. Size: ~80 MB (the event lights: 0.5 MB of that).

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
16:9: while one plays full screen the picture goes back to 16:9 for it (FFX
Demaster's PS2 movies are 4:3 and stay so).

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

## Documents

- [`docs/config.md`](docs/config.md): every setting, and which mod has it.
- [`docs/differences.md`](docs/differences.md): every difference between
  the two versions and how it is restored.
- [`docs/patches.md`](docs/patches.md), [`docs/render.md`](docs/render.md),
  [`docs/screen.md`](docs/screen.md), [`docs/ui.md`](docs/ui.md),
  [`docs/models.md`](docs/models.md), [`docs/music.md`](docs/music.md):
  how each change works and how it was checked.
- [`docs/minimap.md`](docs/minimap.md): the Modern Minimap.
- [`docs/disc.md`](docs/disc.md): how the PS2 disc stores its files.
- [`docs/ci.md`](docs/ci.md): CI and releases.

## Build

| Path | What |
|---|---|
| `converter/` | `demaster`, the converter (Rust; library and program). Uses the ffx-rust crates (`github.com/yutamago/ffx-rust`, public, pinned in `Cargo.toml`). |
| `live/` | `demaster_live.dll`, the live music engine (Rust, 32-bit like FFX.exe) |
| `common/` | `yuta.ffx-mods`, the shared C# library (a Fahrenheit mod): settings, build status, code patch helpers, the F8 window |
| `mods/<name>/` | the mods (C#, .NET 10, `Fahrenheit.Sdk/1.0.0-alpha12`), each with its `settings.toml` and players' `README.txt` |
| `tools/install/` | `install.ps1` and `install.cmd`, in every mod's zip |
| `tools/package.cs` | one zip per mod |

```bash
cargo build --release -p demaster                     # Linux: target/release/demaster
cargo test -p demaster -p demaster-live               # converter tests, incl. that every mod's settings agree
tools/build-windows.sh                                # demaster.exe (cargo-xwin, static CRT)
tools/build-live.sh                                   # demaster_live.dll (32-bit)
dotnet build -c Release ffx-mods.slnx                 # artifacts/deploy/rel/mods/yuta.*/
dotnet run tools/package.cs -- dist 0.1.0             # dist/yuta.<mod>-0.1.0.zip
```

The converter builds one mod at a time: `demaster build --mod <name>
--mod-dir <mod folder> --iso <disc image or drive>` (`demaster mods` lists
the mods, their settings and jobs; `demaster jobs --mod <name>` the jobs).
A `demaster.exe` inside a mod's folder builds that mod. Without a mod it
builds every job into one folder from `demaster.toml`, as FFX Demaster
does.

The disc can be a drive letter (`E:`): on Windows the converter reads the
drive's raw volume (`\\.\E:`) in whole 2048-byte sectors at sector offsets
into aligned memory (`converter/src/disc.rs`, `SectorReader`); images are
read the same way.

CI (`.github/workflows/build.yml`) builds the mods, runs the converter
tests on Linux, builds `demaster.exe` and `demaster_live.dll` on Windows,
packages every mod's zip and checks that they hold no game data, on every
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
   `yuta.ffx-mods` as the manifest's dependency; `settings.toml` and
   `README.txt` are copied into the build. Add the project to
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
   another mod may hook too must call the next one (`chain_from`).
4. **Settings.** Each setting is in `converter/src/config.rs` (`Config`,
   `SETTINGS`), the mod's entry in `converter/src/mods.rs`, its `Mod.cs`,
   its `settings.toml` (with comments; `[source]` only if it has PS2 files)
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
   `.github/workflows/build.yml`; describe it in this README (what it
   changes for the player, disc, size, dependencies) and in its
   `README.txt`.
7. **Test in the game** before calling it done (`CLAUDE.md`, "Testing in
   the game"): alone, and with the other mods.

Never commit disc images, extracted files or anything from the game's
archives.
