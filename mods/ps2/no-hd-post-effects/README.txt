No HD post-effects
==================

A Fahrenheit mod for FINAL FANTASY X HD Remaster (Steam, Windows), one of
the FFX mods (https://github.com/yutamago/ffx-mods): yuta.no-hd-post-effects.

What it changes
---------------

Two things the HD Remaster draws over the picture that the PS2 didn't:
- glow: the HD's blurred bright parts added over the whole picture (a
  bloom); off, the picture is as sharp and dark as the PS2's;
- shadows: the HD's shadow maps (every battle and many field maps); off,
  every character has one soft ground shadow under it, as on the PS2.
Each is a setting.

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
   Fahrenheit's "bin" folder). It holds two folders: mods\yuta.no-hd-post-effects\ (this
   mod) and mods\yuta.ffx-mods\ (the shared library every mod of the
   family needs; unpacking another mod's zip over it is fine).
2. Double-click mods\yuta.no-hd-post-effects\install.cmd. It adds yuta.ffx-mods and
   yuta.no-hd-post-effects to mods\loadorder. This mod builds no files: it needs
   no PS2 disc. (Or add the two lines to mods\loadorder yourself: first
   yuta.ffx-mods, then yuta.no-hd-post-effects.)
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

mods\yuta.no-hd-post-effects\install.cmd -Disable no-hd-post-effects takes
the mod out of mods\loadorder and keeps it out whenever install.cmd runs
again (it is listed in mods\ffx-mods-off.txt); install.cmd -Enable
no-hd-post-effects switches it on again. Its folder stays. Deleting its
line in mods\loadorder by hand does the same.

Uninstall
---------

Delete mods\yuta.no-hd-post-effects\ and its line in mods\loadorder. When no other mod of the
family is left, also delete mods\yuta.ffx-mods\ and its line.

Files
-----

  yuta.no-hd-post-effects.dll, .manifest.json, ...   the mod
  settings.default.toml   the default settings (an update replaces it)
  settings.toml   your settings (made by install.cmd from settings.default.toml)
  install.cmd, install.ps1   the installer (load order)
  LICENSES.txt    licences of this mod and of the code it includes
  README.txt      this file
  ..\yuta.ffx-mods\   the shared library (settings, build status, the F8 window)
