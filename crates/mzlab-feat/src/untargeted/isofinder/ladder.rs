//! The ladder of candidate peaks on the 13C grid of a base peak, and the carbon ratio measured on it (the «Isotopic peaks finder» of mzmine 4.10.90).
//!
//! After `engine/CarbonLadder.java`, `engine/OffsetSpan.java` and `engine/CarbonRatio.java` (MIT, (c) 2004-2022 The mzmine Development Team);
//! rewritten, no code copied. An offset is the number of grid steps (`spacing` = 13C distance / charge) from the base peak.

use std::collections::BTreeMap;

use super::envelope::jround;
use super::Tol;

const GAP_TOL_FACTOR: f64 = 3.0;
const CLUSTER_MAX_GAP: usize = 2;

/// A range of offsets visited with a step (1 for every 13C, 2 for every second one).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OffsetSpan {
    pub min: i64,
    pub max: i64,
    pub step: i64,
}

impl OffsetSpan {
    pub fn size(&self) -> i64 {
        (self.max - self.min) / self.step + 1
    }
    pub fn contains(&self, o: i64) -> bool {
        o >= self.min && o <= self.max
    }
    pub fn extend_to(&self, o: i64) -> OffsetSpan {
        if self.contains(o) {
            *self
        } else {
            OffsetSpan {
                min: self.min.min(o),
                max: self.max.max(o),
                step: self.step,
            }
        }
    }
}

/// Peaks that fall on one offset: the one nearest to the exact grid point, and the intensity-weighted sum of all of them.
#[derive(Clone, Copy, Debug)]
struct LadderPeak {
    nearest_mz: f64,
    nearest_int: f64,
    grid_error: f64,
    sum_int: f64,
    weighted_mz: f64,
}

/// An offset with all the signal found on it.
impl Ladder {
    fn gap_free(&self, step: i64) -> OffsetSpan {
        let (mut hi, mut lo) = (0, 0);
        while self
            .nearest_within(self.exact_mz(hi + step), GAP_TOL_FACTOR)
            .is_some()
        {
            hi += step;
        }
        while self
            .nearest_within(self.exact_mz(lo - step), GAP_TOL_FACTOR)
            .is_some()
        {
            lo -= step;
        }
        OffsetSpan {
            min: lo,
            max: hi,
            step,
        }
    }

    /// The gap-free span through offset 0 in steps of 1 (at least 2 offsets) or, failing that, of 2 (at least 3: the pattern of Cl, Br, Cu); `None` if neither.
    pub fn require_c13_span(&self) -> Option<OffsetSpan> {
        let every = self.gap_free(1);
        if every.size() >= 2 {
            return Some(every);
        }
        let second = self.gap_free(2);
        (second.size() >= 3).then_some(second)
    }

    fn cluster_steps(&self, start: f64, delta: f64) -> i64 {
        let min_advance = delta.abs() / 2.0;
        let (mut steps, mut r) = (0, start);
        'outer: loop {
            for gap in 1..=CLUSTER_MAX_GAP {
                if let Some(found) = self.nearest_within(r + gap as f64 * delta, GAP_TOL_FACTOR) {
                    if (found - r).abs() >= min_advance {
                        r = found;
                        steps += gap as i64;
                        continue 'outer;
                    }
                }
            }
            return steps;
        }
    }

    /// The chain of peaks reached from `anchor_mz` in steps of one or two grid steps (each within 3 x the tolerance and advancing by at least half
    /// a step), as offsets around `from`.
    pub fn cluster_span_around(&self, from: i64, anchor_mz: f64) -> OffsetSpan {
        let up = self.cluster_steps(anchor_mz, self.spacing);
        let down = self.cluster_steps(anchor_mz, -self.spacing);
        OffsetSpan {
            min: from - down,
            max: from + up,
            step: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OffsetPeak {
    pub mz: f64,
    pub intensity: f64,
}

impl Ladder {
    /// Intensity of the nearest peak of every offset whose peak lies within `factor` x the tolerance of the exact grid point.
    pub fn on_grid_intensities(&self, factor: f64) -> BTreeMap<i64, f64> {
        self.by_offset
            .iter()
            .filter(|(o, p)| self.on_grid(**o, p, factor))
            .map(|(o, p)| (*o, p.nearest_int))
            .collect()
    }

    /// m/z of the nearest peak of the offsets that lie on the grid (see [`Ladder::on_grid_intensities`]).
    pub fn on_grid_mz(&self, factor: f64) -> BTreeMap<i64, f64> {
        self.by_offset
            .iter()
            .filter(|(o, p)| self.on_grid(**o, p, factor))
            .map(|(o, p)| (*o, p.nearest_mz))
            .collect()
    }

    /// Every offset with its total intensity and the intensity-weighted m/z (NaN if the total is 0).
    pub fn collapsed(&self) -> BTreeMap<i64, OffsetPeak> {
        self.by_offset
            .iter()
            .map(|(o, p)| {
                (
                    *o,
                    OffsetPeak {
                        mz: if p.sum_int > 0.0 {
                            p.weighted_mz / p.sum_int
                        } else {
                            f64::NAN
                        },
                        intensity: p.sum_int,
                    },
                )
            })
            .collect()
    }

    /// The candidate nearest to `mz` if it lies within `factor` x the tolerance of it.
    fn nearest_within(&self, mz: f64, factor: f64) -> Option<f64> {
        let i = closest_index(&self.sorted_mz, mz)?;
        let n = self.sorted_mz[i];
        ((n - mz).abs() <= self.tol.for_mass(mz) * factor).then_some(n)
    }
}

pub struct Ladder {
    base_mz: f64,
    spacing: f64,
    tol: Tol,
    sorted_mz: Vec<f64>,
    by_offset: BTreeMap<i64, LadderPeak>,
}

impl Ladder {
    /// Ladder of the `(m/z, intensity)` candidates on the grid of `base_mz` with step `spacing`.
    pub fn build(candidates: &[(f64, f64)], base_mz: f64, spacing: f64, tol: Tol) -> Ladder {
        let mut by_offset: BTreeMap<i64, LadderPeak> = BTreeMap::new();
        let mut sorted_mz = Vec::with_capacity(candidates.len());
        for &(mz, inten) in candidates {
            sorted_mz.push(mz);
            let offset = jround((mz - base_mz) / spacing);
            let error = (mz - (base_mz + offset as f64 * spacing)).abs();
            match by_offset.get_mut(&offset) {
                None => {
                    by_offset.insert(
                        offset,
                        LadderPeak {
                            nearest_mz: mz,
                            nearest_int: inten,
                            grid_error: error,
                            sum_int: inten,
                            weighted_mz: mz * inten,
                        },
                    );
                }
                Some(p) => {
                    if error < p.grid_error {
                        p.nearest_mz = mz;
                        p.nearest_int = inten;
                        p.grid_error = error;
                    }
                    p.sum_int += inten;
                    p.weighted_mz += mz * inten;
                }
            }
        }
        sorted_mz.sort_by(f64::total_cmp);
        Ladder {
            base_mz,
            spacing,
            tol,
            sorted_mz,
            by_offset,
        }
    }

    fn exact_mz(&self, offset: i64) -> f64 {
        self.base_mz + offset as f64 * self.spacing
    }

    fn on_grid(&self, offset: i64, p: &LadderPeak, factor: f64) -> bool {
        self.tol
            .within_factor(self.exact_mz(offset), p.nearest_mz, factor)
    }
}

/// M+1 / M ratio read on the 13C ladder at the monoisotopic offset implied by the placement of the envelope.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarbonRatio {
    pub present: bool,
    pub value: f64,
    pub anchor_is_mono: bool,
    pub anchor_is_base: bool,
}

const SLACK: f64 = 0.3;
const LOWER_FACTOR: f64 = 0.25;
const LOWER_PENALTY_FLOOR: f64 = 1e-3;
const REQUIRE_C13_LOWER_FACTOR: f64 = 0.5;
const MONO_DOMINANCE_FRACTION: f64 = 0.1;

impl CarbonRatio {
    pub const ABSENT: CarbonRatio = CarbonRatio {
        present: false,
        value: 0.0,
        anchor_is_mono: false,
        anchor_is_base: false,
    };

    pub fn measure(ladder: &BTreeMap<i64, f64>, placement: i64) -> CarbonRatio {
        let mono = -placement;
        let Some(&mono_i) = ladder.get(&mono) else {
            return Self::ABSENT;
        };
        if mono_i <= 0.0 {
            return Self::ABSENT;
        }
        let below = ladder.range(..mono).map(|(_, v)| *v).fold(0.0f64, f64::max);
        let m1 = ladder.get(&(mono + 1)).copied().unwrap_or(0.0);
        CarbonRatio {
            present: true,
            value: m1 / mono_i,
            anchor_is_mono: below < MONO_DOMINANCE_FRACTION * mono_i,
            anchor_is_base: placement == 0,
        }
    }

    fn supports_lower_bound(&self) -> bool {
        self.present && self.anchor_is_mono && self.anchor_is_base
    }

    /// Factor in (0, 1] that lowers the quality when the ratio is far above the maximum or, where it can be trusted, far under the minimum.
    pub fn plausibility(&self, b: [f64; 2]) -> f64 {
        if !self.present {
            return 1.0;
        }
        let hi = b[1] * (1.0 + SLACK);
        if hi > 0.0 && self.value > hi {
            return hi / self.value;
        }
        let lo = b[0] * LOWER_FACTOR;
        if self.supports_lower_bound() && lo > 0.0 && self.value < lo {
            return (self.value / lo).max(LOWER_PENALTY_FLOOR);
        }
        1.0
    }

    /// The gate of «Require 13C isotope peak»: the ratio is outside the bounds although it can be trusted.
    pub fn fails_require_c13(&self, b: [f64; 2]) -> bool {
        if !self.supports_lower_bound() || self.value <= 0.0 {
            return false;
        }
        self.value < b[0] * REQUIRE_C13_LOWER_FACTOR || self.value > b[1] * (1.0 + SLACK)
    }
}

/// Index of the value nearest to `v` in the ascending `a` (the higher one on a tie), as the closest-value search of mzmine; `None` for an empty slice.
pub fn closest_index(a: &[f64], v: f64) -> Option<usize> {
    if a.is_empty() {
        return None;
    }
    let idx = a.partition_point(|x| *x < v);
    if idx >= a.len() {
        return Some(a.len() - 1);
    }
    if a[idx] == v || idx == 0 {
        return Some(idx);
    }
    Some(if (v - a[idx]).abs() <= (a[idx - 1] - v).abs() {
        idx
    } else {
        idx - 1
    })
}
