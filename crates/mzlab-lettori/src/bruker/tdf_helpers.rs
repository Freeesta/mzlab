//! Small helpers of `tdf`: the folder of a member name, the sum of the points of a spectrum, the frames a cut copy of `*_bin` lacks, and the recognition of a folder.
use super::block::Peaks;
use super::spectra::stretches;
use mzlab_core::error::Error;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{Fiducia, FileSet, Member};

/// The folder part of a member name; both `/` and `\` separate (a folder picked on Windows names its members with `\`).
pub(super) fn cartella(name: &str) -> Option<&str> {
    name.rsplit_once(['/', '\\']).map(|x| x.0)
}

/// For each frame offset in `pos`, whether its block does not end inside the `*_bin` of `dim` bytes (a copy cut short). Blocks follow one another, so a block ends
/// where a later offset starts: only the last block that starts inside the file has its header read (`tsf`: two sizes after it; TDF: the length counts the header).
pub(super) fn cut_ones(pos: &[i64], bin: &mut dyn ReadAt, dim: u64, tsf: bool) -> Vec<bool> {
    let inside = |p: i64| {
        u64::try_from(p)
            .ok()
            .filter(|&p| p.checked_add(8).is_some_and(|e| e <= dim))
    };
    let Some(last_one) = pos.iter().filter_map(|&p| inside(p)).max() else {
        return vec![true; pos.len()];
    };
    let mut h = [0u8; 8];
    // a header that cannot be read is left to the spectrum, which says why
    let last_cut = bin.read_at(last_one, &mut h).is_ok() && {
        let a = u64::from(u32::from_le_bytes([h[0], h[1], h[2], h[3]]));
        let b = u64::from(u32::from_le_bytes([h[4], h[5], h[6], h[7]]));
        let fine = if tsf {
            last_one + 8 + a + b
        } else {
            last_one + a.max(8)
        };
        fine > dim
    };
    pos.iter()
        .map(|&p| match inside(p) {
            None => true,
            Some(p) => p == last_one && last_cut,
        })
        .collect()
}

pub(super) fn bad_index(k: impl std::fmt::Display) -> Error {
    Error::new("error.badIndex").with("k", k)
}

/// The sum of the intensities of the points of `p` in the scan ranges `scan`: the counts are whole numbers, so the order of the sum does not matter.
pub(super) fn sum_scan(p: &Peaks, scan: &[(u32, u32)]) -> f64 {
    let t: u64 = stretches(p, scan)
        .into_iter()
        .flat_map(|r| p.int[r].iter().map(|&y| u64::from(y)))
        .sum();
    t as f64
}

const MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// A folder is Bruker's when its database starts with the SQLite magic and the file of the peaks is beside it; a folder with only `analysis.baf` is recognised to say why it does not open.
pub(super) fn recognize(fs: &mut FileSet, base: &str) -> Fiducia {
    let bin = fs.find(&format!("{base}_bin")).is_some();
    let Some(i) = fs.find(base) else {
        // a `*_bin` without its database is this reader's folder, refused by `apri` with the file it lacks
        return if bin
            || (fs.find("analysis.baf").is_some()
                && fs.find("analysis.tdf").is_none()
                && fs.find("analysis.tsf").is_none())
        {
            Fiducia::Forse
        } else {
            Fiducia::No
        };
    };
    // a database without its `*_bin`, or one that is not SQLite (an empty or cut copy): `apri` says which
    if fs.testa(i, 16) != MAGIC || !bin {
        return Fiducia::Forse;
    }
    Fiducia::Si
}

/// The error of a folder that lacks `file` (`analysis.tsf`, `analysis.tdf_bin`, ...); `nome`: a member of the folder.
pub(super) fn absent(name: &str, file: &str) -> Error {
    Error::new("err.file.brukerNoBin")
        .with("name", cartella(name).unwrap_or(name))
        .with("file", file)
}

pub(super) fn take(fs: &mut FileSet, suffix: &str) -> Option<Member> {
    let i = fs.find(suffix)?;
    Some(fs.membri.remove(i))
}

#[cfg(test)]
#[path = "tdf_helpers_tests.rs"]
mod tests;
