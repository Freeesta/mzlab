//! What two sources of the same count of a folder do not agree on, as notices of `apri`: the channels of a SIR/MRM function (`_FUNCTNS.INF` against the records per scan of
//! the IDX, and scans with fewer records than the others), the functions (`_extern.inf` against `_FUNCTNS.INF`), scans past a DAT cut short and a missing `_HEADER.TXT`.
//! Nothing is guessed: the notice names what is absent. `losses` counts the scans of the index that are not read, with the key of the cause (`Reader::losses`).

use super::inf::Extern;
use super::reader::Fun;
use mzlab_core::readers::{Losses, MEMBER_MISSING, RECORD_WIDTH, TRUNCATED, WATERS_DAMAGED};

const KEY: &str = "meta.nonDecodificato";

/// Scans of the index past the end of a DAT cut short (the rows of a function without its DAT are a missing member, not a cut).
pub(super) fn cut(funs: &[Fun]) -> usize {
    funs.iter().filter(|f| f.dat.is_some()).map(|f| f.cut).sum()
}

/// The scans of sample `c` the index lists and the `n` read: past the end of a DAT cut short, of a scanning function without its DAT, of one whose record
/// width is not decoded, and the `damaged` ones (scans with a record no acquisition writes, functions past a cut `_FUNCTNS.INF`) with the rows of the scanning
/// functions left out whole. The cause is the one of the most scans (the first of that order on a tie).
pub(super) fn losses(funs: &[Fun], c: usize, n: usize, damaged: usize) -> Losses {
    if c != 0 {
        return Losses::complete(n);
    }
    // every row of a function without its DAT lies past its (empty) end: `cut` holds the ones `cuts` set to 0 records
    let rows = |f: &Fun| f.cut + f.righe.iter().filter(|r| r.count > 0).count();
    let scanning = || funs.iter().filter(|f| !f.f.mirato());
    let gone = scanning().filter(|f| f.dat.is_none()).map(rows).sum();
    let width = scanning().filter(|f| f.dat.is_some() && f.cod.is_none());
    let parts = [
        (cut(funs), TRUNCATED),
        (gone, MEMBER_MISSING),
        (width.map(rows).sum(), RECORD_WIDTH),
        (
            damaged + scanning().filter_map(|f| f.left_out).sum::<usize>(),
            WATERS_DAMAGED,
        ),
    ];
    let lost: usize = parts.iter().map(|p| p.0).sum();
    let cause = parts
        .iter()
        .rev()
        .max_by_key(|p| p.0)
        .map_or(TRUNCATED, |p| p.1);
    Losses::of(n + lost, n, cause)
}

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
    v.extend(
        funs.iter()
            .filter(|f| f.cut > 0 && f.dat.is_some())
            .map(|f| {
                let k = f.f.indice;
                format!(
                    "function {k}: {} of {} scans past the end of _FUNC{k:03}.DAT",
                    f.cut,
                    f.righe.len()
                )
            }),
    );
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
    let mut out: Vec<(String, String)> = v
        .into_iter()
        .map(|a| (KEY.to_string(), format!("{name}: {a}")))
        .collect();
    out.extend(
        funs.iter()
            .flat_map(|f| super::idx::mark_notes(name, f.f.indice, &f.righe)),
    );
    // IMS functions are read summed over the drift bins: the drift time is not decoded
    if funs.iter().any(|f| f.ext.mobilita) {
        out.push((KEY.to_string(), format!("{name}: ion mobility")));
    }
    out
}
