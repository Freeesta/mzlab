//! The MS scan nearest to a time (`dad ms-at`, `dad peaks`): a window of one instant holds no scan, so the scan is chosen by distance and the choice is
//! reported (the scan time, the gap, the range of the scans, warnings). A time well outside the scans, or a level the file does not have, is an error.

use crate::error::{Error, Result};

/// The slack, in cycles, before a time counts as outside the scans or as lying in a hole between them.
const SLACK: f64 = 1.5;
/// The least cycle time (minutes) assumed for a level with one scan or a degenerate spacing.
const MIN_CYCLE: f64 = 0.002;

/// The scan chosen for a time: its own time, the distance to the asked time, and the warning codes of the choice.
#[derive(Debug, PartialEq)]
pub struct Pick {
    pub rt: f64,
    pub gap: f64,
    pub range: (f64, f64),
    pub warnings: Vec<&'static str>,
}

/// Median spacing of the scan times, the cycle time of the level.
fn cycle(times: &[f64]) -> f64 {
    let mut d: Vec<f64> = times
        .windows(2)
        .filter_map(|w| match w {
            [a, b] => Some(b - a),
            _ => None,
        })
        .filter(|x| *x > 0.0)
        .collect();
    crate::sortc::slice_by(&mut d, f64::total_cmp);
    d.get(d.len() / 2).copied().unwrap_or(0.0).max(MIN_CYCLE)
}

/// The scan of `times` nearest to `t`. `err.dad.msNoLevel` when there is none (the level is not in the file), `err.dad.msRange` when `t` lies more than
/// 1.5 cycles outside the scans; a time in a hole of more than 1.5 cycles still gets its nearest scan, with the warning `far_from_scan`.
pub fn nearest(times: &[f64], t: f64, level: u8) -> Result<Pick> {
    let lo = times.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = times.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let Some(rt) = times
        .iter()
        .copied()
        .min_by(|a, b| (a - t).abs().total_cmp(&(b - t).abs()))
    else {
        return Err(Error::new("err.dad.msNoLevel").with("level", level));
    };
    let c = cycle(times) * SLACK;
    if t < lo - c || t > hi + c {
        return Err(Error::new("err.dad.msRange")
            .with("level", level)
            .with("rt", t)
            .with("start", lo)
            .with("end", hi));
    }
    let gap = ((rt - t).abs() * 1e6).round() / 1e6;
    let warnings = if gap > c {
        vec!["far_from_scan"]
    } else {
        vec![]
    };
    Ok(Pick {
        rt,
        gap,
        range: (lo, hi),
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearest_scan_is_taken_and_the_gap_is_reported() {
        let t = [1.0, 1.1, 1.2, 1.3];
        let p = nearest(&t, 1.14, 1).unwrap();
        assert_eq!((p.rt, p.gap, p.warnings.len()), (1.1, 0.04, 0));
        assert_eq!(p.range, (1.0, 1.3));
    }

    #[test]
    fn a_time_outside_the_scans_or_a_missing_level_is_an_error_with_the_evidence() {
        let t = [1.0, 1.1, 1.2, 1.3];
        let e = nearest(&t, 5.0, 1).unwrap_err();
        assert_eq!(e.key, "err.dad.msRange");
        assert_eq!(nearest(&[], 1.0, 2).unwrap_err().key, "err.dad.msNoLevel");
        // just past the end, within 1.5 cycles, still gets the last scan
        assert_eq!(nearest(&t, 1.4, 1).unwrap().rt, 1.3);
    }

    #[test]
    fn a_hole_between_scans_is_a_warning() {
        let t = [1.0, 1.1, 1.2, 2.0, 2.1];
        assert_eq!(nearest(&t, 1.6, 1).unwrap().warnings, ["far_from_scan"]);
    }
}
