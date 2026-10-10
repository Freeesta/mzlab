//! CRC32C, LEB128 varints and bounded allocation helpers.

use crate::error::{Error, Result};

const fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ 0x82F6_3B78
            } else {
                c >> 1
            };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}
static TABLE: [u32; 256] = crc_table();

/// CRC32C (Castagnoli), as in iSCSI: `crc32c(b"123456789") == 0xE3069283`.
pub fn crc32c(data: &[u8]) -> u32 {
    crc32c_update(0, data)
}

/// Continues a CRC32C: `crc32c_update(crc32c(a), b) == crc32c(a ++ b)`.
pub fn crc32c_update(prev: u32, data: &[u8]) -> u32 {
    let mut c = !prev;
    for &b in data {
        c = TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

pub fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

/// Reads a varint at `*pos`; an incomplete or overlong one is an error.
pub fn get_varint(buf: &[u8], pos: &mut usize) -> Result<u64> {
    let mut v = 0u64;
    for shift in (0..70).step_by(7) {
        let b = *buf.get(*pos).ok_or_else(|| Error::corrupt("varint"))?;
        *pos += 1;
        if shift == 63 && b > 1 {
            return Err(Error::corrupt("varint"));
        }
        v |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 == 0 {
            return Ok(v);
        }
    }
    Err(Error::corrupt("varint"))
}

/// `Vec` with room for `n` items, or the memory-budget error (a declared size is never trusted).
pub fn vec_with_capacity<T>(n: usize) -> Result<Vec<T>> {
    let mut v = Vec::new();
    v.try_reserve_exact(n).map_err(|_| Error::memory_budget())?;
    Ok(v)
}

/// A zeroed buffer of `n` bytes, or the memory-budget error.
pub fn zeroed(n: usize) -> Result<Vec<u8>> {
    let mut v = vec_with_capacity::<u8>(n)?;
    v.resize(n, 0);
    Ok(v)
}

pub fn le_u16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}
pub fn le_u32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}
pub fn le_u64(b: &[u8], at: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(a)
}
pub fn le_f32(b: &[u8], at: usize) -> f32 {
    f32::from_bits(le_u32(b, at))
}
pub fn le_f64(b: &[u8], at: usize) -> f64 {
    f64::from_bits(le_u64(b, at))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32c_check_value() {
        assert_eq!(crc32c(b"123456789"), 0xE306_9283);
        assert_eq!(crc32c(b""), 0);
        assert_eq!(crc32c_update(crc32c(b"1234"), b"56789"), 0xE306_9283);
    }

    #[test]
    fn varint_round_trip_and_garbage() {
        for v in [0u64, 1, 127, 128, 300, 1 << 35, u64::MAX] {
            let mut b = Vec::new();
            put_varint(&mut b, v);
            let mut p = 0;
            assert_eq!(get_varint(&b, &mut p).unwrap(), v);
            assert_eq!(p, b.len());
        }
        let mut p = 0;
        assert!(get_varint(&[0x80, 0x80], &mut p).is_err());
        let mut p = 0;
        assert!(get_varint(&[0xFF; 12], &mut p).is_err());
    }
}
