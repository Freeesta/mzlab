//! Checks that a Waters folder is not cut short: an IDX cut at a whole row, a DAT cut inside a scan, a `_FUNCTNS.INF` that lists fewer functions than the folder holds,
//! a `_HEADER.TXT` emptied or cut inside a calibration line. Each gives one keyed error or the count of scans left out, never data read short.

use super::cartella::Folder;
use super::idx::Riga;
use mzlab_core::error::{Error, Result};

/// A copy cut short in the IDX or DAT of a function. `None`: the IDX is cut at a whole row (a DAT that goes on for at least one more scan past the last row, an empty IDX
/// next to data, or rows read in the wrong variant, whose offsets go back or lie far past any DAT). Else the scans not whole in a DAT cut short are emptied (neither the
/// index nor the channels read them short) and their count is returned.
pub(super) fn tagli(
    righe: &mut [Riga],
    larg: Option<u64>,
    ha_dat: bool,
    dat_len: u64,
    ha_idx: bool,
) -> Option<usize> {
    let tail = match righe.last() {
        Some(r) => {
            let l = larg.map_or(0, |w| u64::from(r.conta).saturating_mul(w));
            l > 0 && dat_len.saturating_sub(r.offset.saturating_add(l)) >= l
        }
        None => ha_idx && dat_len > 0,
    };
    let storte = righe.windows(2).any(|w| w[1].offset < w[0].offset)
        || righe.iter().any(|r| r.offset >> 40 != 0);
    if tail || (ha_dat && storte) {
        return None;
    }
    let mut tagliate = 0;
    if let Some(w) = larg {
        for r in righe.iter_mut().filter(|r| r.conta > 0) {
            let fine = u64::from(r.conta)
                .checked_mul(w)
                .and_then(|l| r.offset.checked_add(l));
            if fine.is_none_or(|f| f > dat_len) {
                r.conta = 0;
                tagliate += 1;
            }
        }
    }
    Some(tagliate)
}

/// A `_FUNCTNS.INF` cut short at a whole record still parses: the data of the next function tell that functions are missing.
pub(super) fn whole_list(c: &Folder, n: usize) -> Result<()> {
    let dopo = n + 1;
    if [format!("_FUNC{dopo:03}.IDX"), format!("_FUNC{dopo:03}.DAT")]
        .iter()
        .any(|f| c.cerca(f).is_some())
    {
        return Err(Error::new("err.file.functionsCut")
            .with("name", &c.nome)
            .with("listed", n));
    }
    Ok(())
}

/// The calibration of every function is a line of `_HEADER.TXT`: one emptied, or cut inside a calibration line, would give other m/z unsaid.
pub(super) fn whole_header(c: &Folder, header: Option<&[u8]>) -> Result<()> {
    let tronca = |b: &[u8]| {
        b.is_empty()
            || (b.last() != Some(&b'\n')
                && b.rsplit(|&x| x == b'\n')
                    .next()
                    .is_some_and(|r| r.trim_ascii_start().starts_with(b"$$ Cal")))
    };
    if header.is_some_and(tronca) {
        return Err(nel_file(c, "err.file.headerCut", "_HEADER.TXT"));
    }
    Ok(())
}

/// The error `chiave` about one `file` of the folder, with the folder name.
pub(super) fn nel_file(c: &Folder, chiave: &'static str, file: &str) -> Error {
    Error::new(chiave).with("name", &c.nome).with("file", file)
}
