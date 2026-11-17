//! The `ms2_exps` list of the item JSON (`iteminfo.rs`): precursor, CE, and for a stepped CE its centre and half width (`35 ± 15` for 20/35/50),
//! so the page, the app and the CLI show the same numbers read from the file.

use crate::idxread::Scan;
use crate::iteminfo::ms2_exps;
use crate::json::Out;

/// Stepped collision energy of one precursor: the centre and half width (`35 ± 15` for 20/35/50) of the distinct CEs its experiments carry;
/// `(None, None)` with fewer than two of them (the page then shows the plain CE).
pub(crate) fn ce_spread(
    exps: &[(Option<f64>, Option<f64>, usize)],
    prec: Option<f64>,
) -> (Option<f64>, Option<f64>) {
    let ces = exps
        .iter()
        .filter(|e| e.0 == prec)
        .filter_map(|e| e.1)
        .filter(|c| c.is_finite());
    let (lo, hi) = ces.fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), c| {
        (l.min(c), h.max(c))
    });
    if hi > lo {
        (Some((lo + hi) / 2.0), Some((hi - lo) / 2.0))
    } else {
        (None, None)
    }
}

/// The `ms2_exps` list of the item JSON: precursor, CE, centre and half width of a stepped CE, scan count.
pub(crate) fn write_ms2_exps(o: &mut Out, ms2: &[&Scan]) {
    let exps = ms2_exps(ms2);
    for (i, (p, ce, n)) in exps.iter().enumerate() {
        let (mid, spread) = ce_spread(&exps, *p);
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"prec\":")
            .opt(*p)
            .raw(",\"ce\":")
            .opt(*ce)
            .raw(",\"ce_mid\":")
            .opt(mid)
            .raw(",\"ce_spread\":")
            .opt(spread)
            .raw(",\"n\":")
            .int(*n as i64)
            .raw("}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stepped_collision_energy_gives_centre_and_half_width() {
        let e = [
            (Some(253.1), Some(20.0), 3),
            (Some(253.1), Some(35.0), 3),
            (Some(253.1), Some(50.0), 3),
            (Some(300.0), Some(30.0), 2),
            (Some(310.0), None, 1),
        ];
        assert_eq!(ce_spread(&e, Some(253.1)), (Some(35.0), Some(15.0)));
        assert_eq!(
            ce_spread(&e, Some(300.0)),
            (None, None),
            "one CE: no spread"
        );
        assert_eq!(ce_spread(&e, Some(310.0)), (None, None), "no CE at all");
    }
}
