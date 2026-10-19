// Ported from Attributio plugins/chimeric.py.
//! Chimeric MS/MS detection: the features of the same polarity that fall inside the isolation window around a feature's m/z and within a retention-time distance
//! were co-isolated, so their fragments may be in the spectrum. The window is the instrument's (`isolation_window` of the campaign, otherwise 3.0 for a Fusion
//! and 1.5 for the others). The intruders are shown to the chemist; nothing is removed from the spectrum.

#[derive(Debug, Clone, PartialEq)]
pub struct FeaturePoint {
    pub pol: String,
    pub id: i64,
    pub mz: f64,
    pub rt: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Intruder {
    pub id: i64,
    pub mz: f64,
    pub rt: f64,
    pub delta_mz: f64,
    pub delta_rt: f64,
}

/// The default isolation window of an instrument name.
pub fn default_window(instrument: &str) -> f64 {
    if instrument.contains("Fusion") {
        3.0
    } else {
        1.5
    }
}

fn round(x: f64, nd: i32) -> f64 {
    let k = 10f64.powi(nd);
    (x * k).round_ties_even() / k
}

/// The co-isolated features of `target`, nearest in m/z first; values rounded to 4 (m/z) and 2 (RT) decimals as the original reports them.
pub fn intruders(
    all: &[FeaturePoint],
    target: &FeaturePoint,
    isolation_window: f64,
    rt_tol: f64,
) -> Vec<Intruder> {
    let half = isolation_window / 2.0;
    let mut out: Vec<Intruder> = all
        .iter()
        .filter(|f| {
            f.pol == target.pol
                && f.id != target.id
                && (f.mz - target.mz).abs() < half
                && (f.rt - target.rt).abs() < rt_tol
        })
        .map(|f| Intruder {
            id: f.id,
            mz: round(f.mz, 4),
            rt: round(f.rt, 2),
            delta_mz: round(f.mz - target.mz, 4),
            delta_rt: round(f.rt - target.rt, 2),
        })
        .collect();
    out.sort_by(|a, b| a.delta_mz.abs().total_cmp(&b.delta_mz.abs()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(pol: &str, id: i64, mz: f64, rt: f64) -> FeaturePoint {
        FeaturePoint {
            pol: pol.into(),
            id,
            mz,
            rt,
        }
    }

    #[test]
    fn window_rt_polarity_and_order() {
        let all = [
            fp("POS", 1, 300.0, 5.0),
            fp("POS", 2, 300.9, 5.05),
            fp("POS", 3, 299.5, 5.0),
            fp("POS", 4, 300.0, 5.5),
            fp("POS", 5, 302.0, 5.0),
            fp("NEG", 6, 300.1, 5.0),
        ];
        let r = intruders(&all, &all[0], default_window("Q Exactive"), 0.1);
        assert_eq!(r.iter().map(|i| i.id).collect::<Vec<_>>(), vec![3]);
        let r = intruders(&all, &all[0], default_window("Orbitrap Fusion"), 0.1);
        assert_eq!(r.iter().map(|i| i.id).collect::<Vec<_>>(), vec![3, 2]);
        assert_eq!(r[1].delta_mz, 0.9);
        assert_eq!(r[1].delta_rt, 0.05);
    }
}
