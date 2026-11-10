//! FFX's PS2 music driver (`sqseq`, a port of `iopsound.irx`) and the SPU2
//! sound chip it plays on (`spu2`). The soundtrack job (`jobs::music`), the
//! live engine (`demaster-live`) and FFX Music Studio use them.

pub mod spu2;
pub mod sqseq;
