Modern Minimap
=============

A Fahrenheit mod for FINAL FANTASY X HD Remaster (Steam, Windows), one of
the FFX mods (https://github.com/yutamago/ffx-mods): yuta.modern-minimap.

What it changes
---------------

The field minimap becomes round, always centred on you and turned with
the camera, so that up on the map is where the camera looks.
- your character is an arrow showing where they face;
- the walkable area sits over a grid fixed to the world, in the game's
  minimap colours on a small CRT screen;
- the game's markers keep their shapes and colours; those beyond the edge
  sit on the rim and fade in as you come nearer; where you are headed
  stays on the rim;
- it shows and hides when the game's minimap would, and the fog of war
  still clears as you walk.

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
   Fahrenheit's "bin" folder). It holds two folders: mods\yuta.modern-minimap\
   (this mod) and mods\yuta.ffx-mods\ (the shared library every mod of the
   family needs; unpacking another mod's zip over it is fine).
2. Double-click mods\yuta.modern-minimap\install.cmd. It adds yuta.ffx-mods
   and yuta.modern-minimap to mods\loadorder. This mod builds no files: it
   needs no PS2 disc. (Or add the two lines to mods\loadorder yourself:
   first yuta.ffx-mods, then yuta.modern-minimap.)
3. Start the game through Fahrenheit (bin\fhstage0.exe with the path of
   FFX.exe).

In the game, F8 opens one window for every mod of the family: which of its
settings are on. It opens by itself when something is wrong.

Settings
--------

settings.toml in the mod folder, with a comment above each setting. After
changing one, restart the game.

  style   "crt" (the default), "plain" (no CRT screen) or "game" (the
          game's own minimap)
  size    "small", "medium" (the default) or "large"
  corner  "top_right" (the default), "top_left", "bottom_right" or
          "bottom_left"

Uninstall
---------

Delete mods\yuta.modern-minimap\ and its line in mods\loadorder. When no
other mod of the family is left, also delete mods\yuta.ffx-mods\ and its
line.

Files
-----

  yuta.modern-minimap.dll, .manifest.json, ...   the mod
  settings.toml   its settings
  install.cmd, install.ps1   the installer (load order)
  LICENSES.txt    licences of this mod and of the code it includes
  README.txt      this file
  ..\yuta.ffx-mods\   the shared library (settings, the F8 window)
