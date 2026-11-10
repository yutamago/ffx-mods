# Fahrenheit and asset replacement for the demaster mod

Sources read (2026-11-02):
- `~/workspaces/oss_ws/fahrenheit` at `f7e2258`. Its `src/runtime/fileloader.cs`, `cd.cs`, `core/paths.cs`, `settings.cs`, `handles.cs`, `bootloader.cs` and `sdk/Sdk/Sdk.props` are identical to `fahrenheit.upstream`. Since tag `v1.0.0-alpha12`, `fileloader.cs` has only changed by an interop refactor and a doc comment.
- `fahrenheit.wiki`, `ffx-debug-revival` at `20a61cf`, and `ffx-decomp` (`docs/`, `symbols/ffx.sym`, `crates/`).

Paths below are relative to `fahrenheit/` unless they name another repo. Decomp addresses are absolute VAs (base `0x400000`). Fahrenheit offsets are relative to the image base, so `0x207BC0` = `0x00607bc0`.

Labels used below: **[code]** means read directly in source. **[decomp]** means taken from ffx-decomp symbols or ported code and not run in the game by me. **[inference]** means my reasoning, not tested.

---

## 1. How a mod replaces game files: the EFL

Fahrenheit has a built-in **external file loader (EFL)**, `FhFileLoaderModule` (`src/runtime/fileloader.cs:26-346`). It is part of the runtime mod `fhr`, so every mod gets it. No API calls are needed: it replaces files with loose files from the mod folder.

**Layout.** The mod's EFL root is `mods/<mod_id>/efl/` (`src/core/paths.cs:39,99`), with two subfolders:
- `efl/x` for FFX (`FFX_Data.vbf`)
- `efl/x2` for X-2

Below the subfolder you mirror the VBF path (wiki `Mod-structure.md:51-60`):

```
mods/<mod_id>/efl/x/ffx_data/gamedata/ps3data/chr/pc/c001/tex/d3d11/c001.dds.phyre
mods/<mod_id>/efl/x/ffx_ps2/ffx/master/jppc/battle/kernel/takara.bin
```

**How lookup works.**
- **Index build.** At `init`, `_init_index` walks every mod in load order (`fileloader.cs:92-100`). It indexes `efl/x2` and then `efl/x`, so for FFX the `x` files win (`:94-98`). Keys are `"/" + relative path`, with `/` separators, upper-cased (`:71-82`). The lookup is case-insensitive. Note that `x2` files go into the same dictionary while FFX runs. Collisions with FFX paths are unlikely because the path roots differ.
- **Open hook.** The hook is on `Phyre::PSerialization::PStreamFile` ctor (`0x207BC0`) (`fileloader.cs:57,296-345`).
  1. It normalises the game path (`normalize_path`, `:142-202`): strips `../../..`, maps `host0:` to `/ffx_ps2`, maps `GCM` to `D3D11`, and maps `.cgfx.phyre` to `.fx.phyre`.
  2. It upper-cases the path and looks it up in the index.
  3. On a hit it puts a Win32 handle from `CreateFileW` in the PStreamFile (`handle_vbf = null`), opened read-only with `FILE_SHARE_READ` (`:311-334`).
  4. If the open fails, it falls back to the VBF (`:336-341`).
- **Stream prefix.** Fahrenheit also hooks the stream-prefix setters, so game paths are `/ffx_data/...` and `/ffx_ps2/...` (`:204-226`). It also hooks `fiosUnifyFilename` and `ClusterManager::loadPCluster`/`getPClusterByName` so they normalise the same way (`:244-278`).
- **Path limit.** Paths are limited to 0x100 bytes, "a game invariant" (`:291-293`).

**Phyre files vs PS2 files.** Both go through the same hook.
- `.phyre` assets: `ClusterManager_loadPCluster` → `Phyre_PCluster_LoadAssetFile` → `PStreamReaderFile` → the PStreamFile ctor [decomp: `ffx.sym:10070,522,5872`].
- PS2 files: `sceOpen` (`0x0062fa30`) builds `"/ffx_ps2/" + name`, unifies it, calls `fiosIsFileExist`, then `fiosOpen` → PStreamFile ctor [decomp: `ffx.sym:8377,9623`; `crates/ffx-system/src/m5a_platform.rs:302-327`].
- The decomp names `PStreamFileWin32_openFile` (called from the ctor) as "the only way the game opens archive files" (`ffx-decomp/docs/hooking.md:62`).

**Size limits (PS2 CD-numbered files).** The PS2 layer allocates buffers from size tables. A replacement whose size differs from the original "will crash" (`src/runtime/cd.cs:8-23`). `FhCdInterfaceModule` handles this:
- It hooks `CDfileSize_PC` and temporarily sets `cd.module = 0`. That makes the game skip the `sizetbl` lookup (`CDfileSize_CD_EX`) and take the size from `sceOpen`/`sceFileSize` instead, which go through the EFL (`cd.cs:72-105`).
- It forces `check_ex_file_size` to return 0 (`cd.cs:49-70`).
- So in the normal PC path, larger or smaller PS2 replacements get their real size, rounded up to 16 bytes.
- `CDfileSize_CD` (sector table `cdrom.mdg`) is **not** hooked by Fahrenheit. In PC mode `Cd_QueueRead` uses `CDfileSize_PC`, and `CDfileSize_CD` only for Eternal Calm files and HDD mode [decomp: `ffx.sym:12545,23841`]. ffx-decomp's own harness wraps both (`docs/hooking.md:83-93`).

**Size of Phyre files.** `PStreamFile::getFileSize` uses `GetFileSizeEx` on OS handles [decomp: `docs/formats/system.md:128`]. No table is involved. Other limits I have **not** checked: fixed pools or VRAM budgets inside loaders, and the 32-bit address space (Fahrenheit only probes the LAA flag, `bootloader.cs:145-149`).

**Format.** The EFL passes raw bytes through. Phyre replacements must be the PC/D3D11 Phyre container the game expects. Fahrenheit does no conversion.

**Runtime toggle: startup only.**
- The index is built once in `fhr`'s `init`. The remark says "any files subsequently added to the EFL directories will not be available in this game session" (`fileloader.cs:86-91`).
- There is no public API to add, remove or disable entries. `_index` is private (`:42`).
- [inference] Each indexed file is re-opened with `CreateFileW` on every game open (`:325`). Changing the *contents* of an already-indexed file between loads would therefore take effect on the next open. Loaded clusters are ref-counted and cached by name hash (`ffx.sym:10070`), so assets already in memory do not change until the game releases and reloads them. Not tested.

**Logging.**
- Every indexed file is logged ("Mod X replaces file ...") (`:81`).
- An override between mods logs a warning (`:77`).
- VBF misses log an error (`:280-289`).
- Fahrenheit has no "log every open" option. ffx-decomp's harness has `--log-opens` (`docs/hooking.md:79-80`).

## 2. Choosing files per setting at startup

**No supported API exists.** The EFL indexes *every* file under every loaded mod's `efl/` dir unconditionally (`fileloader.cs:71-100`).

Timing makes it hard to decide from settings:
1. Module constructors run first (`FhApi.Mods = new()` → `FhMods` ctor; `api.cs:20`, `mods.cs:21-47`).
2. `boot()` then runs `Localization.initialize` → `Settings.initialize` (loads settings JSON from disk) → `Mods.initialize` (`bootloader.cs:58-66`; `settings.cs:102-113,142-152`).
3. `init()` calls follow load order, and `fhr` is always first (`bootloader.cs:95-98`, `mods.cs:52-74`).

So the EFL index is built before your mod's `init` runs. Your setting values are loaded before that, but no callback fires (`FhSetting<T>.set/load` are internal or sealed, `settings.cs:246-260`).

Options, in decreasing order of how well they are supported:

1. **Split into file-only mods.** A mod with a manifest and an `efl/` dir but no DLL is supported (`bootloader.cs:232-236`). Examples: `demaster.chr`, `demaster.map`, `demaster.ps2`. The user enables each one through `mods/loadorder`. This only switches at startup and only per pack, with no in-game UI.
2. **Arrange the `efl/` dir in your module constructor.** The constructor runs before `fhr.init` builds the index. It may not touch `FhApi` or call handles (wiki `Startup.md`), but plain .NET file I/O is fine.
   - Read your own settings file directly. Its path is `config/global/<mod_id>/<Module.FullName>.json` (`paths.cs:113-114,127`). The Fahrenheit root is the parent of `AppContext.BaseDirectory` (`paths.cs:50-51`).
   - Then add or remove files under `mods/<id>/efl/x/`, for example by moving them from `resources/`.
   - [inference] This works by construction. It is not a documented contract, and the wiki says users must not modify mod files (which is a user rule). Whether `GetFiles(..., AllDirectories)` follows junctions or symlinks was not checked.
3. **Hook the PStreamFile ctor yourself.**
   - `FhCallLocation(0x207BC0, 0x490D40)` and `FhCallHandle<T>` are public (`handles.cs:146,255-297`). ffx-debug-revival builds handles this way (`src/Native/Game.cs:62`).
   - Hooks queued in `init` are applied in `chain_finalize`, latest-loaded first. Your hook therefore runs *before* the EFL hook, sees the raw path, and can rewrite it before chaining (`handles.cs:407-424,514-536`). That allows per-file decisions at runtime.
   - **But** this call is marked restricted: "We neither permit nor support any other mod tampering with them" (`src/core/call.cs:30-42,410-421`).
4. **Redirect names at game level**, as debug-revival's `StandInMaps` does. It hooks `PcFileName` (the CD-number → name function) and `TkCdReadWithMallocWithMessage` (`ffx-debug-revival/src/Features/StandInMaps.cs:59-60,110-121`). This can be decided per call at runtime, but the target path must pass the game's existence checks (see §4).

Settings API, for completeness:
- Subclass `FhSettingProvider<TModule>` and create it in the module constructor (`settings.cs:29-39,83-97`).
- Types: `FhSettingToggle`, `FhSettingNumber<T>`, `FhSettingText`, `FhSettingsCategory` (`settings.cs:415,369,342,278`).
- Read a value with `setting.get()` (`:246`).

## 3. Rendering hooks, ImGui and settings storage

**D3D11 and render path.** There is no public render API beyond ImGui.
- `FhPlatformBindingModule` hooks `D3D11.dll!D3D11CreateDeviceAndSwapChain` and hands the device, context, swapchain and HWND only to the internal `IFhPlatformUser` (`src/runtime/platform.cs:16-27,57-136`; `call.cs:343-363`).
- `FhImguiModule` hooks `IDXGISwapChain::Present` (vtbl 8) and `ResizeBuffers` (vtbl 13) at runtime (`imgui.cs:136-137,219-262`; `call.cs:372-396`).
- All of these call handles are `internal` and restricted.
- No shader-replacement or draw-call hooks exist.
- A mod could hook D3D11 creation itself through a public `FhCallLocation("D3D11.dll", "D3D11CreateDeviceAndSwapChain")` (`handles.cs:158`). That is unsupported [inference], and Special K needs `--extdbg` to coexist (wiki `Compatibility-guide.md`).
- Engine-level render functions would have to come from the decomp (`ffx-decomp/docs/renderer.md`, `render.md`). Not reviewed here.

**ImGui.**
- Override `FhModule.render_imgui()` (`src/core/module.cs:114`). It is called every frame inside the Present hook, on the render thread, for every module (`imgui.cs:240-262`). It is called without a try/catch, so debug-revival wraps its own code (`ffx-debug-revival/src/DebugRevivalModule.cs:96-109`).
- Fonts and window flags come from `FhApi.Gui` (`src/core/gui/gui.cs:11-18,157`).
- Images: `FhApi.Resources.load_texture(new FhTexture(path, type))`. Types are `DISK` (WIC), `GAME`, and `GAME_PHYRE_2D/3D/CUBE` (a VBF path loaded through the cluster manager) (`src/core/resources.cs:11-18,42,159-180`; `src/runtime/resources.cs:43-80`). This is useful for an in-game before/after preview.
- Input is swallowed while ImGui wants it (`imgui.cs:184-200`).

**Settings storage.**
- Format: JSON, indented, one object of `id: value` per module (`settings.cs:155-178,254-257`).
- Location: `<Fahrenheit>/config/global/<mod_id>/<Module.FullName>.json` (`paths.cs:113-114`). A `config/local/default/...` path exists but settings do not use it (`paths.cs:116-117`).
- Loaded at boot. Saved when the user closes the F7 mod-config UI (`src/runtime/gui/settings.cs:18-33`).
- Display names come from localization keys `{id}.name` / `{id}.desc` in `lang/<Module.FullName>/` (`settings.cs:194-201`; wiki `Settings.md`).
- Per-module persistent state: a `global_state_file` stream passed to `init` (`module.cs:97`), stored in `state/global/<mod>/<module>` (`paths.cs:110-111`).
- **ffx-debug-revival uses none of this.** It has no `FhSettingProvider`. Its toggles live in memory in its own F9 ImGui window. Its `init` installs hooks and subscribes to `FhApi.Events.Common.GameLoop.PreUpdate/PostUpdate` (`src/DebugRevivalModule.cs:26-66`). Generated files go to `mod_context.Paths.ResourcesDir` (`:30-32`).

## 4. A file the game never had

- **Opening works.** The EFL checks its index *before* the VBF and never asks the VBF on a hit (`fileloader.cs:306-334`). Any code that opens the new path through the PStreamFile ctor gets the file, including `ClusterManager_loadPCluster` and a mod's own `FhVbfFileScope(path)` (`src/core/vbf.cs:88-152`).
- **Existence checks fail.** `fiosIsFileExist` → `PhyreFIOS::isFileExist` → `File_Probe` (`0x00607e00`) asks `FileSys_Exists` (VBF MD5 lookup only). Otherwise it tries `CreateFileW` on the raw unified path [decomp: `ffx.sym:7778,8054,9621,11221`; `crates/ffx-system/src/files.rs:98-106`, `m5a_df.rs:135-171`]. Fahrenheit does not hook `File_Probe`, `FileSys_Exists` or `fiosIsFileExist`. I grepped `src/` and found these addresses only in generated stubs or reject lists.
  - [inference] Under Fahrenheit's `/` prefix the raw path is `/ffx_ps2/...`, which Win32 resolves against the drive root. So an EFL-only new file is reported missing.
- **Callers that check first** and therefore refuse new EFL-only files [decomp]:
  - `sceOpen` returns 0 (`m5a_platform.rs:323-325`).
  - `PhyreScene_fieldMapLoadAsync` (`ffx.sym:9235`).
  - `ClusterManager_createPreallocateDataList` (`ffx.sym:8644`).
  - ffx-debug-revival relies on this same check to detect missing maps (`src/Features/FieldTools.cs:178-183`, `JapaneseText.cs:87-89`).
- **Unreferenced paths.** Even where loading would work, the game only loads paths it computes from its own data. A new file needs something that references it: an edited table, or a hook.
- **CD-numbered PS2 files** cannot gain new numbers without editing `cdrom.mdg`/`cdrom.fnd`/`sizetbl`.
- **Practical conclusion [inference].** Replacing existing paths is safe. Adding new paths works only through cluster loads or your own hooks. A field map or PS2 file at a new path also needs a hook on `fiosIsFileExist`/`File_Probe`, which is not a restricted Fahrenheit call (not in `call.cs`), or VBF repacking (`ffx-decomp/docs/formats/vbf.md:77-96`).

## 5. Other notes for an asset mod

- **Load order.**
  - `mods/loadorder` holds one mod id per line, and `fhr` is always prepended (`bootloader.cs:95-98`). Without the file no mods load (`:100-104`).
  - EFL conflicts: the later mod in load order wins the whole file, with a warning (`fileloader.cs:76-81`). Files are not merged.
  - Manifest `Dependencies` is declared but nothing in `src/` enforces it (`module.cs:65`).
- **Other loaders.** Fahrenheit's EFL overrides ffgriever's standalone EFL (wiki `Compatibility-guide.md`). Other code mods that hook the same restricted functions are unsupported.
- **SDK packaging.** `Fahrenheit.Sdk` copies `efl/x/**`, `efl/x2/**`, `resources/**` and `lang/**/*.json` from the project into the build output (`src/sdk/Sdk/Sdk.props:33-47`). Current source references package `Fahrenheit 1.0.0-alpha13` (`:27`). debug-revival targets `Fahrenheit.Sdk/1.0.0-alpha12`. Fahrenheit ≥ alpha12 needs the Oct 1 2026 Steam build (wiki `Compatibility-guide.md`).
- **Manifest.** `{id}.manifest.json` is generated from the csproj (`AssemblyName` = id; `FhManifestFlags` is required). File-only mods write it by hand (wiki `Manifests.md`).
- **ffx-debug-revival layout.**
  - Project: `src/yuta.debugrevival.csproj` (SDK project, embedded resources).
  - `Directory.Build.props:26-27` puts output in `artifacts/deploy/{rel,dbg}/mods/<id>/`. That folder holds the dll, deps.json, manifest.json, pdb and runtimeconfig.json.
  - `tools/package/package.cs:24-31` zips it as `mods/yuta.debugrevival/**` plus `README.md`, `LICENSE` and `THIRD-PARTY-NOTICES`, so the zip unpacks into the Fahrenheit folder.
  - `.github/workflows/release.yml` builds on a `v*` tag, packages `yuta.debugrevival-<tag>.zip`, and marks tags containing `-` as prereleases.
  - Install steps: unpack, then add the id to `mods/loadorder` (`README.md` Install).
- **Copyright.** Fahrenheit's own Phyre loader exists so mods need not ship copyrighted assets (`src/runtime/peloader.cs:12-15`). Converted PS2 assets therefore have to be produced on the user's machine and written into `mods/<id>/efl/x/...` *before* the game starts, because the index is built at startup.
- **Finding paths.** Use VBF paths as stored (`ffx_data/gamedata/ps3data/...`, `ffx_ps2/ffx/master/...`) (`ffx-decomp/docs/formats/vbf.md:60-67`). An old doc comment (alpha12) showed an extra `FFX_Data\` segment, `efl\x\FFX_Data\ffx_ps2\...`. The current wiki and the code's key derivation (no prefix besides `/`) say there is no such segment. Confirm with the "replaces file" log lines and a test load.
- [inference] Fahrenheit's `normalize_path` does not convert `\` to `/` in the game's path before the index lookup (`fileloader.cs:142-202`). A game path containing backslashes would miss the index. Not observed in practice.
