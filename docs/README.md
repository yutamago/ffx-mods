# Docs

The research and design docs of every mod: how each change works, the
evidence behind it and how it was checked.

## By mod

| Mod | Docs |
|---|---|
| PS2 music engine, PS2 soundtrack | [mods/music.md](mods/music.md); the soundtrack option: [mods/patches.md](mods/patches.md) (`audio.soundtrack`) |
| PS2 movies | [mods/movies.md](mods/movies.md) |
| PS2 characters | [mods/characters.md](mods/characters.md); texture filter and quality: [mods/render.md](mods/render.md) |
| PS2 maps | [mods/maps.md](mods/maps.md); the 2D backgrounds: [mods/screen.md](mods/screen.md) |
| PS2 effects | [mods/effects.md](mods/effects.md) |
| PS2 lighting | [mods/render.md](mods/render.md) (`render.lighting`) |
| No HD post-effects | [mods/render.md](mods/render.md) (`render.glow`, `render.shadows`) |
| PS2 interface | [mods/interface.md](mods/interface.md), [mods/font.md](mods/font.md); splash videos: [mods/patches.md](mods/patches.md); still open: [plans/interface-remaining.md](plans/interface-remaining.md) |
| PS2 aspect ratio | [mods/screen.md](mods/screen.md) |
| PS2 game feel | [mods/patches.md](mods/patches.md), [mods/timing.md](mods/timing.md) |
| Bug fixes | [mods/patches.md](mods/patches.md) |
| FPS unlock | [mods/fps-unlock.md](mods/fps-unlock.md) |
| Modern Minimap | [mods/modern-minimap.md](mods/modern-minimap.md) |
| Debug Revival | [mods/debug-revival/](mods/debug-revival/README.md) |

## For all mods

- [config.md](config.md): every setting, and which mod has it.
- [differences.md](differences.md): every difference between the PS2
  original and the HD Remaster (the "rows" the other docs refer to).
- [ffx-rust-findings.md](ffx-rust-findings.md): FFX.exe and PS2 facts
  found here, to port into ffx-rust.

## Development

- [dev/ci.md](dev/ci.md): CI and releases.
- [dev/disc.md](dev/disc.md): how the PS2 disc stores its files.
- [dev/compare.md](dev/compare.md), [dev/debugger.md](dev/debugger.md):
  PS2-vs-PC comparisons and the PS2 debugger (the tools are in ffx-demaster).

## Plans and research

- [plans/plan.md](plans/plan.md): plan and status, by row of differences.md.
- [plans/interface-remaining.md](plans/interface-remaining.md): the
  interface items still open.
- [research/](research/): the first surveys (differences catalogue,
  Fahrenheit, HD-vs-PS2 assets, PS2 textures).
