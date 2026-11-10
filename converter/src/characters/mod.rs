//! What the character jobs (`jobs::character_textures`,
//! `jobs::character_meshes`) share: PS2 texels baked into HD pages, material
//! alpha, texture animation, and `demaster materials-audit`.

pub mod alpha;
pub mod bake;
pub mod material_audit;
pub mod texanim;
