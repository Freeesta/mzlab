//! Whether the extras of a scan can change the header text beyond the row `Rows::lettore` builds (the rows are then read from the text the writer gives),
//! and a total as that text gives it back.

use crate::mzml::Spectrum;
use crate::mzmlwrite::SpectrumExtra;

/// The accessions `spectrum` looks for in the text of a header (as substrings: `h.contains` too): an extra that carries one of them would change the row.
const LETTI: [&str; 26] = [
    "MS:1000341",
    "MS:1000326",
    "MS:1000325",
    "MS:1000789",
    "MS:1000790",
    "MS:1000791",
    "MS:1002815",
    "MS:1002476",
    "MS:1000511",
    "MS:1000130",
    "MS:1000129",
    "MS:1000016",
    "MS:1000512",
    "MS:1000800",
    "MS:1000285",
    "MS:1000128",
    "MS:1000045",
    "MS:1000744",
    "MS:1000827",
    "MS:1000828",
    "MS:1000829",
    "MS:1000041",
    "MS:1000042",
    "MS:1000422",
    "MS:1000133",
    "MS:1000598",
];

/// True when nothing in the scan or its extras can make the header text say more than the row `lettore` builds: no `MS:1000...` accession in the id, the
/// filter or the free text of the extras, and no `cv:` extra with an accession `spectrum` reads.
pub(super) fn extra_inerte(sp: &Spectrum, extra: &SpectrumExtra) -> bool {
    let bad = |t: &str| t.contains(['"', '<', '>']);
    let acc = |t: &str| t.contains("MS:1000");
    if acc(&sp.id) || acc(&sp.filter) {
        return false;
    }
    // the maker's TIC is taken as `spectrum` reads it back (`Rows::lettore`); the base peak is written as numbers under accessions no row reads.
    // Precursors and the reader's cvParams (unescaped accessions and units) are read from the text the writer gives them.
    #[cfg(feature = "precursori")]
    if !extra.precursors.is_empty() || !extra.scan_params.is_empty() || !extra.cv_params.is_empty()
    {
        return false;
    }
    extra
        .params
        .iter()
        .all(|(k, v)| match k.strip_prefix("cv:") {
            Some(a) => !bad(a) && !bad(v) && !acc(v) && !LETTI.iter().any(|l| a.contains(l)),
            None => !acc(k) && !acc(v),
        })
}

/// A total as `spectrum` reads it back from the text the writer gives: `{}` of an `f64` is the shortest text that parses to the same bits (`inf` too),
/// but every NaN is written `NaN`, which parses to the one `f64::NAN`.
pub(super) fn tic_letto(v: f64) -> f64 {
    if v.is_nan() {
        f64::NAN
    } else {
        v
    }
}
