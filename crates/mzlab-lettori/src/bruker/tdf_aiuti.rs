//! Small helpers of `tdf`: the folder of a member name, the sum of the points of a spectrum, the frames a cut copy of `*_bin` lacks, and the recognition of a folder.
use super::blocco::Picchi;
use mzlab_core::error::Error;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{Fiducia, FileSet, Membro};

/// The folder part of a member name; both `/` and `\` separate (a folder picked on Windows names its members with `\`).
pub(super) fn cartella(nome: &str) -> Option<&str> {
    nome.rsplit_once(['/', '\\']).map(|x| x.0)
}

/// For each frame offset in `pos`, whether its block does not end inside the `*_bin` of `dim` bytes (a copy cut short). Blocks follow one another, so a block ends
/// where a later offset starts: only the last block that starts inside the file has its header read (`tsf`: two sizes after it; TDF: the length counts the header).
pub(super) fn tagliati(pos: &[i64], bin: &mut dyn ReadAt, dim: u64, tsf: bool) -> Vec<bool> {
    let dentro = |p: i64| {
        u64::try_from(p)
            .ok()
            .filter(|&p| p.checked_add(8).is_some_and(|e| e <= dim))
    };
    let Some(ultimo) = pos.iter().filter_map(|&p| dentro(p)).max() else {
        return vec![true; pos.len()];
    };
    let mut h = [0u8; 8];
    // a header that cannot be read is left to the spectrum, which says why
    let ultimo_tagliato = bin.read_at(ultimo, &mut h).is_ok() && {
        let a = u64::from(u32::from_le_bytes([h[0], h[1], h[2], h[3]]));
        let b = u64::from(u32::from_le_bytes([h[4], h[5], h[6], h[7]]));
        let fine = if tsf {
            ultimo + 8 + a + b
        } else {
            ultimo + a.max(8)
        };
        fine > dim
    };
    pos.iter()
        .map(|&p| match dentro(p) {
            None => true,
            Some(p) => p == ultimo && ultimo_tagliato,
        })
        .collect()
}

pub(super) fn indice_cattivo(k: impl std::fmt::Display) -> Error {
    Error::new("error.badIndex").with("k", k)
}

/// The sum of the intensities of the points of `p` in the scan ranges `scan`: the counts are whole numbers, so the order of the sum does not matter.
pub(super) fn somma_scan(p: &Picchi, scan: &[(u32, u32)]) -> f64 {
    let t: u64 = (0..p.len())
        .filter(|&i| scan.iter().any(|&(a, b)| p.scan[i] >= a && p.scan[i] < b))
        .map(|i| u64::from(p.int[i]))
        .sum();
    t as f64
}

const MAGIA: &[u8; 16] = b"SQLite format 3\0";

/// A folder is Bruker's when its database starts with the SQLite magic and the file of the peaks is beside it; a folder with only `analysis.baf` is recognised to say why it does not open.
pub(super) fn riconosci(fs: &mut FileSet, base: &str) -> Fiducia {
    let Some(i) = fs.trova(base) else {
        return if fs.trova("analysis.baf").is_some()
            && fs.trova("analysis.tdf").is_none()
            && fs.trova("analysis.tsf").is_none()
        {
            Fiducia::Forse
        } else {
            Fiducia::No
        };
    };
    if fs.testa(i, 16) != MAGIA || fs.trova(&format!("{base}_bin")).is_none() {
        return Fiducia::No;
    }
    Fiducia::Si
}

pub(super) fn prendi(fs: &mut FileSet, suffisso: &str) -> Option<Membro> {
    let i = fs.trova(suffisso)?;
    Some(fs.membri.remove(i))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bruker::spettri::somma;

    #[test]
    fn the_folder_of_a_member_name_splits_on_both_separators() {
        assert_eq!(cartella("campione.d/analysis.tdf"), Some("campione.d"));
        assert_eq!(cartella("campione.d\\analysis.tdf"), Some("campione.d"));
        assert_eq!(cartella("a\\b.d/analysis.tdf"), Some("a\\b.d"));
        assert_eq!(cartella("analysis.tdf"), None);
    }

    #[test]
    fn the_sum_of_the_points_is_the_sum_of_the_merged_spectrum() {
        let p = Picchi {
            scan: vec![0, 1, 2, 2, 3, 5, 9],
            tof: vec![10, 10, 11, 12, 10, 13, 14],
            int: vec![4, 5, 6, 7, 8, 9, u32::MAX],
        };
        for scan in [vec![(0, 3)], vec![(1, 2), (3, 6)], vec![(0, 100)], vec![]] {
            let merged: f64 = somma(&p, &scan).iter().map(|x| x.1 as f64).sum();
            assert_eq!(somma_scan(&p, &scan), merged);
        }
    }

    #[test]
    fn a_block_cut_by_the_end_of_the_file_is_told_apart() {
        // TDF: blocks of 20 bytes (the length counts the header); TSF: 8 bytes of header and 12 of payload
        let mut tdf = Vec::new();
        let mut tsf = Vec::new();
        for _ in 0..3 {
            tdf.extend(20u32.to_le_bytes());
            tdf.extend([0u8; 16]);
            tsf.extend(4u32.to_le_bytes());
            tsf.extend(8u32.to_le_bytes());
            tsf.extend([0u8; 12]);
        }
        let pos = [0, 20, 40, -1];
        for (b, t) in [(tdf, false), (tsf, true)] {
            for (dim, attesi) in [
                (60, [false, false, false, true]),
                (59, [false, false, true, true]),
                (44, [false, false, true, true]),
                (40, [false, false, true, true]),
                (39, [false, true, true, true]),
            ] {
                let mut c = b[..dim].to_vec();
                assert_eq!(tagliati(&pos, &mut c, dim as u64, t), attesi, "{t} {dim}");
            }
        }
    }
}
