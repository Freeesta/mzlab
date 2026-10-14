//! The number type of a binary data array of an mzML (floats and integers, 32 and 64 bits).

/// MS:1000521 / 1000523 floats, MS:1000519 / 1000522 integers.
#[derive(PartialEq, Clone, Copy)]
pub enum Num {
    F32,
    F64,
    I32,
    I64,
}

impl Num {
    pub fn width(self) -> usize {
        match self {
            Num::F32 | Num::I32 => 4,
            Num::F64 | Num::I64 => 8,
        }
    }

    /// One little-endian value of `width()` bytes as `f64`.
    pub fn value(self, c: &[u8]) -> f64 {
        match (self, c) {
            (Num::F32, [a, b, c, d, ..]) => f32::from_le_bytes([*a, *b, *c, *d]) as f64,
            (Num::I32, [a, b, c, d, ..]) => i32::from_le_bytes([*a, *b, *c, *d]) as f64,
            (Num::F64, [a, b, c, d, e, f, g, h]) => {
                f64::from_le_bytes([*a, *b, *c, *d, *e, *f, *g, *h])
            }
            (Num::I64, [a, b, c, d, e, f, g, h]) => {
                i64::from_le_bytes([*a, *b, *c, *d, *e, *f, *g, *h]) as f64
            }
            _ => 0.0,
        }
    }

    /// The type named by the accession of a binary data array, if it names one.
    pub fn of_accession(acc: &str) -> Option<Num> {
        match acc {
            "MS:1000521" => Some(Num::F32),
            "MS:1000523" => Some(Num::F64),
            "MS:1000519" => Some(Num::I32),
            "MS:1000522" => Some(Num::I64),
            _ => None,
        }
    }
}
