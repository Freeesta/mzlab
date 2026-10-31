//! The matched peaks of a library candidate (`matched_peaks`): for each pair the m/z and relative intensity on both sides, the difference and whether the
//! precursor shift was needed. They are the evidence behind a score, listed by query m/z; used by `flowmatch`.

use crate::scores::{ModCosine, Spec};
use mzlab_core::json::{round, Out};
use mzlab_picchi::sortc::SortC;

pub(crate) struct Pair {
    query_mz: f64,
    library_mz: f64,
    query_int: f64,
    library_int: f64,
    shifted: bool,
}

/// The pairs of the plain scores: `pairs` are (index in `q`, index in `lib`).
pub(crate) fn from_pairs(q: &Spec, lib: &Spec, pairs: &[(usize, usize)]) -> Vec<Pair> {
    let all = pairs.iter().map(|&(i, j)| (q, i, lib, j, false));
    collect(all)
}

/// The pairs of the modified cosine (indices into its top-32 spectra), shifted ones included.
pub(crate) fn from_modified(m: &ModCosine) -> Vec<Pair> {
    collect(
        m.pairs
            .iter()
            .map(|&(i, j, s)| (&m.a_top, i, &m.b_top, j, s)),
    )
}

fn collect<'a>(it: impl Iterator<Item = (&'a Spec, usize, &'a Spec, usize, bool)>) -> Vec<Pair> {
    let mut v: Vec<Pair> = it
        .map(|(q, i, l, j, shifted)| Pair {
            query_mz: q.mz[i],
            library_mz: l.mz[j],
            query_int: q.it[i],
            library_int: l.it[j],
            shifted,
        })
        .collect();
    v.sortc_by(|a, b| a.query_mz.total_cmp(&b.query_mz));
    v
}

/// The array `matched_peaks` of the candidate record; intensities are fractions of each spectrum's maximum.
pub(crate) fn write(o: &mut Out, v: &[Pair]) {
    o.raw(",\"matched_peaks\":[");
    for (k, p) in v.iter().enumerate() {
        if k > 0 {
            o.raw(",");
        }
        o.raw("{\"query_mz\":").num(round(p.query_mz, 5));
        o.raw(",\"library_mz\":").num(round(p.library_mz, 5));
        o.raw(",\"query_int\":").num(round(p.query_int, 4));
        o.raw(",\"library_int\":").num(round(p.library_int, 4));
        o.raw(",\"shifted\":")
            .raw(if p.shifted { "true" } else { "false" });
        o.raw("}");
    }
    o.raw("]");
}
