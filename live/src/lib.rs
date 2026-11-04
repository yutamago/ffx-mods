//! FFX Demaster's live music engine (`audio.music_engine = "live"`,
//! `docs/music.md`, "Phase 2"): the PS2 sound driver plays the disc's
//! music inside the PC game, driven by the game's own music commands.
//!
//! - [`engine`]: songs per FMOD music event, mixed into one stream.
//! - `hook` (Windows): `demaster_live.dll`'s entry, the FMOD import
//!   wrappers and the user stream.

pub mod engine;
#[cfg(windows)]
mod hook;
