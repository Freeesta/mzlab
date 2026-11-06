//! Small helpers of `tdf`: the folder of a member name, the sum of the points of a spectrum, the frames a cut copy of `*_bin` lacks, and the recognition of a folder.
use super::blocco::Picchi;
use super::spettri::tratti;
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
    let t: u64 = tratti(p, scan)
        .into_iter()
        .flat_map(|r| p.int[r].iter().map(|&y| u64::from(y)))
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
#[path = "tdf_aiuti_prove.rs"]
mod tests;
