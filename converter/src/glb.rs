//! A minimal binary glTF writer for triangle meshes, in the shape
//! `ffx_phyre::import_into_cluster` reads: one glTF mesh per HD instance,
//! its primitives tagged with `extras.ffx_segment` (or untagged for new
//! segments, copies of the tagged one), joints named like the model's
//! skeleton nodes.

use serde_json::{Value, json};

#[derive(Debug, Clone, Default)]
pub struct Primitive {
    /// The HD segment it replaces; `None` adds a segment.
    pub segment: Option<usize>,
    /// The model's material index (picks the template of a new segment).
    pub material: usize,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// glTF convention: v down from the top of the image.
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[u8; 4]>,
    /// Indices into [`Glb::joints`]; empty for a rigid segment.
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct Glb {
    /// Joint node names (`SkeletonNode_i`).
    pub joints: Vec<String>,
    /// One glTF mesh per HD instance.
    pub meshes: Vec<Vec<Primitive>>,
    /// Number of materials to declare (the model's).
    pub materials: usize,
}

struct Buf {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Buf {
    fn add(&mut self, bytes: &[u8], ctype: u32, kind: &str, count: usize, normalized: bool, minmax: Option<(Value, Value)>) -> usize {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
        let view = self.views.len();
        self.views.push(json!({"buffer": 0, "byteOffset": self.bin.len(), "byteLength": bytes.len()}));
        self.bin.extend_from_slice(bytes);
        let mut acc = json!({"bufferView": view, "componentType": ctype, "count": count, "type": kind});
        if normalized {
            acc["normalized"] = json!(true);
        }
        if let Some((lo, hi)) = minmax {
            acc["min"] = lo;
            acc["max"] = hi;
        }
        self.accessors.push(acc);
        self.accessors.len() - 1
    }
}

fn f32s<const N: usize>(v: &[[f32; N]]) -> Vec<u8> {
    v.iter().flat_map(|a| a.iter().flat_map(|x| x.to_le_bytes())).collect()
}

impl Glb {
    pub fn write(&self) -> Vec<u8> {
        let mut buf = Buf {
            bin: Vec::new(),
            views: Vec::new(),
            accessors: Vec::new(),
        };
        let mut meshes = Vec::new();
        for mesh in &self.meshes {
        let mut prims = Vec::new();
        for p in mesh {
            let n = p.positions.len();
            let mut lo = [f32::MAX; 3];
            let mut hi = [f32::MIN; 3];
            for q in &p.positions {
                for k in 0..3 {
                    lo[k] = lo[k].min(q[k]);
                    hi[k] = hi[k].max(q[k]);
                }
            }
            let pos = buf.add(&f32s(&p.positions), 5126, "VEC3", n, false, Some((json!(lo), json!(hi))));
            let nor = buf.add(&f32s(&p.normals), 5126, "VEC3", n, false, None);
            let uv = buf.add(&f32s(&p.uvs), 5126, "VEC2", n, false, None);
            let col: Vec<u8> = p.colors.iter().flatten().copied().collect();
            let col = buf.add(&col, 5121, "VEC4", n, true, None);
            let ib: Vec<u8> = p.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
            let ind = buf.add(&ib, 5125, "SCALAR", p.indices.len(), false, None);
            let mut attributes = json!({"POSITION": pos, "NORMAL": nor, "TEXCOORD_0": uv, "COLOR_0": col});
            if !p.joints.is_empty() {
                let jb: Vec<u8> = p.joints.iter().flat_map(|j| j.iter().flat_map(|x| x.to_le_bytes())).collect();
                attributes["JOINTS_0"] = json!(buf.add(&jb, 5123, "VEC4", n, false, None));
                attributes["WEIGHTS_0"] = json!(buf.add(&f32s(&p.weights), 5126, "VEC4", n, false, None));
            }
            let mut prim = json!({
                "attributes": attributes,
                "indices": ind,
                "material": p.material,
                "mode": 4,
            });
            if let Some(s) = p.segment {
                prim["extras"] = json!({"ffx_segment": s});
            }
            prims.push(prim);
        }
        meshes.push(json!({"name": format!("ps2mesh{}", meshes.len()), "primitives": prims}));
        }
        let mut nodes: Vec<Value> = self.joints.iter().map(|n| json!({"name": n})).collect();
        let joint_ids: Vec<usize> = (0..self.joints.len()).collect();
        let mut roots = Vec::new();
        for (i, mesh) in self.meshes.iter().enumerate() {
            let mut node = json!({"name": format!("ps2mesh{i}"), "mesh": i});
            if mesh.iter().any(|p| !p.joints.is_empty()) {
                node["skin"] = json!(0);
            }
            roots.push(nodes.len());
            nodes.push(node);
        }
        let materials: Vec<Value> = (0..self.materials).map(|i| json!({"name": format!("m{i}")})).collect();
        let doc = json!({
            "asset": {"version": "2.0", "generator": "demaster"},
            "scene": 0,
            "scenes": [{"nodes": roots}],
            "nodes": nodes,
            "skins": [{"joints": joint_ids}],
            "meshes": meshes,
            "materials": materials,
            "buffers": [{"byteLength": buf.bin.len()}],
            "bufferViews": buf.views,
            "accessors": buf.accessors,
        });
        let mut js = serde_json::to_vec(&doc).expect("json");
        while js.len() % 4 != 0 {
            js.push(b' ');
        }
        while buf.bin.len() % 4 != 0 {
            buf.bin.push(0);
        }
        let total = 12 + 8 + js.len() + 8 + buf.bin.len();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&(js.len() as u32).to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&js);
        out.extend_from_slice(&(buf.bin.len() as u32).to_le_bytes());
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&buf.bin);
        out
    }
}
