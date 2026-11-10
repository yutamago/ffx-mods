# Movies (rows 15, 21)

`movies = "ps2"` replaces the PC's FMVs with the PS2 ones from the disc.
Job `movies` (`converter/src/jobs/movies.rs`); decoder
`converter/src/formats/ps2movie.rs`. Debug commands:
`demaster movie-debug <iso> <vbf> <out> <name> [frame]...` (statistics,
PNGs of frames, the sound as WAV) and `demaster movies <iso> <vbf> <mod>
all|<name>...` (the job for some movies).

## The PS2 files

`jppc/movie/adpcm/<name>.dat` and `<name>.sz`, 53 movies, 2.6 GB on the
German disc, stored uncompressed. (The 8 small `jppc/movie/*.dat`, `ikai`,
`hiroba`, `luca4_*`, ..., are the in-map texture videos, not handled.)

`.sz`: the IPU's intra and non-intra quantiser matrices (64 bytes each, raster
order as uploaded with `SETIQ`), then one `u32` per frame: the size of that
frame's chunk in `.dat`. The chunk sizes add up to the `.dat` exactly.

A chunk is a run of records with a 16-byte header:

| Record | Header | Body |
|---|---|---|
| picture (always first) | `u32` bitstream size, `u32`, `+8` type (0 intra, 1 predicted), `+9` intra quantiser, `+0xa` non-intra quantiser, `+0xb`/`+0xc` width/height in macroblocks (0x28 x 0x1a = 640 x 416; 0x20 = 512 wide in four movies), `+0xe`/`+0xf` IPU_CTRL bits (`+0xf` >> 1 = DC precision; 4 in most movies, 10 bits) | the bitstream, padded (the end-of-picture code comes within 32 bytes of the size) |
| `0x40` | `0x40`, code bytes, padded size, 0 | delta-coded sound |
| `0x50`, `0x51` | type, unpacked size, packed size, second track size (0 in every movie) | `sa_decode` LZ (`docs/disc.md`) of delta-coded sound |
| `0x7800` | `0x7800` | raw PCM |
| `0x30` | `0x30`, -1, -1, -1 | 0x30 bytes of camera floats (sda, and the four movies with a 3D scene on top) |
| end | 16 zero bytes | |

Sound (`MovieDecoder_runCommand_0077e460`, `0x0077b270`, `0x0077b1f0`):
s16 samples in blocks of 256 left then 256 right, 48 kHz stereo. Delta
code: a byte with bit 7 set adds its low 7 bits (signed), otherwise two
bytes add a 15-bit signed value; the running sum is the sample. One sound
record holds 7,680 sample frames (4 PAL frames at 48 kHz; the 0x6400 and
0x3c00 sizes in the code are NTSC's 6,400). There is no ADPCM in these
files despite the folder name, and no 5.1: every movie is plain stereo
(row 21's "5.1" is not on this disc; whether the stereo is matrix-encoded
surround was not checked).

## The picture bitstream

FFX.exe still has the PS2 player, unreached: `MovieViewer_IntraDecode`
(`0x0077b480`), the predicted-picture decoder `0x0077b630` (Ghidra drops
most of it: the IPU busy-wait loops look endless; `objdump` shows it),
`IpuLoadMacroblock_0077e210`, `MovieViewer_BlockStep_0077c3c0`. The IPU
commands are writes to `g_GsRegisterSink`. The IPU itself is hardware; its
block decode follows PCSX2's emulation (`pcsx2/IPU/IPU_MultiISA.cpp`).

- Intra picture: per macroblock a `VDEC` macroblock-address-increment code
  (always 1) and a `BDEC` intra block decode: 4 luma + Cb + Cr blocks,
  DC predictors reset at the first macroblock, intra VLC table B.15
  (IPU_CTRL IVF is always set), zigzag scan, linear quantiser (`q * 2`),
  the quantiser matrix indexed by scan position as PCSX2's IPU does, no
  mismatch control; libmpeg2's C IDCT (PCSX2's `IDCT_Block`).
- Predicted picture: per macroblock a mode coded with the
  address-increment table (1 motion + residual, 2 intra, 3 motion only, 4
  end of picture), a second address-increment code (escapes skipped),
  then for modes 1 and 3 four motion codes a, b, c, d (table B.10 with
  sign bits): the vector is (16 a + c, 16 b + d) in whole pixels. Mode 1
  reads `BDEC` non-intra (coded block pattern, table B.14, the non-intra
  quantiser), mode 2 an intra `BDEC` (DC predictors reset at the first
  intra macroblock of the picture).
- Prediction reads the **last intra picture**, not the previous picture:
  the macroblock store (`+0x503c8`) is written only by the intra decoder.
  Predicting from the previous picture falls apart within a few frames
  (checked on sak). Chroma moves by floor(x / 2) whole pixels; averaging
  odd vectors (MPEG half-pel) matched the next intra picture worse
  (sak, PSNR of Cb/Cr 1-1.5 dB lower).
- Outside the picture the PS2 clears the quarters of its 2x2-macroblock
  window (green after colour conversion). A few vectors point there with
  no residual, so the encoder saw real pixels; the decoder repeats the
  edge instead.

All 53 movies decode with every picture ending at its record's size
(68,616 pictures).

## What the PC plays

`PVideoPlaybackWin32` (`docs/formats/platform.md` in ffx-rust, "Movies"):
VP8 + Vorbis WebM (libvpx 1.4.0, mkvparser, libVorbis), any size, drawn
over the whole screen with a full-range BT.601 shader. There is no A/V
clock: the video advances one frame per game update (29.97 a second) and
the sound plays in real time, and the scripts read the frame number as
ms x 29.97. The shipped files are 1280 x 720, 29.97 fps, Vorbis 48 kHz
stereo.

`ffx_data/gamedata/ps3data/video/us/ffx_videolist.txt` names the file per
movie number for the US/European versions (`LocalizationManager` 1 and 2);
the job writes the file that list names (`opn_us.webm`, or `ikm_jp.webm`
where the list has only the JP cut). Japanese-master installs read
`jp/ffx_videolist.txt` and keep the HD `_jp` files where both exist.
PS2 `saw` is the PC's `swa`. The PC has no PS2 counterpart for `fal`,
`syuin` and the `*_480` texture videos.

So each movie becomes: 640 x 416 (512-wide pictures widened to 640) scaled
to 960 x 720; with `render.aspect = "ps2"` (the default: the game's
picture is 4:3 and the movie quad fills it, `docs/screen.md`) that is the
file, with `"hd"` it is centred in 1280 x 720 with black bars (4:3 inside
16:9); studio range expanded to full range (blacks 0, as the PC's shader and the
PS2's IPU colour conversion show them; the HD files are studio range and
so show grey blacks), 25 fps duplicated up to 30000/1001, VP8 (`libvpx`,
`-deadline good -cpu-used 4 -b:v 4M -crf 8`, no alt-ref frames), Vorbis
q5. The movie keeps the PS2 duration (frames / 25); 45 of the 49 are
within 0.2 s of the HD's, `fin` 1.4 s, `bts` and `sls` 1.0 s and `end`
0.8 s shorter.

Left HD: `mst`, `far`, `ggm`, `sns` (ids 0x20, 0x2e, 0x46, 0x48,
`movie_have_camera`): the game draws the 3D scene over them with the PC's
16:9 cameras (`video/<id>.dat`), which would not line up with a 4:3 PS2
picture. Their PS2 versions switch to 512 x 416 where the 3D scene goes on
top. With `render.aspect = "ps2"` the 3D scene shows the middle 4:3 of the
PC camera's view, so the job cuts the HD file to its middle 960 x 720
(VP8 as above, the sound copied; mst and ggm checked with ffprobe; not
seen in the game). Under 4:3 a movie left at 1280 x 720 (`movies = "hd"`)
is squeezed to 4:3.

IWK at 960 x 720 played in the harness with the 4:3 output
(`~/ffx-work/demaster/research/aspect/fmv-iwk-43.png`): it fills the 4:3.

Sound: the PS2's own. It is the same mix as the HD `_us` files (sak:
correlation 0.97 at 8 kHz, the HD 21 ms later). The German disc's movies
have the English voices and no German text in the pictures that were
looked at (iwk, opk, sak).

## ffmpeg

The converter runs an external ffmpeg: `--ffmpeg`, `[source] ffmpeg` in
`demaster.toml`, else `ffmpeg.exe` next to `demaster.exe` or in
`<mod>/tools`, else `ffmpeg` on the PATH. On Windows, when none is found,
it downloads BtbN's LGPL (v3) build of FFmpeg 8.1.3
(`autobuild-2026-11-01-12-49`, 168 MB, pinned by URL and SHA-256 in
`jobs/movies.rs`) with Windows' own `curl.exe`, unpacks only `ffmpeg.exe` and
its licence with `tar.exe` into `<mod>/tools`, and says so. Offline, the
job fails with a message, the movies stay HD, everything else builds, and
the next build tries again. BtbN deletes old autobuilds after a while: when
the URL is gone, pin a newer build (URL, directory name, SHA-256).

Encoding all 49 movies took 25 minutes (WSL, 4 threads, while other work ran); the output is
1.5 GB.

## Verified

- All 53 decode (statistics above); PNGs of intra and predicted frames
  looked at (iwk, sak); frame counts and durations of all 49 outputs with
  ffprobe; bars at Y = 0.
- Side by side with the HD frame at the same time: iwk at 8 s, opk at 3 s
  (the PS2 picture shows more at the top and bottom, the HD crops it).
- In the game (ffx-rust harness, `movie-fmv` replay: IWK_us.webm on
  Besaid beach) the converted movie plays and the frame counter advances
  as with the HD file (frame 320 at update 650).
- The Windows `demaster.exe` downloaded ffmpeg, verified it and converted
  `iwk`.

Not verified: sound sync by ear in the game; the other 48 movies in the
game; a Japanese-master install.
