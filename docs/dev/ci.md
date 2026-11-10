# CI and releases

## Workflows

`.github/workflows/build.yml` runs on every push to a branch, on pull
requests and by hand (Actions > Build > Run workflow):

| Job | Runner | What |
|---|---|---|
| Mods (C#) | ubuntu-latest | `dotnet build -c Release ffx-mods.slnx`: the shared library and every mod; then `dotnet test` of the C# tests (`tests/fps-unlock`: FPS unlock's timing, interpolation, settings and hook checks; the tests against the game's `FFX.exe` skip themselves without `FFX_GAME_DIR`); builds Debug Revival's tools (`tools/debug-revival`; running them needs the game); uploads the mod folders (artifact `mods`). |
| Converter tests (Linux) | ubuntu-latest | `cargo test --locked -p demaster -p demaster-live`, including the check that every mod's settings agree across `config.rs`, `mods.rs`, `mods/<name>/Mod.cs`, `settings.default.toml` and `docs/config.md`, and that an older settings file gets the newer settings' defaults. Tests that need the disc or the game skip themselves. |
| Windows build and package | windows-latest | `demaster.exe` (`x86_64-pc-windows-msvc`) and `demaster_live.dll` (`i686-pc-windows-msvc`), static CRT from `.cargo/config.toml`; runs `demaster.exe --version` and `jobs`; packages every mod's zip and the PS2 bundle's with `tools/package/package.cs` from the `mods` artifact and checks each. A mod's: the shared library, the mod, `settings.default.toml` (never `settings.toml`: that is the player's, made by `install.cmd`; none for Debug Revival, which has no settings), `README.txt`, `LICENSES.txt`, `install.cmd`/`install.ps1`, `demaster.exe` exactly for the mods with PS2 files, `demaster_live.dll` for the music engine. The bundle's (`yuta.ps2-bundle`): exactly the mods its manifest lists (`Dependencies`), each as above but without `demaster.exe`; the shared library with the one `demaster.exe`, the installer and `LICENSES.txt`; the bundle's folder with its DLL, manifest, `README.txt`, `LICENSES.txt` and the installer. Every zip: nothing else (no `settings.toml`, `efl/`, `build/`, `music/`, `.phyre`, `.iso`, images, sound, movies). Then the installer's tests (`tools/install/install.tests.ps1`) under Windows PowerShell 5.1 and pwsh: its settings merge, switched-off mods and load order in a made-up Fahrenheit folder, and the bundle's `install.cmd` run from its zip with the mods that build PS2 files switched off (no disc needed). Uploads `zips` (`yuta.<mod>-ci.zip`, `yuta.ps2-bundle-ci.zip`). |

`.github/workflows/release.yml` runs when a tag `v*` is pushed. It calls
`build.yml` with the tag, which sets the version of the converter
(`DEMASTER_VERSION`) and of the mods (`-p:Version`) to the tag without the
`v` (each mod compares its version with the converter's in its build
stamp), then creates a GitHub release named "FFX mods <tag>" with every
`yuta.<mod>-<version>.zip` and `yuta.ps2-bundle-<version>.zip` (the version
is the tag without the `v`), a short install note and GitHub's generated
notes.
A tag with a `-` (`v0.2.0-alpha`) makes a prerelease. To release:

```bash
git tag v0.2.0 && git push origin v0.2.0
```

The PS2 bundle (formerly FFX Demaster, built by ffx-demaster's CI from a
pinned revision of this repository until 2026-11-09) is built here, from
the same revision as the mods: `tools/package/package.cs` reads the bundle's mods
from `yuta.ps2-bundle.manifest.json` and puts the one `demaster.exe` into
the shared library's folder.

## ffx-rust

The converter uses crates from ffx-rust (`github.com/yutamago/ffx-rust`,
public), fetched by cargo from git at the revision pinned in `Cargo.toml`.
CI needs no secret for that, and pull requests from forks build the same
way. When the pinned revision changes, CI fetches the new one; nothing
else needs updating.
