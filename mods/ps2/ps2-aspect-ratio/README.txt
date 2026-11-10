PS2 aspect ratio
================

A Fahrenheit mod for FINAL FANTASY X HD Remaster (Steam, Windows), one of
the FFX mods (https://github.com/yutamago/ffx-mods): yuta.ps2-aspect-ratio.

What it changes
---------------

The PS2's 4:3 picture with black bars at the sides, and the PS2's
camera projection (the HD widened the view for 16:9). The menus, windows
and battle HUD that the game places in PS2 coordinates land where the PS2
had them; the opening's staff credits and other event pictures get the
PS2 size; the sphere grid gets a 4:3 camera; the Flash screens (Esc menu,
settings, save/load) are drawn in the 4:3 picture. The HD movies are not
reshaped by this mod.

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
   Fahrenheit's "bin" folder). It holds two folders: mods\yuta.ps2-aspect-ratio\ (this
   mod) and mods\yuta.ffx-mods\ (the shared library every mod of the
   family needs; unpacking another mod's zip over it is fine).
2. Double-click mods\yuta.ps2-aspect-ratio\install.cmd. It adds yuta.ffx-mods and
   yuta.ps2-aspect-ratio to mods\loadorder. This mod builds no files: it needs
   no PS2 disc. (Or add the two lines to mods\loadorder yourself: first
   yuta.ffx-mods, then yuta.ps2-aspect-ratio.)
3. Start the game through Fahrenheit (bin\fhstage0.exe with the path of
   FFX.exe).

In the game, F8 opens one window for every mod of the family: whether each
mod's files match its settings and which of its code patches are on. It
opens by itself when something is wrong.

Settings
--------

settings.toml in the mod folder, with a comment above each setting.
install.cmd makes it from settings.default.toml (the defaults) when there
is none; until then the mod uses the defaults. After changing one, restart
the game.

Updating
--------

Unpack the newer zip over your install and run install.cmd again. The zip
holds only settings.default.toml, never settings.toml, so your settings
stay; settings new in that version are added to your settings.toml with
their comments and defaults.

Switching it off
----------------

mods\yuta.ps2-aspect-ratio\install.cmd -Disable ps2-aspect-ratio takes the
mod out of mods\loadorder and keeps it out whenever install.cmd runs again
(it is listed in mods\ffx-mods-off.txt); install.cmd -Enable
ps2-aspect-ratio switches it on again. Its folder stays. Deleting its line
in mods\loadorder by hand does the same.

Uninstall
---------

Delete mods\yuta.ps2-aspect-ratio\ and its line in mods\loadorder. When no other mod of the
family is left, also delete mods\yuta.ffx-mods\ and its line.

Files
-----

  yuta.ps2-aspect-ratio.dll, .manifest.json, ...   the mod
  settings.default.toml   the default settings (an update replaces it)
  settings.toml   your settings (made by install.cmd from settings.default.toml)
  install.cmd, install.ps1   the installer (load order)
  LICENSES.txt    licences of this mod and of the code it includes
  README.txt      this file
  ..\yuta.ffx-mods\   the shared library (settings, build status, the F8 window)
