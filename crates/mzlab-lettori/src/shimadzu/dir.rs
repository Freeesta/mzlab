//! The names in the directory of an OLE2 container, read from a few sectors of the file (never the whole of it): what `riconosci` looks at, so that a Shimadzu file is told by its storages
//! (`GCMS Raw Data`, `Mass Raw Data`, `TLM Raw Data`, `QTFL RawData`) and not by its extension.

use mzlab_core::lettori::Membro;

const MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
/// The most directory sectors followed (each holds 4 or 32 entries).
const SETTORI_MAX: usize = 256;
/// The first FAT sectors the header lists.
const DIFAT: usize = 109;

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn settore(m: &mut Membro, ss: usize, id: u32) -> Option<Vec<u8>> {
    let off = (u64::from(id) + 1).checked_mul(ss as u64)?;
    if off + ss as u64 > m.dim {
        return None;
    }
    let mut b = vec![0u8; ss];
    m.dati.read_at(off, &mut b).ok()?;
    Some(b)
}

/// The names of the entries (storages and streams) of the file, `None` when it is not an OLE2 container or its directory cannot be followed.
pub(super) fn nomi(m: &mut Membro) -> Option<Vec<String>> {
    let mut h = [0u8; 512];
    if m.dim < 512 || m.dati.read_at(0, &mut h).is_err() || h[..8] != MAGIC {
        return None;
    }
    let ss = match u16::from_le_bytes([h[0x1E], h[0x1F]]) {
        9 => 512,
        12 => 4096,
        _ => return None,
    };
    let per = ss / 4;
    let mut dir = u32_at(&h, 0x30)?;
    let mut fat: Vec<Option<Vec<u8>>> = vec![None; DIFAT];
    let mut out = Vec::new();
    for _ in 0..SETTORI_MAX {
        if dir >= 0xFFFF_FFFA {
            break;
        }
        let s = settore(m, ss, dir)?;
        for e in s.chunks_exact(128) {
            let len = usize::from(u16::from_le_bytes([e[64], e[65]])).min(64);
            if len >= 2 && matches!(e[66], 1 | 2 | 5) {
                let u: Vec<u16> = e[..len - 2]
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                out.push(String::from_utf16_lossy(&u));
            }
        }
        let (fi, at) = ((dir as usize) / per, (dir as usize) % per);
        if fi >= DIFAT {
            break;
        }
        if fat[fi].is_none() {
            let sid = u32_at(&h, 0x4C + 4 * fi)?;
            fat[fi] = Some(settore(m, ss, sid)?);
        }
        dir = u32_at(fat[fi].as_ref()?, 4 * at)?;
    }
    Some(out)
}
