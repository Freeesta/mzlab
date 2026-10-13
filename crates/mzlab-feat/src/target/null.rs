//! Chance of a coincidence: how often an ion that nobody predicted shows a peak that passes the same thresholds.
//! The density of such peaks is measured in every file at decoy masses (hypothesis masses shifted by a fraction of a dalton, outside the ppm window
//! and away from isotopes and adducts), searched with the same extraction over the whole run: peaks per decoy per minute. A hypothesis whose
//! window lasts `w` minutes has a chance `1 - exp(-rate * w)` of a random peak in a file (Poisson); the chance that `k` of the files show one is the
//! tail of the sum of independent Bernoulli trials (exact, file by file).

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use super::extract::{peaks, Section};
use super::TargetParams;

/// Distance of a decoy from the mass it is taken from, Da (more than any window in ppm and not an isotope spacing).
pub const DECOY_SHIFT: f64 = 0.0236;
/// The same at unit resolution (tolerance in Da): outside the window of the hypothesis and not an isotope spacing.
pub const DECOY_SHIFT_UNIT: f64 = 0.5;

/// Peaks per decoy per minute in one file; `masses` are the m/z of the hypotheses (any order), of which about `p.decoys` are used.
pub(crate) fn decoy_rate<R: ReadAt>(
    index: &mut Index<R>,
    sec: &Section,
    masses: &[f64],
    p: &TargetParams,
) -> Result<f64> {
    if masses.is_empty() || p.decoys == 0 {
        return Ok(0.0);
    }
    let step = (masses.len() / p.decoys).max(1);
    let run = sec.run();
    let (mut found, mut n) = (0usize, 0usize);
    for (k, &mz) in masses.iter().step_by(step).take(p.decoys).enumerate() {
        let d = if p.ppm < 0.0 {
            DECOY_SHIFT_UNIT
        } else {
            DECOY_SHIFT
        };
        let shift = if k % 2 == 0 { d } else { -d };
        found += peaks(index, sec, mz + shift, run, p)?.len();
        n += 1;
    }
    let minutes = (run.1 - run.0).max(1e-9);
    Ok(found as f64 / (n as f64 * minutes))
}

/// Chance that at least `hits` of the files, each with its own decoy `rates` (peaks per minute), show a random peak in a window of `minutes`.
pub fn coincidence(rates: &[f64], minutes: f64, hits: usize) -> f64 {
    if hits == 0 {
        return 1.0;
    }
    if hits > rates.len() {
        return 0.0;
    }
    let mut dp = vec![0.0f64; rates.len() + 1];
    dp[0] = 1.0;
    for (i, r) in rates.iter().enumerate() {
        let p = 1.0 - libm::exp(-r.max(0.0) * minutes.max(0.0));
        for j in (0..=i + 1).rev() {
            dp[j] = dp[j] * (1.0 - p) + if j > 0 { dp[j - 1] * p } else { 0.0 };
        }
    }
    dp[hits..].iter().sum::<f64>().min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_of_independent_files() {
        // one file, rate 0.1/min over 5 min: 1 - exp(-0.5)
        let p = coincidence(&[0.1], 5.0, 1);
        assert!((p - (1.0 - libm::exp(-0.5))).abs() < 1e-12);
        // three equal files, p = 0.5 each: P(>=2) = 0.5
        let r = -libm::log(0.5);
        assert!((coincidence(&[r, r, r], 1.0, 2) - 0.5).abs() < 1e-12);
        assert_eq!(coincidence(&[r], 1.0, 2), 0.0);
        assert_eq!(coincidence(&[], 1.0, 0), 1.0);
        // more files make the same count less likely by chance only if the count grows
        assert!(coincidence(&[r; 13], 1.0, 10) < 0.1);
    }
}
