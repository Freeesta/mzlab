//! Small helpers of `tdf`: the folder of a member name and the sum of the points of a spectrum.
use super::blocco::Picchi;
use mzlab_core::error::Error;

/// The folder part of a member name; both `/` and `\` separate (a folder picked on Windows names its members with `\`).
pub(super) fn cartella(nome: &str) -> Option<&str> {
    nome.rsplit_once(['/', '\\']).map(|x| x.0)
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
}
