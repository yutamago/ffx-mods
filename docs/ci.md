# CI and releases

## Workflows

`.github/workflows/build.yml` runs on every push to a branch, on pull
requests and by hand (Actions > Build > Run workflow):

| Job | Runner | What |
|---|---|---|
| Mods (C#) | ubuntu-latest | `dotnet build -c Release ffx-mods.slnx`: the shared library and the seven mods; uploads their folders (artifact `mods`). Needs no secret. |
| Converter tests (Linux) | ubuntu-latest | `cargo test --locked -p demaster -p demaster-live`, including the check that every mod's settings agree across `config.rs`, `mods.rs`, `mods/<name>/Mod.cs`, `settings.toml` and `docs/config.md`. Tests that need the disc or the game skip themselves. |
| Windows build and package | windows-latest | `demaster.exe` (`x86_64-pc-windows-msvc`) and `demaster_live.dll` (`i686-pc-windows-msvc`), static CRT from `.cargo/config.toml`; runs `demaster.exe --version` and `jobs`; packages the seven zips with `tools/package.cs` from the `mods` artifact and checks each: the shared library, the mod, `settings.toml`, `README.txt`, `LICENSES.txt`, `install.cmd`/`install.ps1`, `demaster.exe` exactly for the four mods with PS2 files, `demaster_live.dll` for the music engine, and nothing else (no `efl/`, `build/`, `music/`, `.phyre`, `.iso`, images, sound, movies). Uploads `zips` (`yuta.<mod>-ci.zip`). |

`.github/workflows/release.yml` runs when a tag `v*` is pushed. It calls
`build.yml` with the tag, which sets the version of the converter
(`DEMASTER_VERSION`) and of the mods (`-p:Version`) to the tag without the
`v` (each mod compares its version with the converter's in its build
stamp), then creates a GitHub release named "FFX mods <tag>" with the seven
`yuta.<mod>-<tag>.zip`, a short install note and GitHub's generated notes.
A tag with a `-` (`v0.2.0-alpha`) makes a prerelease. To release:

```bash
git tag v0.2.0 && git push origin v0.2.0
```

## The FFX_RUST_TOKEN secret

The converter uses crates from ffx-rust (`github.com/yutamago/ffx-rust`),
a private repository, so cargo needs a token to fetch it. The jobs that build
Rust start with `.github/actions/ffx-rust-access`, which reads the
repository secret `FFX_RUST_TOKEN`, rewrites the ffx-rust URL to carry the
token (`git config url.<token URL>.insteadOf`), lets cargo fetch through the
git CLI (`CARGO_NET_GIT_FETCH_WITH_CLI`) and checks with `git ls-remote`
that the token can read ffx-rust. Without the secret, or with a token that
can't read ffx-rust, the job fails there with an error that says so; the
mod job still runs.

To create it:

1. On GitHub, signed in as `yutamago`: Settings > Developer settings >
   Personal access tokens > Fine-grained tokens > Generate new token
   (<https://github.com/settings/personal-access-tokens/new>).
2. Name: e.g. `ffx-mods CI: read ffx-rust`. Expiration: your choice
   (CI fails with the "cannot read ffx-rust" error once it expires).
3. Resource owner: `yutamago`.
4. Repository access: Only select repositories > `yutamago/ffx-rust`.
5. Permissions > Repository permissions > Contents: Read-only
   (Metadata: Read-only is added by itself). Nothing else.
6. Generate the token and copy it.
7. In `github.com/yutamago/ffx-mods`: Settings > Secrets and variables >
   Actions > New repository secret. Name `FFX_RUST_TOKEN`, value: the token.
   (Or `gh secret set FFX_RUST_TOKEN -R yutamago/ffx-mods` and paste it.)
   The token of ffx-demaster's CI works here too if its repository access
   is ffx-rust only (it is a read token for ffx-rust, not tied to the
   repository that uses it).

Then re-run the last Build run (Actions > the run > Re-run all jobs).

Pull requests from forks get no secrets, so their Rust jobs fail at
"ffx-rust access"; the mod job still checks the C# side.

When the pinned ffx-rust revision in `Cargo.toml` changes, CI fetches it
with the same token; nothing else needs updating.
