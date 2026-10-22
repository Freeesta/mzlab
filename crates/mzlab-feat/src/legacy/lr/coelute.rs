//! Co-elution of an unexpected ion with a heavier strong or possible candidate: a measurement (same RT, lighter by the difference), not a verdict.
//! Used by `summary` for the key `coelutes` of the rows of unexpected ions; a probable in-source fragment of that candidate shows up here.

use super::{Entry, LrDig};

/// An unexpected ion this much lighter than a candidate (Da) is no isotope or adduct of it.
const MIN_LOSS: f64 = 3.0;

/// The candidate an unexpected ion co-elutes with.
pub struct Coel {
    pub id: i64,
    pub mz: f64,
    /// Row m/z minus candidate m/z (negative: the ion is lighter).
    pub delta_mz: f64,
    /// Row RT minus candidate RT (min).
    pub delta_rt: f64,
}

/// The strong or possible candidate heavier than `e` by at least `MIN_LOSS` whose RT is within half the RT tolerance of the ion, the closest in RT.
pub fn of(d: &LrDig, e: &Entry) -> Option<Coel> {
    if !e.unexpected {
        return None;
    }
    let cands = d
        .entries
        .iter()
        .filter(|c| c.id != 0 && !c.unexpected && matches!(c.label, "forte" | "possibile"))
        .filter_map(|c| Some((c.id, c.mz, c.ref_rt?)));
    pick(e.mz, e.ref_rt?, 0.5 * d.settings.rt_tol_min, cands)
}

/// `of` on plain numbers: `cands` = (id, m/z, RT) of the candidates.
fn pick(mz: f64, rt: f64, tol: f64, cands: impl Iterator<Item = (i64, f64, f64)>) -> Option<Coel> {
    cands
        .filter(|&(_, m, t)| m >= mz + MIN_LOSS && (rt - t).abs() <= tol)
        .min_by(|a, b| (rt - a.2).abs().total_cmp(&(rt - b.2).abs()))
        .map(|(id, m, t)| Coel {
            id,
            mz: m,
            delta_mz: mz - m,
            delta_rt: rt - t,
        })
}

#[cfg(test)]
mod tests {
    use super::pick;

    #[test]
    fn closest_heavier_candidate_in_time() {
        let c = [(1, 362.06, 13.02), (2, 396.06, 13.41), (3, 378.05, 13.0)];
        let p = pick(150.05, 13.03, 0.125, c.iter().copied()).unwrap();
        assert_eq!(p.id, 1);
        assert!((p.delta_mz + 212.01).abs() < 1e-9 && (p.delta_rt - 0.01).abs() < 1e-9);
        assert!(
            pick(150.05, 13.2, 0.125, c.iter().copied()).is_none(),
            "too far in time"
        );
        assert!(
            pick(377.0, 13.01, 0.125, c.iter().copied()).is_none(),
            "isotope-like: lighter by less than 3 Da"
        );
        assert!(
            pick(500.0, 13.02, 0.125, c.iter().copied()).is_none(),
            "heavier than every candidate"
        );
    }
}
