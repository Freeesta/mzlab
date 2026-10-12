//! Per-row measurements that feed the proofs of the graph: the isotope pattern against the formula of the hypothesis and the share of the MS2 peaks
//! that the parent explains (equal or shifted by the mass difference). Pure functions of the features and of the spectra they are given.

use crate::table::FileFeatures;
use mzlab_core::composition::isotope_ratios;
use mzlab_core::elements::parse_formula;
use mzlab_core::roles::PROTON;

const D_13C: f64 = 1.003_355;
/// Windows to find the ions of a row in a file: m/z (ppm), RT (min) of the ion, RT (min) of the isotopologue from the apex of its light ion.
const FIND_PPM: f64 = 5.0;
const FIND_RT: f64 = 0.3;
const ISO_RT: f64 = 0.05;
/// Below this expected M+1 / M an absent isotopologue says nothing.
const ISO_MIN_RATIO: f64 = 0.08;

/// The peaks of one file ordered by m/z, to find them by m/z and RT.
pub struct FeatIndex<'a> {
    feats: &'a FileFeatures,
    order: Vec<u32>,
}

impl<'a> FeatIndex<'a> {
    pub fn new(feats: &'a FileFeatures) -> Self {
        let mut order: Vec<u32> = (0..feats.mz.len() as u32).collect();
        order.sort_by(|&a, &b| {
            feats.mz[a as usize]
                .total_cmp(&feats.mz[b as usize])
                .then(a.cmp(&b))
        });
        FeatIndex { feats, order }
    }

    /// The peak with the largest window area within the windows (position in the file), if any.
    pub fn find(&self, mz: f64, rt: f64, rt_tol: f64) -> Option<usize> {
        let tol = mz * FIND_PPM * 1e-6;
        let from = self
            .order
            .partition_point(|&i| self.feats.mz[i as usize] < mz - tol);
        self.order[from..]
            .iter()
            .map(|&i| i as usize)
            .take_while(|&i| self.feats.mz[i] <= mz + tol)
            .filter(|&i| (f64::from(self.feats.rt[i]) - rt).abs() <= rt_tol)
            .max_by(|&a, &b| {
                self.feats.area_win[a]
                    .total_cmp(&self.feats.area_win[b])
                    .then(b.cmp(&a))
            })
    }
}

/// Counts of the [M+H]+ (`polarity` 1) or [M-H]- (-1) ion of a neutral formula.
fn ion_counts(neutral: &str, polarity: i8) -> Option<Vec<(String, i64)>> {
    let mut c: Vec<(String, i64)> = parse_formula(neutral)
        .ok()?
        .into_iter()
        .map(|(e, k)| (e, k as i64))
        .collect();
    let h = c.iter_mut().find(|(e, _)| e == "H")?;
    h.1 += i64::from(polarity.signum());
    (h.1 > 0).then_some(c)
}

/// How well the isotope pattern of a row agrees with its formula, 0..1: 1 - |observed - expected| / expected for the M+1 / M ratio of the file where the
/// light ion is strongest (areas of the raw trace around the apex). `None` when the ion is not found or the expected M+1 is too weak to be seen; an
/// M+1 that is not found where it is expected scores 0.
pub fn isotope_score(
    files: &[FeatIndex],
    mz: f64,
    rt: f64,
    neutral: &str,
    polarity: i8,
) -> Option<f64> {
    let expected = isotope_ratios(&ion_counts(neutral, polarity)?).0;
    if expected < ISO_MIN_RATIO {
        return None;
    }
    let (best, pos) = files
        .iter()
        .filter_map(|f| f.find(mz, rt, FIND_RT).map(|p| (f, p)))
        .max_by(|a, b| a.0.feats.area_win[a.1].total_cmp(&b.0.feats.area_win[b.1]))?;
    let light = best.feats.area_win[pos];
    if light <= 0.0 {
        return None;
    }
    let apex = f64::from(best.feats.rt[pos]);
    let heavy = best
        .find(mz + D_13C, apex, ISO_RT)
        .map_or(0.0, |p| best.feats.area_win[p]);
    Some((1.0 - ((heavy / light) - expected).abs() / expected).clamp(0.0, 1.0))
}

/// A centroided MS2 spectrum.
#[derive(Clone, Debug, Default)]
pub struct Peaks {
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
}

/// Share (0..1) of the child's MS2 peaks above 5 % of the base peak that the parent explains: the same fragment (m/z within `tol` Da) or the fragment
/// shifted by the mass difference `delta` between child and parent. `None` when either spectrum has no peak. `delta` is neutral, polarity does not matter.
pub fn network_share(child: &Peaks, parent: &Peaks, delta: f64, tol: f64) -> Option<f64> {
    let top = |p: &Peaks| p.it.iter().copied().fold(0.0, f64::max);
    let (tc, tp) = (top(child), top(parent));
    if tc <= 0.0 || tp <= 0.0 {
        return None;
    }
    let near = |x: f64| {
        parent.mz.iter().zip(&parent.it).any(|(&m, &i)| {
            i >= 0.05 * tp && ((m - x).abs() <= tol || (m + delta - x).abs() <= tol)
        })
    };
    let used: Vec<f64> = child
        .mz
        .iter()
        .zip(&child.it)
        .filter(|(_, &i)| i >= 0.05 * tc)
        .map(|(&m, _)| m)
        .collect();
    (!used.is_empty()).then(|| used.iter().filter(|&&m| near(m)).count() as f64 / used.len() as f64)
}

/// Neutral mass of an ion of the polarity.
pub fn neutral_of(mz: f64, polarity: i8) -> f64 {
    mz - f64::from(polarity.signum()) * PROTON
}
