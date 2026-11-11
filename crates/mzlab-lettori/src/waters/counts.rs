//! What two sources of the same count of a folder do not agree on, as notices of `apri`: the channels of a SIR/MRM function (`_FUNCTNS.INF` against the records per scan of
//! the IDX, and scans with fewer records than the others), the functions (`_extern.inf` against `_FUNCTNS.INF`), scans past a DAT cut short and a missing `_HEADER.TXT`.
//! Nothing is guessed: the notice names what is absent.

use super::inf::Extern;
use super::reader::Fun;

const KEY: &str = "meta.nonDecodificato";

/// What the channel counts of a SIR/MRM function do not agree on: scans of the index with fewer records than the others, and the channels of `_FUNCTNS.INF`
/// against the records per scan of the index (two sources of the same number).
fn channel_notes(f: &Fun) -> Vec<String> {
    let rows: Vec<_> = f.righe.iter().filter(|r| r.count > 0).collect();
    let n = rows.iter().map(|r| r.count as usize).max().unwrap_or(0);
    let (k, short) = (
        f.f.indice,
        rows.iter().filter(|r| (r.count as usize) < n).count(),
    );
    let mut v = Vec::new();
    if short > 0 {
        v.push(format!(
            "function {k}: {short} of {} scans with fewer than {n} channels",
            rows.len()
        ));
    }
    let inf = f.f.channels().len();
    if n > 0 && inf != n {
        v.push(format!(
            "function {k}: {inf} channels in _FUNCTNS.INF, {n} in _FUNC{k:03}.IDX"
        ));
    }
    v
}

/// The notices of folder `name`, as (key, where).
pub(super) fn notes(name: &str, funs: &[Fun], ext: &Extern, header: bool) -> Vec<(String, String)> {
    let mut v: Vec<String> = funs
        .iter()
        .filter(|f| f.f.mirato())
        .flat_map(channel_notes)
        .collect();
    // scans of the index past the end of a DAT cut short are not read: how many, and in which function
    v.extend(funs.iter().filter(|f| f.cut > 0).map(|f| {
        let k = f.f.indice;
        format!(
            "function {k}: {} of {} scans past the end of _FUNC{k:03}.DAT",
            f.cut,
            f.righe.len()
        )
    }));
    // a function `_extern.inf` describes past the end of `_FUNCTNS.INF` is named; it has no data to read
    let past: Vec<String> = ext
        .funzioni
        .keys()
        .filter(|&&k| k as usize > funs.len())
        .map(u32::to_string)
        .collect();
    if !past.is_empty() {
        v.push(format!(
            "function {} in _extern.inf, {} in _FUNCTNS.INF",
            past.join(", "),
            funs.len()
        ));
    }
    // without `_HEADER.TXT` there is no calibration polynomial: the m/z of a scanning function are the uncalibrated ones of the DAT
    if !header && funs.iter().any(|f| f.cod.is_some()) {
        v.push("_HEADER.TXT missing: m/z without the calibration polynomial".into());
    }
    v.into_iter()
        .map(|a| (KEY.to_string(), format!("{name}: {a}")))
        .collect()
}
