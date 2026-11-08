FPS unlock
==========

A Fahrenheit mod for FINAL FANTASY X HD Remaster (Steam, Windows), one of
the FFX mods (https://github.com/yutamago/ffx-mods): yuta.fps-unlock.

What it changes
---------------

Not a part of the PS2 game: the fixed 30 fps limit goes, without speeding
the game up.
- the game draws at your monitor's refresh rate, at a rate you choose, or
  uncapped, with vsync on or off;
- the world keeps its 30 ticks a second; the camera and the characters
  (their bones, cloth and hair) are drawn between ticks;
- a time scale gives slow motion and fast-forward (x0.05 to x100) without
  drawing more frames (the water, the scrolling skies and other animated
  surfaces follow it too); voice-synced scenes, movies and loads stay at
  their own speed;
- a small counter in the top-right corner shows the frame rate, the
  world's ticks a second (about 30 at x1), vsync and the time scale.

Effects, the 2D layers and the round shadows still move at 30 Hz (the
effects at the game's speed, drawn on every frame). Fast-forward costs
CPU: every tick is simulated, and when the CPU cannot keep up the counter
shows the speed reached ("reached x12 of x18").

PS2 disc: not needed
Size: under 1 MB (code only)
Dependencies: Needs the shared library yuta.ffx-mods (in this zip). Works alone and
with any other mod of the family.

Requirements
------------

- FINAL FANTASY X/X-2 HD Remaster on Steam (Windows).
- Fahrenheit 1.0.0-alpha12 or later:
  https://github.com/fahrenheit-crew/fahrenheit/releases
  installed as its guide says:
  https://github.com/fahrenheit-crew/fahrenheit/wiki/Installation-guide

Install
-------

1. Unpack this zip into your Fahrenheit folder (the folder with
   Fahrenheit's "bin" folder). It holds two folders: mods\yuta.fps-unlock\
   (this mod) and mods\yuta.ffx-mods\ (the shared library every mod of the
   family needs; unpacking another mod's zip over it is fine).
2. Double-click mods\yuta.fps-unlock\install.cmd. It adds yuta.ffx-mods
   and yuta.fps-unlock to mods\loadorder. This mod builds no files: it
   needs no PS2 disc. (Or add the two lines to mods\loadorder yourself:
   first yuta.ffx-mods, then yuta.fps-unlock.)
3. Start the game through Fahrenheit (bin\fhstage0.exe with the path of
   FFX.exe).

Keys
----

  F9              its settings (in the F8 window of the mod family);
                  F9 or F8 closes it
  Alt+F9          vsync on / off
  Ctrl+F9         fast-forward (x10) / x1
  Shift+F9        slow motion (x0.25) / x1
  Ctrl+Shift+F9   back to x1
  Ctrl+PgUp/PgDn  the time scale a step up or down (x0.05 ... x100)

A fast-forward scale is saved only once the game has run at it for 5 s,
and a saved one starts at x1 and doubles every second up to it, so a
scale the game cannot take never crashes the next start.

Settings
--------

settings.toml in the mod folder, with a comment above each setting. Every
setting can also be changed in the game (F9): it applies at once and is
written back into settings.toml (only the lines that changed). A change
made in the file while the game runs is read at the next start.

  frames.enabled        true; false gives the game its own timing back
  frames.target         "monitor" (the default), "game" (30 fps),
                        "fixed" (frames.fixed_fps) or "unlimited"
  frames.vsync          "game" (as the game has it), "on" or "off"
  frames.overlay        the fps counter
  frames.speed.*        the time scale and the Ctrl/Shift+F9 scales;
                        scene_time = false leaves the water and the skies
                        in real time, as in the game
  frames.interpolate.*  what is drawn between ticks
  frames.effects, frames.between_ticks, frames.cuts.*, frames.hold.*,
  frames.budget.*       finer settings; the window's "Advanced" part

Uninstall
---------

Delete mods\yuta.fps-unlock\ and its line in mods\loadorder. When no other
mod of the family is left, also delete mods\yuta.ffx-mods\ and its line.

Files
-----

  yuta.fps-unlock.dll, .manifest.json, ...   the mod
  settings.toml   its settings
  install.cmd, install.ps1   the installer (load order)
  LICENSES.txt    licences of this mod and of the code it includes
  README.txt      this file
  ..\yuta.ffx-mods\   the shared library (settings, the F8 window)
  The log: logs\<start time>\Yuta.FfxMods.FpsUnlock.FpsUnlockModule.log
  in the Fahrenheit folder.
