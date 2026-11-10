//! The PS2 disc: its file table, the PC's names for its files, and the
//! decompressor. Layout: `docs/dev/disc.md`.
//!
//! The disc is an image file (`.iso`) or, on Windows, the disc drive with the
//! PS2 disc in it, read as a raw volume (`\\.\E:`, [`source_path`]). A volume
//! takes only whole sectors at sector offsets into sector-aligned memory, so
//! every read goes through [`SectorReader`], images too.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

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
    iso: SectorReader<File>,
    /// The image, for readers independent of `iso`'s file position.
    path: std::path::PathBuf,
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
        let mut iso = SectorReader::open(iso_path)?;
        let read = |iso: &mut SectorReader<File>, sector: u64, len: usize| -> Result<Vec<u8>> {
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
            path: iso_path.to_path_buf(),
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

    /// A reader over an uncompressed file (the movies: up to 590 MB) and
    /// its size, without loading it.
    pub fn open_stored(&mut self, name: &str) -> Result<Option<(impl Read + use<>, u64)>> {
        let Some(&i) = self.by_name.get(&name.to_ascii_lowercase()) else {
            return Ok(None);
        };
        let f = &self.files[i];
        if f.unpacked.is_some() {
            bail!("{name} is compressed");
        }
        let mut iso = SectorReader::open(&self.path)?;
        iso.seek(SeekFrom::Start(f.sector * SECTOR))?;
        Ok(Some((std::io::BufReader::with_capacity(1 << 20, iso.take(f.stored as u64)), f.stored as u64)))
    }

    /// Reads `len` bytes at `offset` of an uncompressed file (a bank of the
    /// 117 MB `wave_wd.dat`).
    pub fn read_range(&mut self, name: &str, offset: u64, len: usize) -> Result<Option<Vec<u8>>> {
        let Some(&i) = self.by_name.get(&name.to_ascii_lowercase()) else {
            return Ok(None);
        };
        let f = self.files[i].clone();
        if f.unpacked.is_some() {
            bail!("{name} is compressed");
        }
        let len = len.min((f.stored as u64).saturating_sub(offset) as usize);
        let mut b = vec![0; len];
        self.iso.seek(SeekFrom::Start(f.sector * SECTOR + offset))?;
        self.iso.read_exact(&mut b)?;
        Ok(Some(b))
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

/// The path to open for a disc given as `iso` in the settings or `--iso`: a
/// drive letter (`E:`, `E:\`) is that drive's raw volume, `\\.\E:` (Windows);
/// anything else is an image file.
pub fn source_path(s: &str) -> PathBuf {
    let t = s.trim().trim_end_matches(['\\', '/']);
    let b = t.as_bytes();
    if b.len() == 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return PathBuf::from(format!(r"\\.\{}:", (b[0] as char).to_ascii_uppercase()));
    }
    PathBuf::from(s.trim())
}

/// Whether `path` is a device (`\\.\E:`), not a file.
pub fn is_device(path: &Path) -> bool {
    path.to_string_lossy().starts_with(r"\\.\")
}

/// Whether there is a disc to read at `path`: an image file, or a drive
/// whose volume opens (a disc is in it).
pub fn exists(path: &Path) -> bool {
    if is_device(path) {
        SectorReader::open(path).is_ok_and(|mut r| r.read_exact(&mut [0; 1]).is_ok())
    } else {
        path.is_file()
    }
}

/// Checks that `path` holds a Final Fantasy X PS2 disc (its id in sector
/// 0x117) without the PC archive's tables; returns the volume id text.
pub fn check(path: &Path) -> Result<String> {
    let mut r = SectorReader::open(path)?;
    let mut id = vec![0; 0x50];
    r.seek(SeekFrom::Start(ID_SECTOR * SECTOR))?;
    r.read_exact(&mut id).with_context(|| format!("reading {}", path.display()))?;
    if &id[0x40..0x4f] != b"FINAL_FANTASY_X" {
        bail!("{} is not a Final Fantasy X PS2 disc", path.display());
    }
    Ok(String::from_utf8_lossy(&id[0x40..0x50]).trim_end_matches(['\0', ' ']).to_string())
}

/// What the disc is, for the build's job keys: a hash of its id sector and
/// file table, the same for a disc in a drive and its image (and for every
/// copy of that disc), so switching between them rebuilds nothing.
pub fn identity(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut r = SectorReader::open(path)?;
    let mut h = Sha256::new();
    for (sector, len) in [(ID_SECTOR, SECTOR as usize), (FAT_SECTOR, FAT_SIZE)] {
        let mut b = vec![0; len];
        r.seek(SeekFrom::Start(sector * SECTOR))?;
        r.read_exact(&mut b).with_context(|| format!("reading {}", path.display()))?;
        h.update(&b);
    }
    Ok(h.finalize().iter().take(12).map(|b| format!("{b:02x}")).collect())
}

/// Sectors per read: 256 KiB.
const CHUNK_SECTORS: usize = 128;

/// A buffer aligned to 4096 bytes (unbuffered device reads want the
/// memory aligned to the sector size; 4096 covers 2048-byte discs).
struct AlignedBuf {
    ptr: *mut u8,
    len: usize,
}

// The buffer is owned memory, used by one reader at a time.
unsafe impl Send for AlignedBuf {}
unsafe impl Sync for AlignedBuf {}

impl AlignedBuf {
    const ALIGN: usize = 4096;

    fn new(len: usize) -> Self {
        let layout = std::alloc::Layout::from_size_align(len, Self::ALIGN).expect("layout");
        // SAFETY: len > 0; a null pointer is handled.
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        AlignedBuf { ptr, len }
    }

    fn slice(&mut self) -> &mut [u8] {
        // SAFETY: ptr is a live allocation of len bytes owned by self.
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        let layout = std::alloc::Layout::from_size_align(self.len, Self::ALIGN).expect("layout");
        // SAFETY: allocated in new with this layout.
        unsafe { std::alloc::dealloc(self.ptr, layout) }
    }
}

/// Reads a disc image or a disc drive's raw volume at any offset and length,
/// asking the file only for whole 2048-byte sectors at sector offsets, into
/// aligned memory, as a raw volume (`\\.\E:`) requires. Reads near the end of
/// a volume fall back to one sector at a time, so the last sectors read
/// without asking past the end.
pub struct SectorReader<F> {
    inner: F,
    /// Position of the next byte for the caller.
    pos: u64,
    buf: AlignedBuf,
    /// Disc offset of `buf[0]` (a sector boundary) and the valid bytes.
    buf_at: u64,
    buf_len: usize,
}

impl SectorReader<File> {
    pub fn open(path: &Path) -> Result<Self> {
        let f = File::open(path).with_context(|| {
            if is_device(path) {
                format!("opening the disc drive {} (is the PS2 disc in it?)", path.display())
            } else {
                format!("opening {}", path.display())
            }
        })?;
        Ok(SectorReader::new(f))
    }
}

impl<F: Read + Seek> SectorReader<F> {
    pub fn new(inner: F) -> Self {
        SectorReader {
            inner,
            pos: 0,
            buf: AlignedBuf::new(CHUNK_SECTORS * SECTOR as usize),
            buf_at: 0,
            buf_len: 0,
        }
    }

    /// Reads up to `sectors` whole sectors at `at` (a sector boundary) into
    /// the buffer at `into`; returns the bytes read (short at the end).
    fn read_sectors(&mut self, at: u64, into: usize, sectors: usize) -> io::Result<usize> {
        let want = sectors * SECTOR as usize;
        self.inner.seek(SeekFrom::Start(at))?;
        let buf = &mut self.buf.slice()[into..into + want];
        let mut got = 0;
        while got < want {
            match self.inner.read(&mut buf[got..]) {
                Ok(0) => break,
                Ok(n) => got += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return if got > 0 { Ok(got) } else { Err(e) },
            }
        }
        Ok(got)
    }

    fn fill(&mut self) -> io::Result<()> {
        let at = self.pos / SECTOR * SECTOR;
        self.buf_at = at;
        self.buf_len = 0;
        match self.read_sectors(at, 0, CHUNK_SECTORS) {
            Ok(n) => self.buf_len = n,
            // A volume refuses a read that runs past its end: one sector at a
            // time, up to the first that fails (its end, for the caller: a
            // read_exact there fails as it would at the end of a file).
            Err(_) => {
                for k in 0..CHUNK_SECTORS {
                    match self.read_sectors(at + (k as u64) * SECTOR, k * SECTOR as usize, 1) {
                        Ok(n) if n > 0 => self.buf_len += n,
                        _ => break,
                    }
                }
            }
        }
        Ok(())
    }
}

impl<F: Read + Seek> Read for SectorReader<F> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        let inside = self.pos >= self.buf_at && self.pos < self.buf_at + self.buf_len as u64;
        if !inside {
            self.fill()?;
        }
        let off = (self.pos - self.buf_at) as usize;
        if off >= self.buf_len {
            return Ok(0);
        }
        let n = out.len().min(self.buf_len - off);
        out[..n].copy_from_slice(&self.buf.slice()[off..off + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl<F: Read + Seek> Seek for SectorReader<F> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.pos = match to {
            SeekFrom::Start(p) => p,
            SeekFrom::Current(d) => self.pos.checked_add_signed(d).ok_or(io::ErrorKind::InvalidInput)?,
            SeekFrom::End(_) => return Err(io::Error::new(io::ErrorKind::Unsupported, "a disc volume has no known end")),
        };
        Ok(self.pos)
    }
}

/// `sa_decode`: method 0 stored, 1 and 2 LZ (docs/dev/disc.md, "Compression").
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A volume as Windows gives it: whole sectors at sector offsets into
    /// aligned memory only, and nothing past its end.
    struct Volume {
        data: Vec<u8>,
        pos: u64,
        reads: usize,
    }

    impl Read for Volume {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            assert_eq!(self.pos % SECTOR, 0, "read at {}", self.pos);
            assert_eq!(out.len() % SECTOR as usize, 0, "read of {} bytes", out.len());
            assert_eq!(out.as_ptr() as usize % 2048, 0, "unaligned buffer");
            let end = self.pos as usize + out.len();
            if end > self.data.len() {
                return Err(io::Error::other("past the end of the volume"));
            }
            out.copy_from_slice(&self.data[self.pos as usize..end]);
            self.pos = end as u64;
            self.reads += 1;
            Ok(out.len())
        }
    }

    impl Seek for Volume {
        fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
            let SeekFrom::Start(p) = to else { panic!("{to:?}") };
            self.pos = p;
            Ok(p)
        }
    }

    #[test]
    fn sector_reader_reads_any_range_with_aligned_reads() {
        // 300 sectors: more than two chunks, and an end inside a chunk
        let data: Vec<u8> = (0..300 * SECTOR as usize).map(|i| (i * 7 % 251) as u8).collect();
        let mut r = SectorReader::new(Volume { data: data.clone(), pos: 0, reads: 0 });
        for (at, len) in [(0usize, 1usize), (5, 3000), (2047, 2), (100_000, 300_000), (299 * 2048 + 5, 2043), (2048 * 128 - 1, 2)] {
            let mut b = vec![0; len];
            r.seek(SeekFrom::Start(at as u64)).unwrap();
            r.read_exact(&mut b).unwrap();
            assert_eq!(b, data[at..at + len], "{at} {len}");
        }
        // the last chunk is short: read sector by sector up to the end, then EOF
        let mut b = vec![0; 10];
        r.seek(SeekFrom::Start(300 * SECTOR - 4)).unwrap();
        assert_eq!(r.read(&mut b).unwrap(), 4);
        assert_eq!(r.read(&mut b).unwrap(), 0);
        let mut all = Vec::new();
        r.seek(SeekFrom::Start(0)).unwrap();
        r.read_to_end(&mut all).unwrap();
        assert_eq!(all, data);
    }

    #[test]
    fn drive_letters_are_volumes() {
        assert_eq!(source_path("e:"), PathBuf::from(r"\\.\E:"));
        assert_eq!(source_path(r"E:\"), PathBuf::from(r"\\.\E:"));
        assert_eq!(source_path("E:/"), PathBuf::from(r"\\.\E:"));
        assert_eq!(source_path(r"E:\ffx.iso"), PathBuf::from(r"E:\ffx.iso"));
        assert_eq!(source_path("/tmp/dm-ffx.iso"), PathBuf::from("/tmp/dm-ffx.iso"));
        assert!(is_device(&source_path("E:")));
    }

    /// The real disc through the reader (skips without the image).
    #[test]
    fn real_disc_identity_and_check() {
        let iso = std::env::var("FFX_ISO").map_or_else(|_| PathBuf::from("/tmp/dm-ffx.iso"), Into::into);
        if !iso.is_file() {
            eprintln!("skipped: no {}", iso.display());
            return;
        }
        assert_eq!(check(&iso).unwrap(), "FINAL_FANTASY_X");
        assert_eq!(identity(&iso).unwrap().len(), 24);
        // the same bytes as a plain read
        let mut plain = File::open(&iso).unwrap();
        let mut a = vec![0; 70_000];
        plain.seek(SeekFrom::Start(FAT_SECTOR * SECTOR + 123)).unwrap();
        plain.read_exact(&mut a).unwrap();
        let mut r = SectorReader::open(&iso).unwrap();
        let mut b = vec![0; 70_000];
        r.seek(SeekFrom::Start(FAT_SECTOR * SECTOR + 123)).unwrap();
        r.read_exact(&mut b).unwrap();
        assert_eq!(a, b);
    }
}
