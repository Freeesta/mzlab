// Ported from Attributio ms2_engine/generate.py.
//! The explicit command that fills the engine hits: every feature of the campaign with an MS/MS spectrum and a precursor m/z in the feature table is searched in the
//! library and the best hit becomes a row of `engine_<pol>.csv`. Reading a campaign never calls this; the caller decides where the text goes (the page offers it as
//! a download, the desktop app may write it into the campaign folder).

use super::hits::EngineHitRow;
use super::matcher::{Ms2Library, Query, SearchOpts};
use crate::attributio::sources::Sources;

/// The rows of a polarity (`"POS"` / `"NEG"`), by feature id; features without a spectrum, without a precursor or without a hit are left out.
pub fn generate_hits(
    src: &Sources,
    pol: &str,
    lib: &Ms2Library,
    o: &SearchOpts,
) -> Vec<EngineHitRow> {
    let Some(p) = src.pol(pol) else {
        return Vec::new();
    };
    let mut keys = p.feature_keys();
    keys.sort_by_key(|k| k.id);
    let sign = if pol == "POS" { 1 } else { -1 };
    let mut out = Vec::new();
    for k in keys {
        let (Some(mz), Some(sp)) = (k.mz, p.feature_spectra.get(&k.id)) else {
            continue;
        };
        let q = Query {
            id: k.id.to_string(),
            precursor_mz: mz,
            pol: sign,
            mz: sp.mz.clone(),
            it: sp.intensity.clone(),
        };
        if let Some(best) = lib.search(&q, o).first() {
            out.push(EngineHitRow::from_hit(pol, k.id, best));
        }
    }
    out
}
