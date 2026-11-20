//! The honest answers of `dad ms-at` and `dad delay`: the MS scan nearest to a time (a point window matches no scan, so the old answer was an empty
//! spectrum with exit 0), and the refusal to hand back an empty spectrum or a delay with no peak behind it. Failures are exit 1 (`err.dad.*`).

use mzlab_core::error::{Error, Result};

/// The slack, in cycles, before a time counts as outside the scans or as lying in a hole between them.
const SLACK: f64 = 1.5;
/// The least cycle time (minutes) assumed for a level with one scan or a degenerate spacing.
const MIN_CYCLE: f64 = 0.002;

/// The scan chosen for a time: its own time, the distance to the asked time, and the warning codes of the choice.
#[derive(Debug, PartialEq)]
pub(crate) struct Pick {
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
    d.sort_by(f64::total_cmp);
    d.get(d.len() / 2).copied().unwrap_or(0.0).max(MIN_CYCLE)
}

/// The scan of `times` nearest to `t`. `err.dad.msNoLevel` when there is none (the level is not in the file), `err.dad.msRange` when `t` lies more than
/// 1.5 cycles outside the scans; a time in a hole of more than 1.5 cycles still gets its nearest scan, with the warning `far_from_scan`.
pub(crate) fn nearest(times: &[f64], t: f64, level: u8) -> Result<Pick> {
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

/// `Ok` when the spectrum answer has peaks; `err.dad.msEmpty` (the scan chosen, nothing in it) otherwise.
pub(crate) fn has_peaks(spectrum: &str, pick: &Pick, level: u8) -> Result<()> {
    let n = mzlab_core::json::parse(spectrum.as_bytes())
        .ok()
        .and_then(|v| v.get("mz").map(|m| m.arr().len()))
        .unwrap_or(0);
    if n > 0 {
        return Ok(());
    }
    Err(Error::new("err.dad.msEmpty")
        .with("level", level)
        .with("scan_rt", pick.rt))
}

/// The `/api/delay` answer as it is when there is evidence behind it; otherwise the reason as an error: no matrix (`err.pda.none`), traces too short to
/// compare (`err.dad.delayShort`) or an estimate that no UV peak matches in the TIC (`err.dad.delayNoPeaks`, with the delay and correlation found).
pub(crate) fn delay_evidence(answer: String, has_uv: bool) -> Result<String> {
    let v =
        mzlab_core::json::parse(answer.as_bytes()).map_err(|_| Error::new("err.dad.delayShort"))?;
    let Some(d) = v.get("delay").filter(|d| d.get("delay").is_some()) else {
        return Err(Error::new(if has_uv {
            "err.dad.delayShort"
        } else {
            "err.pda.none"
        }));
    };
    if d.get("peaks").and_then(|p| p.f64()).unwrap_or(0.0) >= 1.0 {
        return Ok(answer);
    }
    let num = |k: &str| d.get(k).and_then(|x| x.f64()).unwrap_or(0.0);
    Err(Error::new("err.dad.delayNoPeaks")
        .with("delay", num("delay"))
        .with("corr", num("corr")))
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

    #[test]
    fn delay_with_no_matching_peak_or_no_evidence_is_refused() {
        let ok = "{\"delay\":{\"delay\":0.002,\"peaks\":2,\"corr\":0.9}}".to_string();
        assert_eq!(delay_evidence(ok.clone(), true).unwrap(), ok);
        let none = "{\"delay\":{\"delay\":0.1,\"peaks\":0,\"corr\":0.2}}".to_string();
        assert_eq!(
            delay_evidence(none, true).unwrap_err().key,
            "err.dad.delayNoPeaks"
        );
        let null = "{\"delay\":null}".to_string();
        assert_eq!(
            delay_evidence(null.clone(), false).unwrap_err().key,
            "err.pda.none"
        );
        assert_eq!(
            delay_evidence(null, true).unwrap_err().key,
            "err.dad.delayShort"
        );
    }
}
