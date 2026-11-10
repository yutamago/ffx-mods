Debug Revival
=============

A Fahrenheit mod for FINAL FANTASY X HD Remaster (Steam, Windows), one of
the FFX mods (https://github.com/yutamago/ffx-mods): yuta.debug-revival.

What it changes
---------------

It turns back on the debug tools the developers left in the game. Press F9
in the game to open its window:

- Scripts: the event-script debugger. Every running script with its code,
  where it stands, its values and variables; pause, step and breakpoints.
- PS2 menu: the developers' own debug menu from the PS2 version, worked
  with the mouse.
- Panels: the PC version's ten developer panels.
- Battle: the developers' battle switches and cheats, any battle on
  demand, and HP, MP, overdrive and turn order of everyone in a battle.
- Field: walk through walls, game speed, random encounters, a free camera,
  jumps to any map (unfinished areas included) and the developers' ~110
  saves for every part of the story.
- Log: the developers' debug messages.

The "Debug flag" box at the top switches the game into the developers'
debug mode while it runs.

PS2 disc: not needed
Size: under 1 MB (code only). The first time the game runs with it, it
rebuilds two unfinished rooms of Bevelle from your game files (about
35 MB, in its efl folder); they show from the next start.
Dependencies: Needs the shared library yuta.ffx-mods (in this zip). Works alone and
with any other mod of the family.

Back up your saves first (Documents\Square Enix\FINAL FANTASY X&X-2 HD
Remaster): the cheats and two of the panels change what your next save
writes. Jumping to maps out of story order can break your progress. The
mod refuses the maps that would crash the game and blocks the debug items
that would hang it, clear the story flags or reset your Steam achievements.

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
   Fahrenheit's "bin" folder). It holds two folders: mods\yuta.debug-revival\
   (this mod) and mods\yuta.ffx-mods\ (the shared library every mod of the
   family needs; unpacking another mod's zip over it is fine).
2. Double-click mods\yuta.debug-revival\install.cmd. It adds yuta.ffx-mods
   and yuta.debug-revival to mods\loadorder. This mod needs no PS2 disc.
   (Or add the two lines to mods\loadorder yourself: first yuta.ffx-mods,
   then yuta.debug-revival.)
3. Start the game through Fahrenheit (bin\fhstage0.exe with the path of
   FFX.exe) and press F9.

Had the older Debug Revival (mods\yuta.debugrevival\)? Delete that folder
and its line in mods\loadorder, or both copies run at once.

In the game, F8 opens one window for every mod of the family; F9 opens
Debug Revival's own.

Settings
--------

None: everything is in its F9 window, and nothing there is saved.

Updating
--------

Unpack the newer zip over your install and run install.cmd again.

Switching it off
----------------

mods\yuta.debug-revival\install.cmd -Disable debug-revival takes the mod
out of mods\loadorder and keeps it out whenever install.cmd runs again (it
is listed in mods\ffx-mods-off.txt); install.cmd -Enable debug-revival
switches it on again. Its folder stays. Deleting its line in mods\loadorder
by hand does the same.

Uninstall
---------

Delete mods\yuta.debug-revival\ and its line in mods\loadorder. When no
other mod of the family is left, also delete mods\yuta.ffx-mods\ and its
line.

Files
-----

  yuta.debug-revival.dll, .manifest.json, ...   the mod
  efl\x\   the two rebuilt Bevelle rooms (made by the mod in the game)
  install.cmd, install.ps1   the installer (load order)
  LICENSES.txt    licences of this mod and of the code it includes
  README.txt      this file
  ..\yuta.ffx-mods\   the shared library (the F8 window)
