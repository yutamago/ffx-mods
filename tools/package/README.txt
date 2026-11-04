FFX Demaster
============

A Fahrenheit mod for FINAL FANTASY X HD Remaster (Steam, Windows) that brings
back the look of the original PS2 game (PS2 models, textures, effects, movies,
font and menus) with the PC version's game logic underneath. Every change is
a setting, so you can keep any part of the HD Remaster. The mod is a work in
progress; docs/plan.md on the project page lists what is done.

Project page, documentation and issues: https://github.com/yutamago/ffx-demaster


NO GAME DATA IS INCLUDED
------------------------

This zip has program code only. The PS2 assets are built on your own computer
by demaster.exe, from a disc image of YOUR OWN Final Fantasy X PS2 disc and
your own Steam installation. Without a PS2 disc image the mod does nothing.
The files demaster.exe writes are game data: don't share them.


Requirements
------------

- FINAL FANTASY X/X-2 HD Remaster on Steam (Windows).
- Fahrenheit 1.0.0-alpha12 or later:
  https://github.com/fahrenheit-crew/fahrenheit/releases
  installed as its guide says:
  https://github.com/fahrenheit-crew/fahrenheit/wiki/Installation-guide
- An image (.iso) of your own Final Fantasy X PS2 disc, any region.
- Free disk space for the converted files: a build with the default settings
  is about 7 GB.
- An internet connection for the first build, unless you have ffmpeg (see
  "ffmpeg" below).


Install
-------

1. Unpack this zip into your Fahrenheit folder (the folder with Fahrenheit's
   "bin" folder). The mod ends up in mods\yuta.demaster\.

2. Open mods\yuta.demaster\demaster.toml in a text editor (Notepad works) and
   set "iso" under [source] to your disc image. Use single quotes:

       [source]
       iso = 'D:\PS2\Final Fantasy X.iso'

   demaster.exe finds the game in the usual Steam folders. If yours is
   elsewhere, also set "game" to the folder that holds FFX.exe.

3. Double-click mods\yuta.demaster\demaster.exe. It reads your disc and the
   game's data and writes the PS2 files into the mod folder. The first run
   takes a while (the movies alone take several minutes); the window stays
   open at the end so you can read the result. Press Enter to close it.

4. Add a line with "yuta.demaster" to mods\loadorder (create the file if it
   doesn't exist).

5. Start the game through Fahrenheit, as its guide describes
   (bin\fhstage0.exe with the path of FFX.exe).

In the game, F8 opens a small window that says whether the built files match
demaster.toml and what to do if they don't. It opens by itself when they
don't match.


ffmpeg (first build only)
-------------------------

demaster.exe converts the PS2 movies with ffmpeg, which isn't included in
this zip. If you have no ffmpeg, the first build downloads an LGPL build of
FFmpeg 8.1.3 (168 MB, from github.com/BtbN/FFmpeg-Builds), checks its
SHA-256 checksum and keeps ffmpeg.exe and its licence in
mods\yuta.demaster\tools\. Later builds reuse it.

To use your own ffmpeg instead, set "ffmpeg" under [source] in demaster.toml,
or put ffmpeg.exe next to demaster.exe. Without internet and without ffmpeg,
only the movies stay HD; everything else is built, and the next build tries
again. To keep the HD movies, set movies = "hd".


Changing settings
-----------------

All settings are in demaster.toml, with a comment above each one. config.md
in this folder describes them all. Presentation settings take "ps2" or "hd";
fixes take true or false.

After changing a setting:
  1. run demaster.exe again (it only redoes what changed, and removes the
     files of whatever you switched back to HD);
  2. restart the game. Fahrenheit loads the mod's files only when the game
     starts.

After updating the mod, run demaster.exe again too.


Uninstall
---------

Delete mods\yuta.demaster and its line in mods\loadorder.


Files
-----

  yuta.demaster.dll, .deps.json, .runtimeconfig.json, .manifest.json, .pdb
                    the Fahrenheit mod
  demaster.exe      the converter (builds the PS2 files from your disc)
  demaster.toml     the settings
  config.md         what each setting does
  LICENSES.txt      licences of this mod and of the code it includes
  README.txt        this file

Files that demaster.exe creates later: efl\ (the converted game files),
build\ (what was built, for F8 and the next build), tools\ (ffmpeg).
