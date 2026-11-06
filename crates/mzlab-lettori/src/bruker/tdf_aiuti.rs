//! Small helpers of `tdf`: the folder of a member name and the sum of the points of a spectrum.
use super::blocco::Picchi;
use super::spettri::tratti;
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
    let t: u64 = tratti(p, scan)
        .into_iter()
        .flat_map(|r| p.int[r].iter().map(|&y| u64::from(y)))
        .sum();
    t as f64
}

#[cfg(test)]
#[path = "tdf_aiuti_prove.rs"]
mod tests;
