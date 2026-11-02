//! The PS2 disc: its file table, the PC's names for its files, and the
//! decompressor. Layout: `docs/disc.md`.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use anyhow::{Context, Result, bail};

const SECTOR: u64 = 2048;
const FAT_SECTOR: u64 = 0x118;
const FAT_SIZE: usize = 0x2_0000;
const FID_SECTOR: u64 = 0x158;
const FID_SIZE: usize = 0x82;
const ID_SECTOR: u64 = 0x117;

/// The PC archive's copies of the disc tables.
const CDIDX: &str = "ffx_ps2/ffx/proj/prog/cdidx";
const NAMES: &str = "ffx_ps2/ffx/proj/battle/jp/cddata/cdrom.fnd";
const NAMES_FID: &str = "ffx_ps2/ffx/proj/battle/jp/cddata/cdrom.fid";
const REGIONS: [&str; 7] = ["de", "us", "uk", "fr", "it", "es", "jp"];

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"))
}

fn fid(b: &[u8]) -> Vec<i32> {
    (0..FID_SIZE / 2)
        .map(|m| i32::from(i16::from_le_bytes([b[2 * m], b[2 * m + 1]])))
        .collect()
}

/// One file on the disc.
#[derive(Debug, Clone)]
pub struct DiscFile {
    pub number: usize,
    pub sector: u64,
    /// Bytes stored on the disc.
    pub stored: usize,
    /// Unpacked size for compressed files.
    pub unpacked: Option<usize>,
}

pub struct Disc {
    iso: File,
    /// The region folder whose tables match the disc (`de`, `us`, ...).
    pub region: String,
    files: Vec<DiscFile>,
    /// PC-style path (`jppc/chr/...`, without `ffx_ps2/ffx/master/`;
    /// `_host0/magic/mag_0007.bin` for the effects) to the index in `files`.
    by_name: HashMap<String, usize>,
}

impl Disc {
    /// Opens a disc image and names its files with the PC archive's tables.
    pub fn open(iso_path: &Path, pc: &ffx_vbf::Archive) -> Result<Self> {
        let mut iso =
            File::open(iso_path).with_context(|| format!("opening {}", iso_path.display()))?;
        let read = |iso: &mut File, sector: u64, len: usize| -> Result<Vec<u8>> {
            let mut b = vec![0; len];
            iso.seek(SeekFrom::Start(sector * SECTOR))?;
            iso.read_exact(&mut b)?;
            Ok(b)
        };
        let id = read(&mut iso, ID_SECTOR, 0x50)?;
        if &id[0x40..0x4f] != b"FINAL_FANTASY_X" {
            bail!("{} is not a Final Fantasy X PS2 disc", iso_path.display());
        }
        let fat = read(&mut iso, FAT_SECTOR, FAT_SIZE)?;
        let disc_fid = read(&mut iso, FID_SECTOR, FID_SIZE)?;

        let pc_file = |path: &str| -> Result<Vec<u8>> {
            let e = pc
                .find(path)
                .with_context(|| format!("{path} is missing from the PC archive"))?;
            Ok(pc.read(e)?)
        };
        let mut region = None;
        for r in REGIONS {
            if pc_file(&format!("{CDIDX}/{r}/cdrom.mdg"))? == fat {
                region = Some(r);
                break;
            }
        }
        let Some(region) = region else {
            bail!("this disc's file table matches none of the PC's region tables ({REGIONS:?})");
        };
        let sizes = pc_file(&format!("{CDIDX}/{region}/sizetbl.bin"))?;
        let names = pc_file(NAMES)?;
        let names_fid = fid(&pc_file(NAMES_FID)?);
        let disc_fid = fid(&disc_fid);

        let entries: Vec<u32> = (0..FAT_SIZE / 4).map(|i| u32_at(&fat, 4 * i)).collect();
        let mut starts: Vec<(i32, usize)> = disc_fid
            .iter()
            .enumerate()
            .filter(|&(_, &s)| s >= 0)
            .map(|(m, &s)| (s, m))
            .collect();
        starts.sort_unstable();
        let name_count = u32_at(&names, 0) as usize;
        let name_of = |i: usize| -> Option<String> {
            let &(s, m) = starts.iter().rev().find(|&&(s, _)| s as usize <= i)?;
            if names_fid[m] < 0 {
                return None;
            }
            let n = usize::try_from(names_fid[m] + (i as i32 - s)).ok()?;
            if n >= name_count {
                return None;
            }
            let (a, b) = (
                u32_at(&names, 4 + 4 * n) as usize,
                u32_at(&names, 8 + 4 * n) as usize,
            );
            if b <= a {
                return None;
            }
            let raw = &names[a..b];
            let z = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
            let s = String::from_utf8_lossy(&raw[..z]);
            // As the extractor (tools/ps2x) names them: the effects
            // (`host0:magic/mag_NNNN.bin`) become `_host0/magic/...`.
            let s = if let Some(rest) = s.strip_prefix("host0:/ffx/master/") {
                rest.to_string()
            } else if let Some(rest) = s.strip_prefix("host0:/ffx/") {
                format!("_ffx/{rest}")
            } else {
                format!("_host0/{}", s.strip_prefix("host0:")?)
            };
            Some(s.to_ascii_lowercase())
        };

        let mut files = Vec::new();
        let mut by_name = HashMap::new();
        for i in 0..entries.len() - 1 {
            let e = entries[i];
            if e & 0x80_0000 != 0 {
                continue;
            }
            let sector = u64::from(e & 0x3f_ffff);
            let next = u64::from(entries[i + 1] & 0x3f_ffff);
            if next <= sector {
                continue;
            }
            let compressed = e & 0x40_0000 != 0;
            let mut stored = usize::try_from((next - sector) * SECTOR)?;
            if !compressed {
                stored -= (e >> 24) as usize * 8;
            }
            let unpacked = compressed.then(|| {
                let b = &sizes[3 * i..3 * i + 3];
                (u32::from_le_bytes([b[0], b[1], b[2], 0]) as usize) << 3
            });
            if let Some(name) = name_of(i) {
                by_name.insert(name, files.len());
            }
            files.push(DiscFile {
                number: i,
                sector,
                stored,
                unpacked,
            });
        }
        Ok(Self {
            iso,
            region: region.to_string(),
            files,
            by_name,
        })
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Names (`jppc/...`) of every named file.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(String::as_str)
    }

    /// Reads and unpacks a file by its PC name (`jppc/chr/pc/c001/mdl/c001.chr`).
    pub fn read(&mut self, name: &str) -> Result<Option<Vec<u8>>> {
        let Some(&i) = self.by_name.get(&name.to_ascii_lowercase()) else {
            return Ok(None);
        };
        let f = self.files[i].clone();
        let mut b = vec![0; f.stored];
        self.iso.seek(SeekFrom::Start(f.sector * SECTOR))?;
        self.iso.read_exact(&mut b)?;
        match f.unpacked {
            None => Ok(Some(b)),
            Some(size) => {
                let mut d = sa_decode(&b)
                    .with_context(|| format!("unpacking {name} (file {})", f.number))?;
                d.resize(size.max(d.len()), 0);
                Ok(Some(d))
            }
        }
    }
}

/// `sa_decode`: method 0 stored, 1 and 2 LZ (docs/disc.md, "Compression").
pub fn sa_decode(src: &[u8]) -> Result<Vec<u8>> {
    let get = |i: usize| src.get(i).copied().context("compressed stream ends early");
    let method = get(0)?;
    let size = u32_at(src, 1) as usize;
    if method == 0 {
        return Ok(src
            .get(5..5 + size)
            .context("stored stream too short")?
            .to_vec());
    }
    if method > 2 {
        bail!("unknown compression method {method}");
    }
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let mut i = 5;
    loop {
        let c = usize::from(get(i)?);
        i += 1;
        match c {
            0 => break,
            1..=0x7d => {
                out.extend_from_slice(src.get(i..i + c).context("literal run past the end")?);
                i += c;
            }
            0x7e => {
                let n = usize::from(get(i)?) + 4;
                let b = get(i + 1)?;
                i += 2;
                out.resize(out.len() + n, b);
            }
            0x7f => {
                let n = usize::from(u16::from_le_bytes([get(i)?, get(i + 1)?]));
                let b = get(i + 2)?;
                i += 3;
                out.resize(out.len() + n, b);
            }
            _ => {
                let x = usize::from(get(i)?);
                i += 1;
                let (len, dist) = if method == 1 {
                    ((c >> 3 & 0xf) + 3, ((c & 7) << 8 | x) + 1)
                } else {
                    ((c >> 4 & 7) + 3, ((c & 0xf) << 8 | x) + 1)
                };
                for _ in 0..len {
                    let b = if out.len() >= dist {
                        out[out.len() - dist]
                    } else {
                        0
                    };
                    out.push(b);
                }
            }
        }
    }
    Ok(out)
}
