//! The `SampleSubtree/SampleN/Idx` stream: a 32-byte header and 54-byte records, one per scan block of the `.wiff.scan`.
//! Layout after OpenSXRaw (Apache-2.0, `src/raw/idx.rs`): u32 offset, u32 size, u32 ?, f32 RT (min), u8 level flag (1 = MS1),
//! u8 ?, f64 TIC (cps), f64 grid field. Records of size <= 56 are placeholders.

const HEADER: usize = 32;
const RECORD: usize = 54;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Record {
    pub offset: u32,
    pub size: u32,
    /// Retention time in minutes.
    pub rt: f32,
    pub level: u8,
    /// Total ion current of the index, in cps (not the sum of the raw counts).
    pub tic: f64,
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

pub fn parse(data: &[u8]) -> Option<Vec<Record>> {
    let body = data.get(HEADER..)?;
    let mut out = Vec::with_capacity(body.len() / RECORD / 4);
    for r in body.chunks_exact(RECORD) {
        let size = u32_at(r, 4);
        if size <= 56 {
            continue;
        }
        let mut t = [0u8; 8];
        t.copy_from_slice(&r[0x12..0x1A]);
        out.push(Record { offset: u32_at(r, 0), size, rt: f32_at(r, 0x0C), level: if r[0x10] == 1 { 1 } else { 2 }, tic: f64::from_le_bytes(t) });
    }
    Some(out)
}
