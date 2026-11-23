//! The spectrum-type term of a converted mzML (MS:1000127 centroid / MS:1000128 profile). A reader says "profile" for a whole instrument family, but sparse
//! data (isolated ion counts, no continuous trace, as a unit-resolution triple quad gives) are centroids: a downstream tool that trusts the profile term
//! smooths them and loses counts. `auto` looks at the points; `centroid` and `profile` are the user's word. One decision per sample, same on every route.
use crate::error::Result;
use crate::mzml::Spectrum;
use crate::readers::Reader;

/// Scans looked at to decide (equally spaced over the sample, as the estimate of the centroider does).
const PROBE: usize = 64;
/// Consecutive points of a run on a grid: a profile peak has at least this many, isolated counts almost never line up.
const RUN_POINTS: usize = 4;
/// Spacing of a point against the previous one inside a run: within this share of it.
const RUN_TOL: f64 = 0.25;
/// A run is a profile only when its points are closer than this (Da): a unit-resolution peak is about 0.7 Da wide, so a trace sampled every 1 Da or more is
/// a stick spectrum (nominal masses, isolated counts), however regular.
const MAX_STEP: f64 = 0.35;
/// Below this share of the points in runs the data are sparse.
const SPARSE_BELOW: f64 = 0.5;

/// What the user asks for (`--spectrum`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpectrumType {
    /// Keep what the reader says unless the data are sparse (then centroid).
    #[default]
    Auto,
    Centroid,
    Profile,
}

impl SpectrumType {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            "centroid" => Some(Self::Centroid),
            "profile" => Some(Self::Profile),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Centroid => "centroid",
            Self::Profile => "profile",
        }
    }
}

/// Why the term is what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// Asked for by the user.
    Asked,
    /// The reader reports centroids (or the peaks written are ours): nothing to decide.
    Reader,
    /// The points are on a continuous grid: the reader's profile stays.
    Continuous,
    /// Isolated counts: written as centroid although the reader says profile.
    Sparse,
    /// Nothing to look at (no scan with points).
    Empty,
}

/// The decision for one sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeChoice {
    pub asked: SpectrumType,
    /// `Some(p)`: every scan is written as profile (`p`) or centroid; `None`: each scan keeps the flag of its reader.
    pub force: Option<bool>,
    pub basis: Basis,
    /// Scans looked at and the share of their points that sit in continuous runs (`None` when not measured).
    pub scans: usize,
    pub share: Option<f64>,
}

impl TypeChoice {
    /// The choice that changes nothing.
    pub fn keep(asked: SpectrumType, basis: Basis) -> Self {
        Self {
            asked,
            force: None,
            basis,
            scans: 0,
            share: None,
        }
    }

    /// Sets the flag of `s` as decided.
    pub fn apply(&self, s: &mut Spectrum) {
        if let Some(p) = self.force {
            s.profile = p;
        }
    }

    /// The decision changes what the reader alone would have written, or the user asked for it: it is recorded in the file and the run.
    pub fn recorded(&self) -> bool {
        self.asked != SpectrumType::Auto || self.basis == Basis::Sparse
    }

    /// The type written for a scan the reader flagged `reader_profile`.
    pub fn written(&self, reader_profile: bool) -> &'static str {
        if self.force.unwrap_or(reader_profile) {
            "profile"
        } else {
            "centroid"
        }
    }

    /// One line for stderr, the `<run>` of the mzML and the provenance; `reader_profile`: the flag of the first scan.
    pub fn text(&self, reader_profile: bool) -> String {
        let basis = match self.basis {
            Basis::Asked => "asked".to_string(),
            Basis::Reader => "reader reports centroids".to_string(),
            Basis::Continuous => "continuous points".to_string(),
            Basis::Sparse => "sparse points, no continuous profile".to_string(),
            Basis::Empty => "no points".to_string(),
        };
        let mut s = format!(
            "spectrum={}; written={}; basis={basis}",
            self.asked.name(),
            self.written(reader_profile)
        );
        if let Some(v) = self.share {
            s += &format!("; continuous_points={v:.3}; scans_probed={}", self.scans);
        }
        s
    }
}

/// `(points in runs, points)` of one scan: a run is `RUN_POINTS` or more consecutive points closer than `MAX_STEP` whose spacing changes by at most
/// `RUN_TOL` from one to the next.
pub fn run_points(mz: &[f64]) -> (usize, usize) {
    let n = mz.len();
    // the points `from..to` as a run: long enough and on a fine grid
    let run = |from: usize, to: usize| -> usize {
        if to - from >= RUN_POINTS && (mz[to - 1] - mz[from]) / ((to - from - 1) as f64) <= MAX_STEP {
            to - from
        } else {
            0
        }
    };
    let (mut inside, mut start, mut prev) = (0usize, 0usize, f64::NAN);
    for i in 1..n {
        let d = mz[i] - mz[i - 1];
        let fits = d > 0.0 && prev.is_finite() && (d - prev).abs() <= RUN_TOL * prev;
        if !fits {
            inside += run(start, i);
            start = i - 1;
        }
        prev = if d > 0.0 { d } else { f64::NAN };
    }
    inside += run(start, n);
    // a point shared by two runs is counted twice
    (inside.min(n), n)
}

/// Decides the term of sample `c`: from the user's word or, for `auto`, from at most 64 scans of the sample (read and dropped).
/// `file_peaks`: the points written are the reader's own (`Peaks::File`); with other peaks `auto` leaves the scans as they are.
pub fn choose(
    asked: SpectrumType,
    file_peaks: bool,
    l: &mut dyn Reader,
    c: usize,
) -> Result<TypeChoice> {
    match asked {
        SpectrumType::Centroid | SpectrumType::Profile => Ok(TypeChoice {
            force: Some(asked == SpectrumType::Profile),
            ..TypeChoice::keep(asked, Basis::Asked)
        }),
        SpectrumType::Auto if !file_peaks => Ok(TypeChoice::keep(asked, Basis::Reader)),
        SpectrumType::Auto => {
            let n = l.index_without_tic(c)?.len();
            let probe = n.min(PROBE);
            let (mut flagged, mut inside, mut total, mut scans) = (0usize, 0usize, 0usize, 0);
            for j in 0..probe {
                let k = if probe > 1 { j * (n - 1) / (probe - 1) } else { 0 };
                let s = l.scan(c, k)?;
                if s.mz.is_empty() {
                    continue;
                }
                scans += 1;
                if !s.profile {
                    continue;
                }
                flagged += 1;
                let (i, t) = run_points(&s.mz);
                inside += i;
                total += t;
            }
            if scans == 0 {
                return Ok(TypeChoice::keep(asked, Basis::Empty));
            }
            if flagged == 0 {
                return Ok(TypeChoice {
                    scans,
                    ..TypeChoice::keep(asked, Basis::Reader)
                });
            }
            let share = inside as f64 / total.max(1) as f64;
            let sparse = share < SPARSE_BELOW;
            Ok(TypeChoice {
                asked,
                force: sparse.then_some(false),
                basis: if sparse { Basis::Sparse } else { Basis::Continuous },
                scans,
                share: Some(share),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_is_one_run_and_isolated_counts_are_none() {
        let grid: Vec<f64> = (0..50).map(|i| 100.0 + f64::from(i) * 0.1).collect();
        assert_eq!(run_points(&grid), (50, 50));
        let sparse = [100.2, 101.9, 103.0, 107.4, 108.1, 115.6, 120.0, 130.5];
        assert_eq!(run_points(&sparse).0, 0);
        assert_eq!(run_points(&[]), (0, 0));
        assert_eq!(run_points(&[100.0]), (0, 1));
    }

    #[test]
    fn a_regular_stick_spectrum_is_not_a_profile() {
        // nominal masses every 1 Da: regular, but wider than a unit-resolution peak
        let sticks: Vec<f64> = (0..50).map(|i| 100.0 + f64::from(i)).collect();
        assert_eq!(run_points(&sticks), (0, 50));
    }

    #[test]
    fn zero_suppressed_peaks_of_five_points_stay_runs() {
        let mut mz = Vec::new();
        for p in 0..10 {
            for i in 0..5 {
                mz.push(100.0 + f64::from(p) * 7.3 + f64::from(i) * 0.1);
            }
        }
        assert_eq!(run_points(&mz), (50, 50));
    }
}
