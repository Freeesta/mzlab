//! Drift of the RT of the parent along a series: the tolerance of RT of the Dig must hold the parent in every file, so it is `max(2 × FWHM, drift)`
//! (`params::Estimate::rt_tol` is the first term). The drift is the spread of the RT of the strongest feature near the m/z of the parent over the files.

use mzlab_core::json::Out;

use crate::table::FileFeatures;

/// Half window of m/z around the parent, Da (the one of the offset of the calibration).
const MZ_WINDOW: f64 = 0.6;
/// Half window of RT around the RT of the strongest parent, minutes: a feature farther away is not the parent.
const RT_SEARCH: f64 = 2.0;

/// The RT of the parent in every file (`None` where it is not seen) and the spread of those.
#[derive(Debug, Clone, PartialEq)]
pub struct Drift {
    pub rts: Vec<Option<f64>>,
    /// Largest minus smallest RT over the files that show the parent (two at least).
    pub drift: Option<f64>,
}

/// The feature of `f` near `parent_mz` with the greatest height, among those within `rt` (centre, half window) when given.
fn strongest(f: &FileFeatures, parent_mz: f64, rt: Option<(f64, f64)>) -> Option<usize> {
    let a = f.mz.partition_point(|&m| m < parent_mz - MZ_WINDOW);
    let b = f.mz.partition_point(|&m| m <= parent_mz + MZ_WINDOW);
    (a..b)
        .filter(|&i| rt.is_none_or(|(c, w)| (f64::from(f.rt[i]) - c).abs() <= w))
        .max_by(|&i, &j| f.height[i].total_cmp(&f.height[j]))
}

pub fn parent_drift(files: &[&FileFeatures], parent_mz: f64) -> Drift {
    let first: Vec<Option<usize>> = files
        .iter()
        .map(|f| strongest(f, parent_mz, None))
        .collect();
    let centre = files
        .iter()
        .zip(&first)
        .filter_map(|(f, i)| i.map(|i| (f.height[i], f64::from(f.rt[i]))))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, rt)| rt);
    let rts: Vec<Option<f64>> = files
        .iter()
        .map(|f| {
            let c = centre?;
            strongest(f, parent_mz, Some((c, RT_SEARCH))).map(|i| f64::from(f.rt[i]))
        })
        .collect();
    let seen: Vec<f64> = rts.iter().flatten().copied().collect();
    let drift = (seen.len() >= 2).then(|| {
        let hi = seen.iter().copied().fold(f64::MIN, f64::max);
        let lo = seen.iter().copied().fold(f64::MAX, f64::min);
        hi - lo
    });
    Drift { rts, drift }
}

/// `{"drift", "n", "rts", "rt_tol"}`: `rt_tol` = the larger of `base_rt_tol` (twice the FWHM; NaN = none) and the drift, `null` when neither is known.
pub fn drift_json(d: &Drift, base_rt_tol: f64) -> String {
    let base = base_rt_tol.is_finite().then_some(base_rt_tol);
    let tol = match (base, d.drift) {
        (Some(b), Some(x)) => Some(b.max(x)),
        (b, x) => b.or(x),
    };
    let mut o = Out::default();
    o.raw("{\"drift\":")
        .opt(d.drift.map(|x| (x * 1e4).round() / 1e4))
        .raw(",\"n\":")
        .int(d.rts.iter().flatten().count() as i64);
    o.raw(",\"rts\":[");
    for (i, r) in d.rts.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .opt(r.map(|x| (x * 1e4).round() / 1e4));
    }
    o.raw("],\"rt_tol\":")
        .opt(tol.map(|x| (x * 1e4).round() / 1e4))
        .raw("}");
    o.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(peaks: &[(f64, f32, f32)]) -> FileFeatures {
        let mut f = FileFeatures::default();
        for &(mz, rt, h) in peaks {
            f.mz.push(mz);
            f.rt.push(rt);
            f.height.push(h);
        }
        f
    }

    #[test]
    fn the_drift_is_the_spread_of_the_rt_of_the_strongest_parent() {
        let a = file(&[(100.0, 1.0, 5.0), (230.1, 6.0, 1e5), (230.3, 3.0, 9.0)]);
        let b = file(&[(230.2, 6.5, 8e4), (400.0, 6.0, 1e9)]);
        let c = file(&[(230.0, 5.8, 7e4), (230.2, 9.5, 6e4)]);
        let d = parent_drift(&[&a, &b, &c], 230.2);
        let r: Vec<f64> = d.rts.iter().map(|x| x.unwrap()).collect();
        assert!(
            r.iter()
                .zip([6.0, 6.5, 5.8])
                .all(|(a, b)| (a - b).abs() < 1e-5),
            "{r:?}"
        );
        assert!((d.drift.unwrap() - 0.7).abs() < 1e-5, "{d:?}");
    }

    #[test]
    fn one_file_or_no_parent_gives_no_drift() {
        let a = file(&[(230.2, 6.0, 1e5)]);
        let none = file(&[(500.0, 6.0, 1e5)]);
        assert_eq!(parent_drift(&[&a], 230.2).drift, None);
        let d = parent_drift(&[&a, &none], 230.2);
        assert_eq!((d.rts[1], d.drift), (None, None));
        assert_eq!(parent_drift(&[&none], 230.2).rts, vec![None]);
    }

    #[test]
    fn the_tolerance_is_the_larger_of_the_width_and_the_drift() {
        let d = Drift {
            rts: vec![Some(6.0), Some(6.5)],
            drift: Some(0.5),
        };
        assert!(drift_json(&d, 0.2).contains("\"rt_tol\":0.5}"));
        assert!(drift_json(&d, 0.8).contains("\"rt_tol\":0.8}"));
        assert!(drift_json(&d, f64::NAN).contains("\"rt_tol\":0.5}"));
        let none = Drift {
            rts: vec![None],
            drift: None,
        };
        assert!(drift_json(&none, f64::NAN).contains("\"rt_tol\":null}"));
    }
}
