# CI and releases

## Workflows

`.github/workflows/build.yml` runs on every push to a branch, on pull
requests and by hand (Actions > Build > Run workflow):

| Job | Runner | What |
|---|---|---|
| Mods (C#) | ubuntu-latest | `dotnet build -c Release ffx-mods.slnx`: the shared library and every mod; then `dotnet test` of the C# tests (`tests/fps-unlock`: FPS unlock's timing, interpolation, settings and hook checks; the tests against the game's `FFX.exe` skip themselves without `FFX_GAME_DIR`); uploads the mod folders (artifact `mods`). |
| Converter tests (Linux) | ubuntu-latest | `cargo test --locked -p demaster -p demaster-live`, including the check that every mod's settings agree across `config.rs`, `mods.rs`, `mods/<name>/Mod.cs`, `settings.toml` and `docs/config.md`. Tests that need the disc or the game skip themselves. |
| Windows build and package | windows-latest | `demaster.exe` (`x86_64-pc-windows-msvc`) and `demaster_live.dll` (`i686-pc-windows-msvc`), static CRT from `.cargo/config.toml`; runs `demaster.exe --version` and `jobs`; packages every mod's zip with `tools/package.cs` from the `mods` artifact and checks each: the shared library, the mod, `settings.toml`, `README.txt`, `LICENSES.txt`, `install.cmd`/`install.ps1`, `demaster.exe` exactly for the four mods with PS2 files, `demaster_live.dll` for the music engine, and nothing else (no `efl/`, `build/`, `music/`, `.phyre`, `.iso`, images, sound, movies). Uploads `zips` (`yuta.<mod>-ci.zip`). |

`.github/workflows/release.yml` runs when a tag `v*` is pushed. It calls
`build.yml` with the tag, which sets the version of the converter
(`DEMASTER_VERSION`) and of the mods (`-p:Version`) to the tag without the
`v` (each mod compares its version with the converter's in its build
stamp), then creates a GitHub release named "FFX mods <tag>" with every
`yuta.<mod>-<tag>.zip`, a short install note and GitHub's generated notes.
A tag with a `-` (`v0.2.0-alpha`) makes a prerelease. To release:

```bash
git tag v0.2.0 && git push origin v0.2.0
```

## ffx-rust

The converter uses crates from ffx-rust (`github.com/yutamago/ffx-rust`,
public), fetched by cargo from git at the revision pinned in `Cargo.toml`.
CI needs no secret for that, and pull requests from forks build the same
way. When the pinned revision changes, CI fetches the new one; nothing
else needs updating.
