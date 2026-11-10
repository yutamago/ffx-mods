//! The converter as a library: the `demaster` binary (`main.rs`), the live
//! music engine (`demaster-live/`) and FFX Music Studio use these
//! modules. The studio needs the disc reader (`disc`), the settings
//! (`config`, `build::find_vbf`) and the music engine (`sqseq`, `spu2`,
//! `musicjob`); keep their public API (the re-exports below keep the paths
//! they had before the modules moved into folders).
//!
//! - `build`, `config`, `disc`, `mods`: the build, the settings, the PS2
//!   disc reader and the mods the converter builds for.
//! - `jobs`: one file per job (`jobs::registry()`), named after what it builds.
//! - `formats`: readers and writers of the PS2 and PC file formats.
//! - `characters`: what the character jobs share (texel baking, alpha,
//!   texture animation, the material audit).
//! - `audio`: the PS2 sound driver (`sqseq`) on a model of the SPU2 (`spu2`).

pub mod audio;
pub mod build;
pub mod characters;
pub mod config;
pub mod disc;
pub mod formats;
pub mod jobs;
pub mod mods;

pub use audio::{spu2, sqseq};
pub use jobs::music as musicjob;
