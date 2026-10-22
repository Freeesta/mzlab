//! Search of the MS2 of one feature in an MSn library (`docs/agenti/msnlib.md` §2): the proof "MSn tree of a reference" of Attributio. Shares the readers of `cerca`.

use super::cerca::{bad, best_of, read_graph, text, Sp};
use crate::error::Result;
use crate::json::parse;

/// An MSn library entry whose MS2 resembles the MS2 of a feature: the entropy similarity of its best MS2 and the peaks they share.
pub struct Ms2Hit {
    pub id: String,
    pub ion: String,
    pub entropy: f64,
    pub shared: usize,
}

/// The entries of an `msnlib/1` text whose precursor is within `ppm` of `mz` (same polarity) and whose best MS2, at any activation or energy, shares
/// at least `min_shared` peaks with `peaks` `(m/z, intensity)`; most similar first. A feature has no activation label, so every MS2 of the entry counts.
pub fn cerca_ms2(
    libreria: &str,
    mz: f64,
    positive: bool,
    peaks: &[(f64, f64)],
    ppm: f64,
    tol: f64,
    min_shared: usize,
) -> Result<Vec<Ms2Hit>> {
    let lib = parse(libreria.as_bytes())?;
    let u = Sp {
        path: String::new(),
        level: 2,
        act: String::new(),
        ce: 0.0,
        prec: String::new(),
        peaks: peaks.iter().map(|p| (p.0, p.1, None)).collect(),
    };
    let mut out = Vec::new();
    for v in lib.get("voci").map_or(&[][..], |x| x.arr()) {
        let e = read_graph(v.get("grafo").ok_or_else(bad)?)?;
        if (e.pol >= 0.0) != positive || ((e.mz - mz) / mz * 1e6).abs() > ppm {
            continue;
        }
        let best = e
            .spectra
            .iter()
            .filter(|x| x.level == 2)
            .filter_map(|x| best_of(&u, std::iter::once(x), tol))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((entropy, shared)) = best.filter(|b| b.1 >= min_shared) {
            out.push(Ms2Hit {
                id: text(v.get("id")),
                ion: text(v.get("ione").and_then(|i| i.get("formula"))),
                entropy,
                shared,
            });
        }
    }
    crate::sortc::slice_by(&mut out, |a, b| {
        b.entropy.total_cmp(&a.entropy).then(a.id.cmp(&b.id))
    });
    Ok(out)
}
