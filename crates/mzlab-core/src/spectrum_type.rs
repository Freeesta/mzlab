//! The spectrum-type term of a converted mzML (MS:1000127 centroid / MS:1000128 profile). A reader says "profile" for a whole instrument family, but sparse
//! data (isolated ion counts, no continuous trace, as a unit-resolution triple quad gives) are centroids: a downstream tool that trusts the profile term
//! smooths them and loses counts. `auto` looks at the points of each MS level and acts only on unit-resolution data (a fine grid is never touched);
//! `centroid` and `profile` are the user's word. One decision per sample, same on every route.
use crate::error::Result;
use crate::mzml::Spectrum;
use crate::readers::Reader;
use crate::readers_mzml::Peaks;

/// Scans looked at to decide (equally spaced over the sample, as the estimate of the centroider does).
const PROBE: usize = 64;
/// Consecutive points of a run on a grid: a profile peak has at least this many, isolated counts almost never line up.
const RUN_POINTS: usize = 4;
/// Spacing of a point against the previous one inside a run: within this share of it.
const RUN_TOL: f64 = 0.25;
/// A run is a profile only when its points are closer than this (Da): a unit-resolution peak is about 0.7 Da wide, so a Q1 trace sampled every 0.5 Da is
/// still a profile, but one sampled every 1 Da or more is a stick spectrum (nominal masses, isolated counts), however regular.
const MAX_STEP: f64 = 0.6;
/// Below this share of the points in runs a level is sparse.
const SPARSE_BELOW: f64 = 0.5;
/// A neighbour spacing under this (Da) is a fine grid: peaks of two or three points of a high-resolution profile have no run but are never ion counts.
const FINE_STEP: f64 = 0.05;
/// A level where this share of the neighbour spacings is fine is a high-resolution profile: `auto` never touches it. Low on purpose: zero-suppressed
/// profile peaks sit among many isolated single-bin hits whose coarse spacings dilute the share (a few real peaks per hundreds of hits still give about 1 %);
/// unit-resolution ion counts have no spacing this fine to speak of.
const FINE_SHARE: f64 = 0.02;
/// MS levels told apart (the higher ones count as the last).
const LEVELS: usize = 32;

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
    /// The reader reports centroids, or the points are the profile points of the file: nothing to decide.
    Reader,
    /// The points are on a continuous grid: the reader's profile stays.
    Continuous,
    /// Isolated counts: the scans of the sparse MS levels are written as centroid although the reader says profile.
    Sparse,
    /// The points written are the peaks of mzLab (centroids): nothing to decide.
    Peaks,
    /// Nothing to look at (no scan with points).
    Empty,
}

/// The decision for one sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeChoice {
    pub asked: SpectrumType,
    /// `Some(p)`: every scan is written as profile (`p`) or centroid; `None`: each scan keeps the flag of its reader, except the sparse levels.
    pub force: Option<bool>,
    /// Bit `l` set: the profile scans of MS level `l` (the last bit for any higher) are isolated counts and are written as centroid.
    pub sparse: u32,
    pub basis: Basis,
    /// Scans looked at and the share of their profile points that sit in continuous runs (`None` when not measured).
    pub scans: usize,
    pub share: Option<f64>,
}

impl TypeChoice {
    /// The choice that changes nothing.
    pub fn keep(asked: SpectrumType, basis: Basis) -> Self {
        Self {
            asked,
            force: None,
            sparse: 0,
            basis,
            scans: 0,
            share: None,
        }
    }

    /// The flag written for a scan of MS level `level` that the reader flagged `profile`.
    pub fn keeps(&self, profile: bool, level: u8) -> bool {
        match self.force {
            Some(p) => p,
            None => profile && self.sparse & level_bit(level) == 0,
        }
    }

    /// Sets the flag of `s` as decided.
    pub fn apply(&self, s: &mut Spectrum) {
        s.profile = self.keeps(s.profile, s.level);
    }

    /// The decision changes what the reader alone would have written, or the user asked for it: it is recorded in the file and the run.
    pub fn recorded(&self) -> bool {
        self.asked != SpectrumType::Auto || self.basis == Basis::Sparse
    }

    /// The MS levels written as centroid by the decision of the points, in order ("1,2").
    fn sparse_levels(&self) -> String {
        let v: Vec<String> = (0..LEVELS)
            .filter(|l| self.sparse & (1 << l) != 0)
            .map(|l| l.to_string())
            .collect();
        v.join(",")
    }

    /// One line for stderr, the `<run>` of the mzML and the provenance.
    pub fn text(&self) -> String {
        let written = match (self.force, self.basis) {
            (Some(true), _) => "profile".to_string(),
            (Some(false), _) => "centroid".to_string(),
            (None, Basis::Peaks) => "centroid".to_string(),
            (None, Basis::Sparse) => format!(
                "centroid for ms_level {}, as the reader elsewhere",
                self.sparse_levels()
            ),
            (None, _) => "as the reader".to_string(),
        };
        let basis = match self.basis {
            Basis::Asked => "asked",
            Basis::Reader => "points as the reader gives them",
            Basis::Continuous => "continuous points",
            Basis::Sparse => "sparse points, no continuous profile",
            Basis::Peaks => "peaks mzlab",
            Basis::Empty => "no points",
        };
        let mut s = format!(
            "spectrum={}; written={written}; basis={basis}",
            self.asked.name()
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
        if to - from >= RUN_POINTS && (mz[to - 1] - mz[from]) / ((to - from - 1) as f64) <= MAX_STEP
        {
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
/// `peaks`: which points are written; only the reader's own (`Peaks::File`) are looked at, with other peaks `auto` leaves the scans as they are.
pub fn choose(
    asked: SpectrumType,
    peaks: Peaks,
    l: &mut dyn Reader,
    c: usize,
) -> Result<TypeChoice> {
    match asked {
        SpectrumType::Centroid | SpectrumType::Profile => Ok(TypeChoice {
            force: Some(asked == SpectrumType::Profile),
            ..TypeChoice::keep(asked, Basis::Asked)
        }),
        SpectrumType::Auto if peaks == Peaks::Mzlab => Ok(TypeChoice::keep(asked, Basis::Peaks)),
        SpectrumType::Auto if peaks != Peaks::File => Ok(TypeChoice::keep(asked, Basis::Reader)),
        SpectrumType::Auto => {
            let seen = probe(l, c)?;
            if seen.scans == 0 {
                return Ok(TypeChoice::keep(asked, Basis::Empty));
            }
            if seen.flagged == 0 {
                return Ok(TypeChoice {
                    scans: seen.scans,
                    ..TypeChoice::keep(asked, Basis::Reader)
                });
            }
            // each level on its own: a few profile scans among many sparse product scans stay profile; a fine grid is never sparse
            let sparse = (0..LEVELS)
                .filter(|&l| seen.of[l].is_sparse())
                .fold(0u32, |m, l| m | 1 << l);
            let (inside, total) = seen
                .of
                .iter()
                .fold((0, 0), |(i, t), s| (i + s.inside, t + s.total));
            Ok(TypeChoice {
                sparse,
                basis: if sparse != 0 {
                    Basis::Sparse
                } else {
                    Basis::Continuous
                },
                scans: seen.scans,
                share: Some(inside as f64 / total.max(1) as f64),
                ..TypeChoice::keep(asked, Basis::Continuous)
            })
        }
    }
}

/// The profile scans looked at of one MS level: their points in runs and in all, and their neighbour spacings in all and on a fine grid.
#[derive(Clone, Copy, Default)]
struct Level {
    scans: usize,
    inside: usize,
    total: usize,
    gaps: usize,
    fine: usize,
}

impl Level {
    /// Profile scans were seen, few of their points are in runs and the spacings are not those of a fine grid.
    fn is_sparse(&self) -> bool {
        let fine_grid = self.fine > 0 && (self.fine as f64) >= FINE_SHARE * self.gaps as f64;
        self.scans > 0 && (self.inside as f64) < SPARSE_BELOW * self.total as f64 && !fine_grid
    }
}

/// What the scans looked at show: how many hold points, how many of those the reader flags profile, and the profile ones per MS level.
struct Probe {
    scans: usize,
    flagged: usize,
    of: [Level; LEVELS],
}

/// The bit of MS level `level` in `TypeChoice::sparse`.
fn level_bit(level: u8) -> u32 {
    1 << (usize::from(level)).min(LEVELS - 1)
}

/// At most `PROBE` scans of sample `c`, equally spaced, read and dropped.
fn probe(l: &mut dyn Reader, c: usize) -> Result<Probe> {
    let n = l.index_without_tic(c)?.len();
    let count = n.min(PROBE);
    let mut seen = Probe {
        scans: 0,
        flagged: 0,
        of: [Level::default(); LEVELS],
    };
    for j in 0..count {
        let k = if count > 1 {
            j * (n - 1) / (count - 1)
        } else {
            0
        };
        // a scan that does not read is not the probe's to report: the writer reads it again and drops, counts or refuses it as it always did
        let Ok(s) = l.scan(c, k) else { continue };
        if s.mz.is_empty() {
            continue;
        }
        seen.scans += 1;
        if s.profile {
            let (inside, total) = run_points(&s.mz);
            let lv = &mut seen.of[usize::from(s.level).min(LEVELS - 1)];
            seen.flagged += 1;
            lv.scans += 1;
            lv.inside += inside;
            lv.total += total;
            for w in s.mz.windows(2) {
                let d = w[1] - w[0];
                if d > 0.0 {
                    lv.gaps += 1;
                    lv.fine += usize::from(d < FINE_STEP);
                }
            }
        }
    }
    Ok(seen)
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
