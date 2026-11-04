# CI and releases

## Workflows

`.github/workflows/build.yml` runs on every push to a branch, on pull
requests and by hand (Actions > Build > Run workflow):

| Job | Runner | What |
|---|---|---|
| Mod (C#) | ubuntu-latest | `dotnet build -c Release ffx-demaster.slnx`; uploads the mod folder (artifact `mod`). Needs no secret. |
| Converter and studio tests (Linux) | ubuntu-latest | `cargo test --locked -p demaster -p ffx-music-studio`. Tests that need the disc or the game skip themselves. |
| Windows build and package | windows-latest | `cargo build --locked --release --target x86_64-pc-windows-msvc -p demaster -p ffx-music-studio` (static CRT from `.cargo/config.toml`), runs `demaster.exe --version` and `jobs`, packages the zip with `tools/package.cs` from the `mod` artifact and checks it: the required files are there and nothing else (no `efl/`, `build/`, `tools/`, `.phyre`, `.iso`, images, sound, movies). Uploads `zip` (`yuta.demaster-ci.zip`) and `studio` (`ffx-music-studio.exe`). |

`.github/workflows/release.yml` runs when a tag `v*` is pushed. It calls
`build.yml` with the tag, which sets the version of the converter
(`DEMASTER_VERSION`) and of the mod (`-p:Version`) to the tag without the
`v` (the mod compares the two), then creates a GitHub release named
"FFX Demaster <tag>" with `yuta.demaster-<tag>.zip` and
`ffx-music-studio.exe`, a short install note and GitHub's generated notes.
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
2. Name: e.g. `ffx-demaster CI: read ffx-rust`. Expiration: your choice
   (CI fails with the "cannot read ffx-rust" error once it expires).
3. Resource owner: `yutamago`.
4. Repository access: Only select repositories > `yutamago/ffx-rust`.
5. Permissions > Repository permissions > Contents: Read-only
   (Metadata: Read-only is added by itself). Nothing else.
6. Generate the token and copy it.
7. In `github.com/yutamago/ffx-demaster`: Settings > Secrets and variables >
   Actions > New repository secret. Name `FFX_RUST_TOKEN`, value: the token.
   (Or `gh secret set FFX_RUST_TOKEN -R yutamago/ffx-demaster` and paste it.)

Then re-run the last Build run (Actions > the run > Re-run all jobs).

Pull requests from forks get no secrets, so their Rust jobs fail at
"ffx-rust access"; the mod job still checks the C# side.

When the pinned ffx-rust revision in `Cargo.toml` changes, CI fetches it
with the same token; nothing else needs updating.
