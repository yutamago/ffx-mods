# The PS2 disc

How the converter finds and unpacks files on an FFX PS2 disc image. Checked
on the German PAL disc (`SCES_504.92`). Extractor:
[`tools/ps2x`](../../tools/dev/ps2x/src/main.rs).

## ISO 9660 holds only the boot files

The disc's file system lists the ELF (`SCES_504.92`), `SYSTEM.CNF`, the IOP
modules (`*.IRX`) and `IOPRP234.IMG`. The game data is in raw sectors that the
ELF finds through its own tables.

## Tables

FFX.exe still has the PS2 constants (`dvd_cd_*`, `0x0076d6c0` + `0x80 * kind`;
ffx-rust `docs/formats/system.md`), and the German disc uses the same values:

| Sector | Size | What |
|---|---|---|
| `0x117` | `0x30` | ID block: the table sectors and sizes again, then `FINAL_FANTASY_X` at `+0x40` |
| `0x118` | `0x20000` bytes | `cdrom.mdg`: one `u32` per file number |
| `0x158` | `0x82` bytes | `cdrom.fid`: 65 `i16`, the first file number of each module (-1: none) |

`cdrom.mdg` entry (`CDfileSize_CD` `0x0076c320`, `cd_file_sector`):

| Bits | Meaning |
|---|---|
| 0-21 | start sector |
| 22 | stored compressed; the unpacked size is in `sizetbl.bin` |
| 23 | the file is not on this disc |
| 24-31 | uncompressed files: unused bytes at the end of the last sector, / 8 |

The stored size is `(next start - start) * 2048`, less `top byte * 8` for an
uncompressed file.

The PC ships the German tables byte for byte as
`ffx_ps2/ffx/proj/prog/cdidx/de/{cdrom.mdg,cdrom.fid,cdrom.id}`, and next to
them `de/sizetbl.bin`: 3 bytes per file number, the unpacked size / 8. Other
regions have their own folders (`jp`, `us`, `uk`, `fr`, `it`, `es`). Each
disc's numbering differs, so the converter has to pick the folder whose
`cdrom.mdg` equals the disc's.

## Names

The PC's file names are in `ffx_ps2/ffx/proj/battle/jp/cddata/cdrom.fnd`
(`pc_fat_index_filename_set` `0x00a42fc0`, `PcFileName_00a42c00`): a `u32`
count, `count + 1` offsets from the table's start, then NUL-terminated
`host0:/ffx/master/...` paths. Entry *n* runs from offset *n* to offset *n + 1*;
an empty range has no name.

`cdrom.fnd` is numbered after the Japanese International disc
(`battle/jp/cddata/cdrom.fid`). A file of another disc is named by its module:
for file *i* in module *m* (the module whose start in the disc's `cdrom.fid` is
the largest at or below *i*), the name index is
`jp_fid[m] + (i - disc_fid[m])`. On the German disc every module starts 49
files earlier than on the Japanese one, except modules 0, 1 and 3.

With this mapping, 2,552 `chr` files and 250 event motion groups come out
byte-identical to the PC's copies, which confirms the names.

## Compression

Files with bit 22 set start with a method byte and the unpacked size (`u32`,
unaligned), then codes from `+5` (`sa_decode` `0x00a43de0`, ported in ffx-rust
`crates/ffx-sg/src/decode.rs`):

| Code | Meaning |
|---|---|
| `0` | end |
| `1..=0x7d` | that many literal bytes |
| `0x7e n b` | `n + 4` bytes `b` |
| `0x7f n:u16 b` | `n` bytes `b` |
| `0x80..=0xff x` | back reference. Method 1: length `(c >> 3 & 0xf) + 3`, distance `((c & 7) << 8 \| x) + 1`. Method 2: length `(c >> 4 & 7) + 3`, distance `((c & 0xf) << 8 \| x) + 1`. Bytes from before the start read as 0 |

Method 0 stores the bytes uncompressed. All 6,774 compressed files on the
German disc decode.

## What the German disc holds

7,363 files, 5.2 GB unpacked. 509 have no name in `cdrom.fnd` (most of them
are in the gaps between modules); 584 effect files are named
`host0:magic/mag_NNNN.bin`, and 121 sound files `host0:/ffx/proj/sound/...`.

Compared with the PC's `ffx_ps2/ffx/master/` copies:

| PS2 files | On PC |
|---|---|
| 857 `chr/**/*.chr` | 805 identical except the texture pixels, which are zeroed; 52 identical |
| 311 field and 56 battle map `mapout.txa` (map textures) | missing |
| 311 `mapout.vpa` (map geometry) | 277 the same size with regions zeroed (most PS2 geometry packets and the 2D background image) |
| 584 `mag_NNNN.bin` (effects) | missing; the PC has native `magicFiles/FFX/magicNNNN.dll` and Phyre textures |
| `movie/**` | missing; the PC has its own videos |
| event scripts, kernel, battle data | mostly identical or only size-equal (PAL text) |

So the PC still ships the PS2 meshes, skeletons and palettes. The PS2 pixels,
map textures, effects and movies have to come from the disc.
