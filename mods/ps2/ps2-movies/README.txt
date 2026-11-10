PS2 movies
==========

A Fahrenheit mod for FINAL FANTASY X HD Remaster (Steam, Windows), one of
the FFX mods (https://github.com/yutamago/ffx-mods): yuta.ps2-movies.

What it changes
---------------

The PS2 movies (FMVs): the PS2's pictures with the PS2's sound, in place of
the HD Remaster's re-made ones. The PS2 made them for a 4:3 screen: with
PS2 aspect ratio installed (its 4:3 picture on) they are built 4:3; without
it they are built 16:9 with the 4:3 picture between black bars. Install or
remove PS2 aspect ratio and the F8 window says the movies need building
again. The four movies the PC shows with the 3D scene drawn over them (the
Farplane and three more) stay HD: their 3D scene was redone for the HD
cameras.

The converter encodes the movies with ffmpeg. If none is found, install.cmd
downloads an LGPL build of FFmpeg 8.1.3 (168 MB) into the mod's tools\
folder and checks its SHA-256. The first build takes 30 to 35 minutes.

PS2 disc: needed (read once, by install.cmd)
Size: about 1.6 GB
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
   Fahrenheit's "bin" folder). It holds two folders: mods\yuta.ps2-movies\ (this
   mod) and mods\yuta.ffx-mods\ (the shared library every mod of the
   family needs; unpacking another mod's zip over it is fine).
2. Double-click mods\yuta.ps2-movies\install.cmd. It adds yuta.ffx-mods and
   yuta.ps2-movies to mods\loadorder, asks where your PS2 disc is
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

settings.toml in the mod folder, with a comment above each setting.
install.cmd makes it from settings.default.toml (the defaults) when there
is none; until then the mod uses the defaults. After changing one: run
install.cmd again (it redoes only what changed and removes the files of
whatever you switched back to HD), then restart the game.

Updating
--------

Unpack the newer zip over your install and run install.cmd again (it
rebuilds only what changed). The zip holds only settings.default.toml,
never settings.toml, so your settings stay; settings new in that version
are added to your settings.toml with their comments and defaults.

Switching it off
----------------

mods\yuta.ps2-movies\install.cmd -Disable ps2-movies takes the mod out of
mods\loadorder and keeps it out whenever install.cmd runs again (it is
listed in mods\ffx-mods-off.txt); install.cmd -Enable ps2-movies switches
it on again. Its folder and its PS2 files stay. Deleting its line in
mods\loadorder by hand does the same.

Uninstall
---------

Delete mods\yuta.ps2-movies\ and its line in mods\loadorder. That removes the PS2 files
it built too (they are in its folder). When no other mod of the
family is left, also delete mods\yuta.ffx-mods\ and its line.

Files
-----

  yuta.ps2-movies.dll, .manifest.json, ...   the mod
  settings.default.toml   the default settings (an update replaces it)
  settings.toml   your settings (made by install.cmd from settings.default.toml)
  install.cmd, install.ps1   the installer (load order, PS2 files)
  demaster.exe    the converter (builds the PS2 files from your disc)
  LICENSES.txt    licences of this mod and of the code it includes
  README.txt      this file
  ..\yuta.ffx-mods\   the shared library (settings, build status, the F8 window)

Made by install.cmd: efl\ (the PS2 files), build\ (what was built, for F8
and the next build), tools\ (ffmpeg, if it was downloaded).