PS2 soundtrack
==============

A Fahrenheit mod for FINAL FANTASY X HD Remaster (Steam, Windows), one of
the FFX mods (https://github.com/yutamago/ffx-mods): yuta.ps2-soundtrack.

What it changes
---------------

The original soundtrack as the PS2 played it: install.cmd renders every
track again from your disc's music sequences and instruments with a port of
the PS2 game's own sound driver (on a model of the PS2's SPU2 sound chip),
and the result takes the place of the PC's recordings in the game's
"Original" soundtrack. New games start with the soundtrack option on
"Original" (the PC starts on the arranged one); loaded saves keep their
choice and the game's Config menu still switches it.

With PS2 music engine installed too, that engine plays the music live and
this mod's bank is what it falls back on (the three tracks with streamed
parts).

PS2 disc: needed (read once, by install.cmd)
Size: about 650 MB
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
   Fahrenheit's "bin" folder). It holds two folders: mods\yuta.ps2-soundtrack\ (this
   mod) and mods\yuta.ffx-mods\ (the shared library every mod of the
   family needs; unpacking another mod's zip over it is fine).
2. Double-click mods\yuta.ps2-soundtrack\install.cmd. It adds yuta.ffx-mods and
   yuta.ps2-soundtrack to mods\loadorder, asks where your PS2 disc is
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

Delete mods\yuta.ps2-soundtrack\ and its line in mods\loadorder. That removes the PS2 files
it built too (they are in its folder). When no other mod of the
family is left, also delete mods\yuta.ffx-mods\ and its line.

Files
-----

  yuta.ps2-soundtrack.dll, .manifest.json, ...   the mod
  settings.toml   its settings
  install.cmd, install.ps1   the installer (load order, PS2 files)
  demaster.exe    the converter (builds the PS2 files from your disc)
  LICENSES.txt    licences of this mod and of the code it includes
  README.txt      this file
  ..\yuta.ffx-mods\   the shared library (settings, build status, the F8 window)

Made by install.cmd: efl\ (the PS2 files), build\ (what was built, for F8
and the next build).