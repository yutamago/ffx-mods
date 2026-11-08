PS2 music engine
================

A Fahrenheit mod for FINAL FANTASY X HD Remaster (Steam, Windows), one of
the FFX mods (https://github.com/yutamago/ffx-mods): yuta.ps2-music-engine.

What it changes
---------------

The original soundtrack played the PS2 way: the PS2 game's own sound
driver (a port of its iopsound.irx on a model of the PS2's SPU2 sound
chip) runs inside the game and plays the music from the disc's sequences
and instruments, driven by the game's music commands (fades, pauses,
tempo, as the scripts ask). It replaces what the game plays while its
soundtrack option (Config > Soundtrack) is "Original": choose "Original"
there. The three tracks with streamed parts, and anything the engine can't
play, stay the PC's recordings of the original soundtrack.

PS2 disc: needed (read once, by install.cmd)
Size: about 60 MB (the disc's music sequences, instrument banks and sound driver tables in music\)
Dependencies: Needs the shared library yuta.ffx-mods (in this zip). Works alone and
with any other mod of the family.

NO GAME DATA IS INCLUDED
------------------------

This zip has program code only. The PS2 files are built on your own
computer by install.cmd (with demaster.exe, the converter in this folder),
from YOUR OWN Final Fantasy X PS2 disc and your own Steam installation. The
files it writes are game data: don't share them.

Requirements
------------

- FINAL FANTASY X/X-2 HD Remaster on Steam (Windows).
- Fahrenheit 1.0.0-alpha12 or later:
  https://github.com/fahrenheit-crew/fahrenheit/releases
  installed as its guide says:
  https://github.com/fahrenheit-crew/fahrenheit/wiki/Installation-guide
- Your own Final Fantasy X PS2 disc, any region: an image of it (.iso),
  or the disc itself in a disc drive. install.cmd reads it once; after that
  the mod needs neither the disc nor the image.

Install
-------

1. Unpack this zip into your Fahrenheit folder (the folder with
   Fahrenheit's "bin" folder). It holds two folders: mods\yuta.ps2-music-engine\ (this
   mod) and mods\yuta.ffx-mods\ (the shared library every mod of the
   family needs; unpacking another mod's zip over it is fine).
2. Double-click mods\yuta.ps2-music-engine\install.cmd. It adds yuta.ffx-mods and
   yuta.ps2-music-engine to mods\loadorder, asks where your PS2 disc is
   (a disc drive with the disc in it, or an image file), finds the game
   (or asks for its folder), and builds the PS2 files into the mod folder.
   It saves your choices in settings.toml, so running it again asks less.
   From a terminal: install.cmd -Iso 'D:\PS2\FFX.iso', or -Drive E.
3. Start the game through Fahrenheit (bin\fhstage0.exe with the path of
   FFX.exe).

In the game, F8 opens one window for every mod of the family: whether each
mod's files match its settings and which of its code patches are on. It
opens by itself when something is wrong.

Settings
--------

settings.toml in the mod folder, with a comment above each setting. After
changing one: run install.cmd again (it redoes only what changed and
removes the files of whatever you switched back to HD), then restart the
game. After updating the mod, run install.cmd again too.

Uninstall
---------

Delete mods\yuta.ps2-music-engine\ and its line in mods\loadorder. That removes the PS2 files
it built too (they are in its folder). When no other mod of the
family is left, also delete mods\yuta.ffx-mods\ and its line.

Files
-----

  yuta.ps2-music-engine.dll, .manifest.json, ...   the mod
  settings.toml   its settings
  install.cmd, install.ps1   the installer (load order, PS2 files)
  demaster.exe    the converter (builds the PS2 files from your disc)
  demaster_live.dll   the PS2 sound driver that plays in the game
  LICENSES.txt    licences of this mod and of the code it includes
  README.txt      this file
  ..\yuta.ffx-mods\   the shared library (settings, build status, the F8 window)

Made by install.cmd: efl\ (the PS2 files), build\ (what was built, for F8
and the next build), music\ (the disc's music files).
