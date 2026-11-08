//! The converter as a library: the `demaster` binary (`main.rs`) and FFX
//! Music Studio (`studio/`) use these modules. The studio needs the disc
//! reader (`disc`), the settings (`config`, `build::find_vbf`) and the music
//! engine (`sqseq`, `spu2`, `musicjob`).

pub mod bake;
pub mod bgjob;
pub mod build;
pub mod chralpha;
pub mod chrjob;
pub mod config;
pub mod disc;
pub mod dxbc;
pub mod eventlight;
pub mod fontjob;
pub mod fsb5;
pub mod fxjob;
pub mod glb;
pub mod gsmem;
pub mod jobs;
pub mod livejob;
pub mod mapjob;
pub mod mataudit;
pub mod models;
pub mod moviejob;
pub mod musicjob;
pub mod phyretex;
pub mod ps2chr;
pub mod ps2fx;
pub mod ps2map;
pub mod ps2mesh;
pub mod ps2movie;
pub mod sgjob;
pub mod shaderjob;
pub mod spu2;
pub mod sqseq;
pub mod texanim;
pub mod uijob;
