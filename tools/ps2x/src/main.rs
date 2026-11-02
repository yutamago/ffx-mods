//! ps2x <iso> <cdrom.fnd> <sizetbl.bin> <out> <name-table cdrom.fid>: extract FFX PS2 disc files.
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

pub fn sa_decode(src: &[u8]) -> Option<Vec<u8>> {
    let method = src[0];
    let size = u32le(src, 1) as usize;
    if method == 0 {
        return Some(src[5..5 + size].to_vec());
    }
    if method > 2 {
        return None;
    }
    let mut out: Vec<u8> = Vec::with_capacity(size);
    let mut i = 5;
    loop {
        let c = *src.get(i)? as usize;
        i += 1;
        match c {
            0 => break,
            1..=0x7d => {
                out.extend_from_slice(src.get(i..i + c)?);
                i += c;
            }
            0x7e => {
                let n = src[i] as usize + 4;
                let b = src[i + 1];
                i += 2;
                out.extend(std::iter::repeat(b).take(n));
            }
            0x7f => {
                let n = u16::from_le_bytes([src[i], src[i + 1]]) as usize;
                let b = src[i + 2];
                i += 3;
                out.extend(std::iter::repeat(b).take(n));
            }
            _ => {
                let x = src[i] as usize;
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
    Some(out)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut iso = fs::File::open(&a[1]).unwrap();
    let fnd = fs::read(&a[2]).unwrap();
    let sizetbl = fs::read(&a[3]).unwrap();
    let out = Path::new(&a[4]);
    // module starts: the disc's FID (sector 0x158) and the name table's (jp cdrom.fid)
    let mut disc_fid = vec![0u8; 0x82];
    iso.seek(SeekFrom::Start(0x158 * 2048)).unwrap();
    iso.read_exact(&mut disc_fid).unwrap();
    let name_fid = fs::read(&a[5]).unwrap();
    let fid = |b: &[u8]| -> Vec<i32> {
        (0..65)
            .map(|m| i32::from(i16::from_le_bytes([b[2 * m], b[2 * m + 1]])))
            .collect()
    };
    let (dfid, nfid) = (fid(&disc_fid), fid(&name_fid));
    let mut starts: Vec<(i32, usize)> = dfid
        .iter()
        .enumerate()
        .filter(|(_, &s)| s >= 0)
        .map(|(m, &s)| (s, m))
        .collect();
    starts.sort();
    let to_name_index = |i: usize| -> Option<usize> {
        let (s, m) = *starts.iter().rev().find(|(s, _)| *s as usize <= i)?;
        if nfid[m] < 0 {
            return None;
        }
        Some((nfid[m] + (i as i32 - s)) as usize)
    };
    let mut fat = vec![0u8; 0x20000];
    iso.seek(SeekFrom::Start(0x118 * 2048)).unwrap();
    iso.read_exact(&mut fat).unwrap();
    let ent: Vec<u32> = (0..fat.len() / 4).map(|i| u32le(&fat, i * 4)).collect();
    let n = u32le(&fnd, 0) as usize;
    let name = |i: usize| -> Option<String> {
        if i >= n {
            return None;
        }
        let (s, e) = (
            u32le(&fnd, 4 + 4 * i) as usize,
            u32le(&fnd, 8 + 4 * i) as usize,
        );
        if e <= s {
            return None;
        }
        let raw = &fnd[s..e];
        let z = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        Some(String::from_utf8_lossy(&raw[..z]).into_owned())
    };
    let mut idx = fs::File::create(out.join("index.tsv")).unwrap();
    let (mut ok, mut bad) = (0, 0);
    for i in 0..ent.len() - 1 {
        let e = ent[i];
        if e & 0x80_0000 != 0 {
            continue;
        }
        let st = (e & 0x3f_ffff) as u64;
        let nx = (ent[i + 1] & 0x3f_ffff) as u64;
        if nx <= st {
            continue;
        }
        let stored =
            ((nx - st) << 11) as usize - ((e >> 24) as usize) * 8 * usize::from(e & 0x40_0000 == 0);
        let mut buf = vec![0u8; stored];
        iso.seek(SeekFrom::Start(st * 2048)).unwrap();
        iso.read_exact(&mut buf).unwrap();
        let compressed = e & 0x40_0000 != 0;
        let data = if compressed {
            let want =
                (u32::from_le_bytes([sizetbl[3 * i], sizetbl[3 * i + 1], sizetbl[3 * i + 2], 0])
                    as usize)
                    << 3;
            match sa_decode(&buf) {
                Some(mut d) => {
                    if d.len() < want {
                        d.resize(want, 0);
                    }
                    d
                }
                None => {
                    bad += 1;
                    buf
                }
            }
        } else {
            buf
        };
        let rel = match to_name_index(i).and_then(name) {
            Some(s) => s
                .replace("host0:/ffx/master/", "")
                .replace("host0:/ffx/", "_ffx/")
                .replace("host0:", "_host0/")
                .trim_start_matches('/')
                .to_string(),
            None => format!("_unnamed/{i:05}.bin"),
        };
        let p = out.join(&rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, &data).unwrap();
        writeln!(
            idx,
            "{i}\t{st}\t{}\t{}\t{rel}",
            stored,
            if compressed { data.len() as i64 } else { -1 }
        )
        .unwrap();
        ok += 1;
    }
    eprintln!("files {ok}, undecodable {bad}");
}
