//! The records of the `MS Raw Data` stream of a `.qgd` (GC/MS) and of the `Mass Raw Data` storage of a `.lcd` (LCMS-2020 single quadrupole): one record per scan, found by the offsets of
//! `Spectrum Index`, each a header and `npk` points of a `u16` m/z (m/z × 20) and an intensity of `w` little-endian bytes. A `.qgd` MRM record has two `u16` before the intensity: Q1 and Q3
//! (both × 20). The layout is read from the files (docs/agenti/shimadzu.md); a record whose length is not `head + npk × size` is not decoded.

/// The widest intensity (bytes) a record may use.
const W_MAX: usize = 8;

/// How one record is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Forma {
    /// Bytes of the header.
    pub testa: usize,
    /// Bytes of an intensity.
    pub w: usize,
    pub npk: usize,
    /// Points are (Q1, Q3, intensity): MRM of a triple quadrupole.
    pub mrm: bool,
    /// The event (`.lcd`: segment of the method that made the scan), 0 when the record has none.
    pub ev: u16,
}

impl Forma {
    /// Bytes of one point.
    fn punto(&self) -> usize {
        (if self.mrm { 4 } else { 2 }) + self.w
    }
}

fn u16_at(r: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(r.get(at..at + 2)?.try_into().ok()?))
}

fn valida(r: &[u8], f: Forma) -> Option<Forma> {
    let ok = (1..=W_MAX).contains(&f.w)
        && f.npk.checked_mul(f.punto())?.checked_add(f.testa)? == r.len();
    ok.then_some(f)
}

/// The layout of a `.qgd` record: `kind` (the width of the intensity, +100 for MRM) and the point count sit at `ko` (20 in the files with a 32-bit index, 24 in the others).
pub(super) fn forma_gc(r: &[u8], ko: usize) -> Option<Forma> {
    let (kind, npk) = (usize::from(u16_at(r, ko)?), usize::from(u16_at(r, ko + 2)?));
    let mrm = kind >= 100;
    valida(
        r,
        Forma {
            testa: 32,
            w: if mrm { kind - 100 } else { kind },
            npk,
            mrm,
            ev: 0,
        },
    )
}

/// The layout of a record of `Mass Raw Data`: event at 8, width of the intensity at 52, point count at 54, points from 64.
pub(super) fn forma_lc(r: &[u8]) -> Option<Forma> {
    valida(
        r,
        Forma {
            testa: 64,
            w: usize::from(u16_at(r, 52)?),
            npk: usize::from(u16_at(r, 54)?),
            mrm: false,
            ev: u16_at(r, 8)?,
        },
    )
}

/// Point `j` of a record already validated by `forma_*`: (m/z or Q1 × 20, Q3 × 20 for MRM, intensity).
pub(super) fn punto(r: &[u8], f: &Forma, j: usize) -> (u16, Option<u16>, u64) {
    let p = f.testa + j * f.punto();
    let a = u16_at(r, p).unwrap_or(0);
    let (q3, y0) = if f.mrm {
        (u16_at(r, p + 2), p + 4)
    } else {
        (None, p + 2)
    };
    let mut y = 0u64;
    for (i, b) in r.get(y0..y0 + f.w).unwrap_or(&[]).iter().enumerate() {
        y |= u64::from(*b) << (8 * i);
    }
    (a, q3, y)
}

/// A hash of the m/z list of a record (to tell SIM, whose list repeats, from scans).
pub(super) fn impronta(r: &[u8], f: &Forma) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for j in 0..f.npk {
        h = (h ^ u64::from(punto(r, f, j).0)).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The offsets of `Spectrum Index`: `n` 32-bit values, or 2 bytes and `n` 64-bit values (newer `.qgd`).
pub(super) fn offset(idx: &[u8], n: usize) -> Option<Vec<u64>> {
    if idx.len() == 4 * n {
        Some(
            idx.chunks_exact(4)
                .map(|c| u64::from(u32::from_le_bytes([c[0], c[1], c[2], c[3]])))
                .collect(),
        )
    } else if idx.len() == 8 * n + 2 {
        Some(
            idx[2..]
                .chunks_exact(8)
                .map(|c| u64::from_le_bytes(c.try_into().unwrap_or([0; 8])))
                .collect(),
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gc_and_lc_records() {
        let mut r = vec![0u8; 32];
        r[20..22].copy_from_slice(&2u16.to_le_bytes());
        r[22..24].copy_from_slice(&2u16.to_le_bytes());
        for (mz, y) in [(2000u16, 7u16), (2100, 9)] {
            r.extend_from_slice(&mz.to_le_bytes());
            r.extend_from_slice(&y.to_le_bytes());
        }
        let f = forma_gc(&r, 20).expect("gc record");
        assert_eq!((f.w, f.npk, f.mrm), (2, 2, false));
        assert_eq!(punto(&r, &f, 1), (2100, None, 9));
        assert!(forma_gc(&r[..r.len() - 1], 20).is_none());
        assert_eq!(offset(&[1, 0, 0, 0, 5, 0, 0, 0], 2), Some(vec![1, 5]));
        assert_eq!(offset(&[0; 7], 2), None);
    }
}
