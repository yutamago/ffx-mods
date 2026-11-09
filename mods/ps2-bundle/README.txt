PS2 bundle
==========

FINAL FANTASY X HD Remaster (Steam, Windows) with the look and sound of the
original PS2 game: PS2 models, textures, maps, effects, movies, music,
lighting, font and menus, with the PC version's game logic underneath.

The PS2 bundle (yuta.ps2-bundle, formerly FFX Demaster) is every PS2 mod of
the FFX mods (https://github.com/yutamago/ffx-mods) in this one zip,
installed and built in one step. Each mod works alone too, and every change
is a setting, so you can keep any part of the HD Remaster.

Project page and issues: https://github.com/yutamago/ffx-mods


NO GAME DATA IS INCLUDED
------------------------

This zip has program code only. The PS2 files are built on your own computer
by install.cmd (with demaster.exe, the converter), from YOUR OWN Final
Fantasy X PS2 disc and your own Steam installation. The files it writes are
game data: don't share them.


Requirements
------------

- FINAL FANTASY X/X-2 HD Remaster on Steam (Windows).
- Fahrenheit 1.0.0-alpha12 or later:
  https://github.com/fahrenheit-crew/fahrenheit/releases
  installed as its guide says:
  https://github.com/fahrenheit-crew/fahrenheit/wiki/Installation-guide
- Your own Final Fantasy X PS2 disc, any region: an image of it (.iso), or
  the disc itself in a disc drive. install.cmd reads it once; after that the
  mods need neither the disc nor the image.
- About 7.5 GB of disk space for the PS2 files, and an internet connection for
  the first install (the movies need ffmpeg: if none is found, an LGPL build
  of FFmpeg, 168 MB, is downloaded and checked).


Install
-------

1. Unpack this zip into your Fahrenheit folder (the folder with Fahrenheit's
   "bin" folder). It adds one folder per mod under mods\ (mods\yuta.<mod>\),
   the shared library mods\yuta.ffx-mods\ (which holds the converter) and
   this folder, mods\yuta.ps2-bundle\.
2. Double-click mods\yuta.ps2-bundle\install.cmd. It
   - adds yuta.ffx-mods, every mod of the bundle and yuta.ps2-bundle to
     mods\loadorder (and takes out yuta.demaster, the old all-in-one FFX
     Demaster: delete its folder mods\yuta.demaster\ afterwards), except
     the mods you switched off (below);
   - makes each mod's settings.toml from its settings.default.toml (the
     defaults) when it has none;
   - asks where your PS2 disc is (the disc drives with the disc in them are
     listed; or pick an image file) and finds the game in your Steam folders
     (or asks for its folder);
   - builds the PS2 files of every mod from one pass over the disc, each
     into its own folder. The first build takes about an hour (half of it the
     movies); later runs redo only what changed.
   From a terminal: install.cmd -Iso 'D:\PS2\FFX.iso', or -Drive E, and
   -Game <game folder>, -Force (build everything again), -Disable <mod>
   and -Enable <mod> (below).
   (mods\yuta.ffx-mods\install.cmd does the same.)
3. Start the game through Fahrenheit (bin\fhstage0.exe with the path of
   FFX.exe).

In the game, F8 opens one window with a section for each mod: whether its
PS2 files match its settings (and the command to rebuild them), and which
code patches are on. The "PS2 bundle" section lists the bundle's mods and
which of them are on. The window opens by itself when something is wrong.

Upgrading (a newer PS2 bundle, or FFX Demaster's ffx-demaster-<version>.zip):
unpack the zip over your install and run mods\yuta.ps2-bundle\install.cmd.
The zip holds each mod's settings.default.toml (the defaults), never its
settings.toml, so your settings stay; settings new in that version are
added to your settings.toml files with their comments and defaults. It
adds yuta.ps2-bundle to mods\loadorder and builds only what is not up to
date.


The mods
--------

  yuta.ps2-music-engine    the PS2's sound driver plays the music live in the game
  yuta.ps2-soundtrack      the original soundtrack rendered from the PS2 music;
                           new games start with it
  yuta.ps2-movies          the PS2 movies, 4:3, with the PS2 sound
  yuta.ps2-characters      PS2 models and textures of the party, NPCs, monsters,
                           aeons, weapons and objects
  yuta.ps2-maps            PS2 field and battle map textures, PS2 painted backgrounds
  yuta.ps2-effects         PS2 spell, summon and effect textures
  yuta.ps2-lighting        characters lit the PS2 way
  yuta.no-hd-post-effects  no HD glow, no shadow maps (the PS2's ground shadows)
  yuta.ps2-interface       PS2 menus, windows, font, portraits, title screen,
                           button glyphs; a clean start-up
  yuta.ps2-aspect-ratio    the 4:3 picture with the PS2 camera
  yuta.ps2-game-feel       PS2 run speed, battle menu without input delay,
                           battle subtitles
  yuta.bug-fixes           NPC mouths, the Farplane video, Lulu's Fury spins

Each mod's folder has a README.txt with the details (it describes the mod as
its own zip; here demaster.exe is in mods\yuta.ffx-mods\, shared by all).
The other FFX mods (Modern Minimap, FPS unlock) are not PS2 parts and not in
the bundle; their own zips install next to it.


Settings
--------

Each mod has its own settings file, mods\yuta.<mod>\settings.toml, with a
comment above each setting; the defaults are the PS2's. install.cmd makes
it from settings.default.toml next to it (the defaults; an update replaces
only that one) and writes the disc and the game folder into it under
[source]. The bundle itself has no settings.

After changing a setting: run mods\yuta.ps2-bundle\install.cmd again (it
rebuilds what changed, in every mod; or the mod's own install.cmd for that
mod alone), then restart the game. Settings that are code only (the aspect
ratio, game feel, bug fixes, the post-effects) need only the restart.


Switching single mods off
-------------------------

- A part of a mod: set its setting to "hd" (or false for a fix) in that
  mod's settings.toml and run install.cmd again; the files of that part are
  removed.
- A whole mod: mods\yuta.ps2-bundle\install.cmd -Disable <mod>, for example
  -Disable ps2-movies (several: -Disable ps2-movies,ps2-maps). It takes the
  mod out of mods\loadorder and lists it in mods\ffx-mods-off.txt, and
  install.cmd never adds it back or builds it, until
  install.cmd -Enable <mod> (or until you delete its line there and run
  install.cmd). Deleting its line in mods\loadorder by hand does the same:
  the next install.cmd puts it on that list. Its folder and PS2 files stay
  (switching it on again rebuilds only what changed); delete the folder
  mods\yuta.<mod>\ too to get its disk space back. F8's "PS2 bundle"
  section shows the mod as switched off.

The mods work in any combination. PS2 movies and PS2 maps are built for the
picture the game shows: 4:3 with PS2 aspect ratio, 16:9 without it. After
removing or adding PS2 aspect ratio (or changing its setting), run
install.cmd again; F8 says so.


Uninstall
---------

Delete the mod folders (mods\yuta.ps2-*, mods\yuta.no-hd-post-effects,
mods\yuta.bug-fixes, and mods\yuta.ffx-mods if no other FFX mod is left)
and their lines in mods\loadorder. That removes the PS2 files they built too.


Files
-----

  mods\yuta.ps2-bundle\    the bundle: its manifest (which lists the mods of
                           the bundle) and DLL (its part of the F8 window),
                           install.cmd and install.ps1 (the installer for all
                           the mods), LICENSES.txt, this README.txt
  mods\yuta.ffx-mods\      the shared library (settings, build status, the F8
                           window), demaster.exe (the converter), install.cmd
                           (the same installer), LICENSES.txt
  mods\yuta.<mod>\         each mod: its DLL, settings.default.toml (the
                           defaults), README.txt, install.cmd (that mod
                           alone), LICENSES.txt
  mods\ffx-mods-off.txt    the switched-off mods (made by install.cmd -Disable)

Made by install.cmd in each mod's folder: settings.toml (your settings),
efl\ (the PS2 files), build\ (what was built, for F8 and the next build);
music\ (PS2 music engine), tools\ (ffmpeg, PS2 movies).
