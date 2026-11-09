# ffx-mods

All of the user's Fahrenheit mods for the FFX HD Remaster (Steam), each
working alone and in any combination, plus the converter (`demaster`) that
builds the PS2 mods' files from the player's own disc. Most are PS2 mods
(they restore parts of the PS2 version: presentation from the PS2, logic
from the PC); others are the user's own changes (`modern-minimap`, `fps-unlock`). Every
change is a setting. The PS2 mods were split out of ffx-demaster (github.com/yutamago/ffx-demaster,
history kept up to its 314bbfc). Their bundle (formerly FFX Demaster) is the
mod `mods/ps2-bundle` (`yuta.ps2-bundle`): its manifest's dependencies (its
csproj's `FfxModsDependencies`) are the twelve PS2 mods, `tools/package.cs`
makes `yuta.ps2-bundle-<version>.zip` with exactly those, and its
`install.cmd` installs and builds them all (`demaster build --all`).
ffx-demaster is now the research and test-tool home of the PS2 mods: the
PS2-vs-PC tools (`tools/shotrun`, `tools/calllist`, `tools/replays`,
`tools/compare`, `docs/compare.md`, `docs/debugger.md`) are there.
README.md describes each mod and the bundle for players.

## Rules

- Never commit game data: no disc images, extracted files, `.phyre`, PNG/DDS
  output, built mod folders. The zips ship code only; `install.cmd` builds
  the PS2 files from the player's own disc on their machine.
- Never add a `Co-Authored-By: Claude` footer (or any Claude attribution) to
  commit messages, PR descriptions or other messages.
- Pushing straight to `main` is fine. Agents working in a worktree commit on
  their branch; the orchestrator merges.
- ffx-rust (`../ffx-decomp`, public, github.com/yutamago/ffx-rust) is ours:
  use its crates and docs freely (git dependency, pinned rev in `Cargo.toml`).
  Its `docs/formats/*.md` and `symbols/ffx.sym` are the reference for FFX.exe.
- Decompilation findings go back into ffx-rust: every new FFX.exe
  name/address/struct/format fact (and PS2 ELF / IOP / disc format facts) is
  logged in `docs/ffx-rust-findings.md` (address, name, evidence, which of
  our docs explains it) as you find it. The orchestrator ports them to
  ffx-rust on a branch + PR there; agents don't edit ffx-rust directly.
- Fahrenheit (`../fahrenheit`) is a reference for facts; the C# mods use its
  SDK (`Fahrenheit.Sdk/1.0.0-alpha12`).
- Check results by eye: write PNG previews and look at them, and test in the
  game (below) before calling a feature done. Compare against the PS2
  original, not against what seems plausible.
- Python: `uv run --with <deps> python -I script.py`. Rust: if cargo stalls
  on `sccache`, build with `RUSTC_WRAPPER=`.

## Layout

| Path | What |
|---|---|
| `converter/` | crate `demaster` (lib + binary): the jobs, the build, the disc reader. `mods.rs` lists each mod's settings and jobs. |
| `live/` | crate `demaster-live`: `demaster_live.dll` (32-bit, the live PS2 music engine, `tools/build-live.sh`) |
| `common/` | `yuta.ffx-mods`, the shared C# library (itself a Fahrenheit mod): `Native`/`Fn`, `ModHost` (settings.toml, build status, patches), the one F8 window (`FamilyWindow`), `ModFiles` (the PS2 files of every installed mod) |
| `mods/<name>/` | the mods, `yuta.<name>`: `Mod.cs` (settings with defaults, code patches), the patch code, `settings.toml` (defaults), `README.txt` (for the zip) |
| `mods/ps2-bundle/` | the PS2 bundle, `yuta.ps2-bundle`: no settings, no patches, not in `mods.rs`; its csproj lists the bundle's mods (manifest `Dependencies`), its `Mod.cs` shows them in F8, its `README.txt` is the bundle zip's |
| `tests/<name>/` | C# tests of a mod's portable part (its sources linked in; the mods build for win-x86 against Fahrenheit): `tests/fps-unlock`. Tests that need the game read `FFX_GAME_DIR` and skip without it |
| `tools/install/` | `install.ps1` + `install.cmd`, shipped in every mod folder |
| `tools/package.cs` | one zip per mod, one per bundle (`BUNDLES`) |
| `docs/` | moved from ffx-demaster: the research and design docs of every feature (also of the parts that stay in ffx-demaster) |

The mods and what is in each: README.md and `converter/src/mods.rs`.

## Rules for the mods

- Each mod works alone and with any other: no mod reads another mod's
  setting, a converter job reads only its own mod's settings (a test in
  `mods.rs` checks), no two mods write the same file. One exception, made
  by the converter: a job whose output depends on what another mod does in
  the game declares that setting in its mod's `reads` (`mods.rs`,
  `Foreign`: `movies` and `maps.backgrounds` read `render.aspect`); the
  build takes it from that mod's `settings.toml` when the mod is installed
  (in `mods\loadorder`), else the `absent` value, records it in the stamp
  (`foreign`), and the C# `BuildStatus` shows "rebuild needed" when it
  changes. Hooks on the same
  function chain (Fahrenheit's `chain_from`); always call the next hook.
- Shared state lives in `common/` (loaded once for all mods); never copy
  shared code into a mod.
- Settings are read once at startup, except a mod's that reads them live
  (FPS unlock): it draws them in its part of the F8 window
  (`ModInfo.Section`, opened by its own key through `FamilyWindow.toggle`)
  and writes changes back with `ModSettings.write` (the changed lines only).
- A setting lives in 4 places a test keeps in sync: `converter/src/config.rs`
  (`SETTINGS`), `converter/src/mods.rs` (which mod), `mods/<name>/Mod.cs`
  (with its default, and a `CodePatch` if the mod applies it),
  `mods/<name>/settings.toml`, and `docs/config.md`.
- Every setting belongs to one mod (a test; `maps.geometry` has none yet).
- A new job goes in `jobs::registry()` and in its mod's `jobs` in `mods.rs`;
  say `needs_disc() = false` if it builds from the PC game alone.
- A change that alters a job's output must bump that job's `Job::version`, or
  `demaster build` keeps the old files.
- Jobs must be deterministic (same input, same bytes): no HashMap iteration
  order in results; parallel work writes in a fixed order.
- A new PS2 mod goes into the bundle: `FfxModsDependencies` in
  `mods/ps2-bundle/yuta.ps2-bundle.csproj` (converter order) and its
  `README.txt`. The installer treats any mod whose manifest depends on
  family mods besides `yuta.ffx-mods` as a bundle (installs everything, puts
  it after the mods in `mods\loadorder`). ffx-remix's `yuta.ffxremix.*` mods
  are not bundles: they use the shared library and the F8 window but depend on their
  own `yuta.ffxremix.core`, and ffx-remix's launcher installs them. A mod
  with no settings needs no `settings.toml` (`ModSettings.Missing`).
- The converter is a library too (`src/lib.rs`, every module `pub`) for
  ffx-demaster and FFX Music Studio: keep the public API of `sqseq`,
  `spu2`, `musicjob`, `disc`, `config`, `build`.
- The manifest of each mod lists `yuta.ffx-mods` as a dependency
  (`mods/Directory.Build.targets`); manifest texts must not contain `;`.

```bash
RUSTC_WRAPPER= cargo build --release -p demaster
RUSTC_WRAPPER= cargo test --release -p demaster -p demaster-live   # disc tests use /tmp/dm-ffx.iso
./target/release/demaster build --mod ps2-lighting --mod-dir <dir> --iso /tmp/dm-ffx.iso --game ~/steam-games/FFX-X2-HD
./target/release/demaster build --all --mods-dir <Fahrenheit>/mods --iso ...  # every installed mod, one disc pass
tools/build-windows.sh && tools/build-live.sh       # demaster.exe, demaster_live.dll
dotnet build -c Release ffx-mods.slnx               # artifacts/deploy/rel/mods/yuta.*/
dotnet test -c Release --no-build ffx-mods.slnx     # C# tests; FFX_GAME_DIR=<game folder> for the FFX.exe ones
dotnet run tools/package.cs -- <out dir> <version>  # one zip per mod + yuta.ps2-bundle-<version>.zip
```

CI and tag releases: `docs/ci.md` (no secrets needed).

## Data (never in git)

| What | Where |
|---|---|
| PS2 disc (German PAL, SCES_504.92) | `~/ffx-ps2/Final Fantasy X (Germany).iso`; symlink it to `/tmp/dm-ffx.iso` for tools (its `(`/`)` upset the agent sandbox) |
| PS2 files extracted under PC names | `~/ffx-work/demaster/ps2/` (`index.tsv`) |
| PC game | `~/steam-games/FFX-X2-HD` (`data/FFX_Data.vbf`); `FFX.exe` sha256 in ffx-rust `symbols/ffx.sym` |
| Game copy for Fahrenheit tests | `~/ffx-work/demaster/game` (no ffx-rust `DINPUT8.dll`) |
| Scratch | `~/ffx-work/demaster/scratch/ffx-mods` |
| Ghidra dump of FFX.exe | `~/ffx-work/decomp` (grep first) |

Windows processes see WSL symlinks through `\\wsl.localhost` as missing:
give them the real path (`\\wsl.localhost\Ubuntu\home\angelo\ffx-ps2\Final Fantasy X (Germany).iso`).

## Testing in the game

- Fahrenheit install for these mods: `C:\_workspaces\fh-mods-test` (alpha12;
  `mods/` holds whatever is under test); `C:\_workspaces\fh-bundle-test`
  for the PS2 bundle. Never touch `C:\_workspaces\fh-demaster-test` (the
  user's install).
- Install a mod as a player does: unpack its zip (`tools/package.cs`) into
  the install and run its `install.cmd` (`-Iso <real Windows path> -Game
  <game> -NoPause`); it writes `mods\loadorder`. In `mods\yuta.ps2-bundle\`
  (and `mods\yuta.ffx-mods\`) `install.cmd` installs every mod there; a mod
  folder without `demaster.exe` uses the shared one. From WSL:
  `powershell.exe -File <Windows path>\install.ps1 ...`.
- Every game run under `flock /home/angelo/ffx-work/demaster/fh-test.lock`
  (the user's other sessions use it), one game at a time. Launch
  `bin\fhstage0.exe '\\wsl.localhost\Ubuntu\home\angelo\ffx-work\demaster\game\FFX.exe' --extdbg`
  (`--extdbg` skips the ~45 s Sophos freeze; dumps at `SophosED.dll+0x7d193`
  in `crash/` are that: delete them). End only the fhstage0.exe PID you
  started (`taskkill.exe /PID <pid> /F`; its FFX.exe closes with it); never
  kill FFX by name: other sessions run their own games. A second game
  running at the same time (another session's) keeps a new boot from
  starting: wait for it.
- Capture only the game window (by the PID of the FFX.exe your fhstage0
  started, `PrintWindow`), never the desktop.
- Logs: `logs/<newest>/Yuta.FfxMods.*.log` (one per mod, plus the shared
  library's), "replaces file" lines in `Fahrenheit.Runtime.FhFileLoaderModule.log`.
- The boot intro (logo, staff credits, the campfire with the party, then
  the title) needs no input: it shows characters, lighting, 4:3, credits
  and the title screen.
- Disk is tight on C: delete test outputs and screenshots when done.
