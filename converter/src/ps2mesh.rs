//! PS2 character meshes (`.chr` entry 0, the model header): skeleton,
//! models (vertices, normals, triangle groups) and the skin lists that say
//! which bones move each vertex. Layout: ffx-rust `docs/formats/chara.md`,
//! `motion.md`; `docs/models.md` here.
//!
//! - Header: `u16` version `+4`, model count `+6`, skin entry count `+8`,
//!   bone count `+0xa`; model table `+0x10`, skin entries `+0x14`, bone
//!   table `+0x1c` (offsets relative to the header).
//! - Model entry (24 bytes for versions `0x831..0x1314`, else 40): `u16`
//!   base bone `+0`, vertex count `+4`, group count `+6`, vertices `+8`
//!   (`i16` x, y, z, in the base bone's frame), normals `+0xc` (count
//!   `+0x16`), groups `+0x10` (12 bytes each), cloth count `+0x18`, cloth
//!   table `+0x1c` (`0x24` bytes each: `u16` pinned vertex count `+0`,
//!   moving count `+2`, vertex list `+8`, total `+0x1c` = pinned + moving;
//!   the list holds the pinned vertices, then the moving ones, each run
//!   sorted; `Sg_ClothStep` copies the pinned ones from the model's
//!   vertices every step and moves the others).
//! - Group: `u8` flags `+0` (bit 0 strips), `u8` `+1` (MAT), `u8` texture
//!   list index `+2`, `u16` count `+4`, data `+8`. Points are 4 `u16`
//!   (vertex, normal, u, v); a vertex or normal with bit 15 set repeats an
//!   earlier halfword (`Ch_EffectPacketCreateMain`, FFX.exe `0x00842c80`).
//! - Skin entry (12 bytes): `u16` bone, model, base bone, kind; lists `+8`.
//!   The lists (`sg_calcenvelope_asm` `0x006ee570`, weighted
//!   `sg_calcenvelopeW_asm` `0x006ed9e0`) hold positions in the bone's
//!   frame and the vertex they move.

use anyhow::{Context, Result, ensure};

fn u8_(b: &[u8], o: usize) -> Result<u8> {
    b.get(o).copied().context("chr: read past the end")
}
fn u16_(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        b.get(o..o + 2).context("chr: read past the end")?.try_into()?,
    ))
}
fn i16_(b: &[u8], o: usize) -> Result<i16> {
    Ok(u16_(b, o)?.cast_signed())
}
fn u32_(b: &[u8], o: usize) -> Result<usize> {
    Ok(u32::from_le_bytes(
        b.get(o..o + 4).context("chr: read past the end")?.try_into()?,
    ) as usize)
}

/// One bone of the bind pose (`SG_InitBones`).
#[derive(Debug, Clone, Copy)]
pub struct Bone {
    pub parent: u16,
    /// 1/100 degree.
    pub rot: [i16; 3],
    /// PS2 units (the vertices' units).
    pub trans: [i16; 3],
    /// 1/4096.
    pub scale: [i16; 3],
}

/// A triangle corner: indices into its model's vertices and normals, and
/// the UV in 1/16 texel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Corner {
    pub vertex: u16,
    pub normal: u16,
    pub uv: [u16; 2],
}

#[derive(Debug, Clone, Copy)]
pub struct Tri {
    /// Texture-list index.
    pub texture: u8,
    /// The group's flag byte.
    pub flags: u8,
    pub corners: [Corner; 3],
}

#[derive(Debug, Clone, Default)]
pub struct Cloth {
    /// Model vertices of the piece: the first `pinned` follow the skin
    /// (`Sg_ClothStep` copies them from the model's vertices every step),
    /// the rest are simulated (the moving ones: Wakka's shin piece moves
    /// only the 4 vertices of its ankle flaps).
    pub vertices: Vec<u16>,
    pub pinned: usize,
}

impl Cloth {
    /// The vertices the simulation moves.
    pub fn moving(&self) -> &[u16] {
        &self.vertices[self.pinned.min(self.vertices.len())..]
    }
}

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub base: u16,
    pub vertices: Vec<[i16; 3]>,
    pub normals: Vec<[i16; 3]>,
    pub tris: Vec<Tri>,
    pub cloths: Vec<Cloth>,
    /// Per vertex: (bone, weight, position in the bone's frame). Empty for
    /// a vertex no skin list names: it follows the base bone.
    pub influences: Vec<Vec<(u16, f32, [i16; 3])>>,
    /// Corners dropped for an index past the arrays.
    pub bad_corners: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub version: u16,
    pub bones: Vec<Bone>,
    pub models: Vec<Model>,
}

/// Reads point `p` (4 halfwords at byte `at`), resolving back references.
fn point(b: &[u8], at: usize) -> Result<Corner> {
    let back = |w: u16| usize::from(w & 0x7fff) * 4 / 3;
    let mut vertex = u16_(b, at)?;
    if vertex & 0x8000 != 0 {
        vertex = u16_(b, at.checked_sub(2 * back(vertex)).context("back ref")?)?;
    }
    let mut normal = u16_(b, at + 2)?;
    if normal & 0x8000 != 0 {
        normal = u16_(b, (at + 2).checked_sub(4 + 2 * back(normal)).context("back ref")?)?;
    }
    Ok(Corner {
        vertex,
        normal,
        uv: [u16_(b, at + 4)?, u16_(b, at + 6)?],
    })
}

impl Mesh {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let h = u32_(b, 0x10)?;
        ensure!(h != 0, "chr: no model header");
        let version = u16_(b, h + 4)?;
        let nmodels = usize::from(u16_(b, h + 6)?);
        let nskins = usize::from(u16_(b, h + 8)?);
        let nbones = usize::from(u16_(b, h + 0xa)?);
        let esize = if (0x831..0x1314).contains(&version) { 24 } else { 40 };
        let mut mesh = Self {
            version,
            ..Self::default()
        };
        let bt = h + u32_(b, h + 0x1c)?;
        for k in 0..nbones {
            let e = bt + 20 * k;
            let w = |j: usize| i16_(b, e + 2 + 2 * j);
            mesh.bones.push(Bone {
                parent: u16_(b, e)?,
                rot: [w(0)?, w(1)?, w(2)?],
                trans: [w(3)?, w(4)?, w(5)?],
                scale: [w(6)?, w(7)?, w(8)?],
            });
        }
        let mt = h + u32_(b, h + 0x10)?;
        for m in 0..nmodels {
            let e = mt + esize * m;
            let base = u16_(b, e)?;
            let nv = usize::from(u16_(b, e + 4)?);
            let ng = usize::from(u16_(b, e + 6)?);
            let vp = h + u32_(b, e + 8)?;
            let np = h + u32_(b, e + 0xc)?;
            let gp = h + u32_(b, e + 0x10)?;
            let nn = if esize == 40 { usize::from(u16_(b, e + 0x16)?) } else { 0 };
            let triple = |at: usize| -> Result<[i16; 3]> {
                Ok([i16_(b, at)?, i16_(b, at + 2)?, i16_(b, at + 4)?])
            };
            let mut model = Model {
                base,
                vertices: (0..nv).map(|v| triple(vp + 6 * v)).collect::<Result<_>>()?,
                normals: (0..nn).map(|v| triple(np + 6 * v)).collect::<Result<_>>()?,
                influences: vec![Vec::new(); nv],
                ..Model::default()
            };
            let push = |model: &mut Model, texture: u8, flags: u8, c: [Corner; 3]| {
                if c.iter().any(|c| {
                    usize::from(c.vertex) >= nv || (nn > 0 && usize::from(c.normal) >= nn)
                }) {
                    model.bad_corners += 1;
                    return;
                }
                model.tris.push(Tri {
                    texture,
                    flags,
                    corners: c,
                });
            };
            for g in 0..ng {
                let ge = gp + 12 * g;
                let flags = u8_(b, ge)?;
                let texture = u8_(b, ge + 2)?;
                let count = usize::from(u16_(b, ge + 4)?);
                let mut q = h + u32_(b, ge + 8)?;
                if flags & 1 != 0 {
                    for _ in 0..count {
                        let size = usize::from(u8_(b, q)?) << 4;
                        let mut at = q + usize::from(u8_(b, q + 1)?);
                        let strips = usize::from(u8_(b, q + 2)?);
                        for s in 0..strips {
                            let len = usize::from(u8_(b, q + 4 + s)?);
                            let mut c: Vec<Corner> = Vec::with_capacity(len);
                            for k in 0..len {
                                c.push(point(b, at)?);
                                at += 8;
                                // FUN_00844370: odd k (a, b, c) = (k-2, k-1, k),
                                // even k (k-1, k-2, k).
                                if k >= 2 {
                                    let t = if k % 2 == 1 {
                                        [c[k - 2], c[k - 1], c[k]]
                                    } else {
                                        [c[k - 1], c[k - 2], c[k]]
                                    };
                                    push(&mut model, texture, flags, t);
                                }
                            }
                        }
                        if size == 0 {
                            break;
                        }
                        q += size;
                    }
                } else {
                    for _ in 0..count / 3 {
                        let t = [point(b, q)?, point(b, q + 8)?, point(b, q + 16)?];
                        push(&mut model, texture, flags, t);
                        q += 24;
                    }
                }
            }
            let nc = if esize == 40 { usize::from(u16_(b, e + 0x18)?) } else { 0 };
            if nc > 0 {
                let ct = h + u32_(b, e + 0x1c)?;
                for c in 0..nc {
                    let ce = ct + 0x24 * c;
                    let n = usize::from(u16_(b, ce + 0x1c)?);
                    let (pinned, moving) = (usize::from(u16_(b, ce)?), usize::from(u16_(b, ce + 2)?));
                    ensure!(pinned + moving == n, "chr: cloth {c} of {pinned} pinned and {moving} moving vertices, {n} in all");
                    let lp = h + u32_(b, ce + 8)?;
                    model.cloths.push(Cloth {
                        vertices: (0..n).map(|k| u16_(b, lp + 2 * k)).collect::<Result<_>>()?,
                        pinned,
                    });
                }
            }
            mesh.models.push(model);
        }
        let st = h + u32_(b, h + 0x14)?;
        for s in 0..nskins {
            let e = st + 12 * s;
            let bone = u16_(b, e)?;
            let m = usize::from(u16_(b, e + 2)?);
            let weighted = u16_(b, e + 6)? != 0;
            let lp = u32_(b, e + 8)?;
            if lp == 0 || m >= mesh.models.len() {
                continue;
            }
            let lp = h + lp;
            let model = &mut mesh.models[m];
            let nv = model.vertices.len();
            let add = |model: &mut Model, idx: usize, w: f32, p: [i16; 3]| -> Result<()> {
                ensure!(idx < nv, "skin list names vertex {idx} of {nv}");
                model.influences[idx].push((bone, w, p));
                Ok(())
            };
            let n1 = usize::from(u16_(b, lp)?);
            for k in 0..n1 {
                let at = lp + 4 + 8 * k;
                let p = [i16_(b, at)?, i16_(b, at + 2)?, i16_(b, at + 4)?];
                add(model, usize::from(u16_(b, at + 6)?), 1.0, p)?;
            }
            let mut q = lp + 4 + 8 * n1;
            if weighted {
                // two runs of (x, y, z, vertex, weight × 10000)
                for _ in 0..2 {
                    let n = usize::from(u16_(b, q)?);
                    for k in 0..n {
                        let at = q + 2 + 10 * k;
                        let p = [i16_(b, at)?, i16_(b, at + 2)?, i16_(b, at + 4)?];
                        let w = f32::from(u16_(b, at + 8)?) * 0.0001;
                        add(model, usize::from(u16_(b, at + 6)?), w, p)?;
                    }
                    q += 2 + 10 * n;
                }
            } else {
                // two runs of blocks (count, vertex base >> 8, then
                // (x, y, z, weight byte, vertex low byte)), each ended by
                // 0xffff
                for _ in 0..2 {
                    loop {
                        let n = u16_(b, q)?;
                        if n == 0xffff {
                            q += 4;
                            break;
                        }
                        let hi = usize::from(u16_(b, q + 2)?) << 8;
                        for k in 0..usize::from(n) {
                            let at = q + 4 + 8 * k;
                            let p = [i16_(b, at)?, i16_(b, at + 2)?, i16_(b, at + 4)?];
                            let w = f32::from(u8_(b, at + 6)?) / 255.0;
                            add(model, hi + usize::from(u8_(b, at + 7)?), w, p)?;
                        }
                        q += 4 + 8 * usize::from(n);
                    }
                }
            }
        }
        Ok(mesh)
    }

    /// The bind pose's world matrices (column-major, PS2 units), from the
    /// bone table: local = T · Rz · Ry · Rx · S.
    pub fn bind_world(&self) -> Vec<[f64; 16]> {
        let mut world: Vec<Option<[f64; 16]>> = vec![None; self.bones.len()];
        fn get(bones: &[Bone], world: &mut [Option<[f64; 16]>], i: usize, depth: usize) -> [f64; 16] {
            if let Some(m) = world[i] {
                return m;
            }
            let b = bones[i];
            let local = local_matrix(&b);
            let p = usize::from(b.parent);
            let m = if p == i || p >= bones.len() || depth > bones.len() {
                local
            } else {
                mul(&get(bones, world, p, depth + 1), &local)
            };
            world[i] = Some(m);
            m
        }
        (0..self.bones.len())
            .map(|i| get(&self.bones, &mut world, i, 0))
            .collect()
    }
}

pub fn local_matrix(b: &Bone) -> [f64; 16] {
    let r = b.rot.map(|a| f64::from(a) / 100.0 * std::f64::consts::PI / 180.0);
    let (sx, cx) = r[0].sin_cos();
    let (sy, cy) = r[1].sin_cos();
    let (sz, cz) = r[2].sin_cos();
    let rx = [1.0, 0.0, 0.0, 0.0, 0.0, cx, sx, 0.0, 0.0, -sx, cx, 0.0, 0.0, 0.0, 0.0, 1.0];
    let ry = [cy, 0.0, -sy, 0.0, 0.0, 1.0, 0.0, 0.0, sy, 0.0, cy, 0.0, 0.0, 0.0, 0.0, 1.0];
    let rz = [cz, sz, 0.0, 0.0, -sz, cz, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    // the game keeps the integer part of the scale
    let s = b.scale.map(|s| f64::from(s / 4096).max(1.0));
    let sm = [s[0], 0.0, 0.0, 0.0, 0.0, s[1], 0.0, 0.0, 0.0, 0.0, s[2], 0.0, 0.0, 0.0, 0.0, 1.0];
    let mut m = mul(&rz, &mul(&ry, &mul(&rx, &sm)));
    m[12] = f64::from(b.trans[0]);
    m[13] = f64::from(b.trans[1]);
    m[14] = f64::from(b.trans[2]);
    m
}

/// Column-major 4×4 product `a · b`.
pub fn mul(a: &[f64; 16], b: &[f64; 16]) -> [f64; 16] {
    let mut m = [0f64; 16];
    for c in 0..4 {
        for r in 0..4 {
            m[c * 4 + r] = (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum();
        }
    }
    m
}

pub fn transform(m: &[f64; 16], p: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|r| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r])
}

pub fn rotate(m: &[f64; 16], p: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|r| m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2])
}
