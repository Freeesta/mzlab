//! The baseline of a peak integration: `zero` (y = 0, the default), `edges` (the straight line each engine draws between the boundaries, the
//! numbers of records made before `zero` was the default) or `line` (a line the user sets: a level or two points, which may cut through the peak). The area against `zero` or `line` is
//! the signed trapezoid integral of the trace minus the line; the part of the trace under the line is reported apart (`below`), never dropped.
//! A `line` is never below zero: it is held at 0 wherever it would go below it (`at`), and the area it would have counted under zero is reported apart
//! (`clamped`, only when it applies).
//! One function for the Data view (`peak_area` of mzlab-wasm, `mzlab-cli peak area`), `peak-auto`, `chrom.transitions@2` and the SIM table.

use crate::peakq_lr::near_idx;

/// What the area of a peak is measured from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Baseline {
    /// The engine's own straight line between the two boundaries (unchanged numbers).
    Edges,
    /// y = 0: the absolute area under the trace between the boundaries.
    Zero,
    /// The line through (t0, y0) and (t1, y1), times in min; t0 = t1 is the level y0.
    Line { t0: f64, y0: f64, t1: f64, y1: f64 },
}

/// The parameter names, the same in `--set`, the flow steps and the page.
pub const MODES: [&str; 3] = ["edges", "zero", "line"];

/// The mode of an integration that names none: from zero (the background under the peak is part of the area).
pub const DEFAULT: Baseline = Baseline::Zero;

impl Baseline {
    /// The mode's name (`MODES`).
    pub fn name(&self) -> &'static str {
        match self {
            Baseline::Edges => "edges",
            Baseline::Zero => "zero",
            Baseline::Line { .. } => "line",
        }
    }

    /// The baseline at time `t`; `None` for `Edges`, which each engine draws itself. A `line` is held at 0 wherever it would go below it (between its two
    /// points, past them and for a level alike): the area never counts anything under the zero line. `line_at` is the line itself.
    pub fn at(&self, t: f64) -> Option<f64> {
        let v = self.line_at(t)?;
        Some(if matches!(self, Baseline::Line { .. }) {
            v.max(0.0)
        } else {
            v
        })
    }

    /// The straight line itself at `t`, extended without limit (what `at` holds at 0 past its points).
    pub fn line_at(&self, t: f64) -> Option<f64> {
        match *self {
            Baseline::Edges => None,
            Baseline::Zero => Some(0.0),
            Baseline::Line { t0, y0, t1, y1 } if t1 != t0 => {
                Some(y0 + (y1 - y0) * (t - t0) / (t1 - t0))
            }
            Baseline::Line { y0, .. } => Some(y0),
        }
    }

    /// From the parameters `baseline` (a name of `MODES`, absent = `DEFAULT`) and `base_t0`, `base_y0`, `base_t1`, `base_y1` of a `line`
    /// (`base_y0` is required; `base_y1` absent = `base_y0`, the times absent = 0, so a level needs only `base_y0`).
    pub fn from_params(
        mode: Option<&str>,
        num: impl Fn(&str) -> Option<f64>,
    ) -> Result<Baseline, String> {
        match mode.unwrap_or(DEFAULT.name()) {
            "edges" => Ok(Baseline::Edges),
            "zero" => Ok(Baseline::Zero),
            "line" => {
                let y0 = num("base_y0").ok_or("baseline=line needs base_y0")?;
                let (t0, t1) = (num("base_t0").unwrap_or(0.0), num("base_t1").unwrap_or(0.0));
                let y1 = num("base_y1").unwrap_or(y0);
                if ![t0, y0, t1, y1].iter().all(|v| v.is_finite()) {
                    return Err("the baseline line needs finite numbers".into());
                }
                Ok(Baseline::Line { t0, y0, t1, y1 })
            }
            o => Err(format!(
                "unknown baseline {o} (one of {})",
                MODES.join(", ")
            )),
        }
    }
}

/// The integral of a trace against a line, in the units of y times those of the times (counts x min for times in min).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Integral {
    /// Signed: the part of the trace above the line minus the part under it.
    pub area: f64,
    /// The part under the line, as a positive number (0 when the trace never goes under it).
    pub below: f64,
    /// The highest point above the line (negative when the whole stretch is under it).
    pub height: f64,
    /// Index of that point (the first one on a tie).
    pub apex: usize,
}

/// The integral of `y - line(rt)` over the points `lo..=hi`: trapezoids, a segment that crosses the line split exactly where it crosses it.
pub fn integrate(
    rt: &[f64],
    y: &[f64],
    lo: usize,
    hi: usize,
    line: impl Fn(f64) -> f64,
) -> Integral {
    let n = rt.len().min(y.len());
    let hi = hi.min(n.saturating_sub(1));
    if n == 0 || lo > hi {
        return Integral {
            area: 0.0,
            below: 0.0,
            height: 0.0,
            apex: lo.min(n.saturating_sub(1)),
        };
    }
    let d = |j: usize| y[j] - line(rt[j]);
    let (mut area, mut below, mut height, mut apex) = (0.0, 0.0, d(lo), lo);
    for j in lo + 1..=hi {
        let (d0, d1, w) = (d(j - 1), d(j), rt[j] - rt[j - 1]);
        area += (d0 + d1) / 2.0 * w;
        below += if d0 <= 0.0 && d1 <= 0.0 {
            -(d0 + d1) / 2.0 * w
        } else if d0 < 0.0 {
            -d0 * (d0 / (d0 - d1)) * w / 2.0
        } else if d1 < 0.0 {
            -d1 * (d1 / (d1 - d0)) * w / 2.0
        } else {
            0.0
        };
        if d1 > height {
            height = d1;
            apex = j;
        }
    }
    Integral {
        area,
        below,
        height,
        apex,
    }
}

/// The measure of the Data view between the times `a` and `b` (each taken to the nearest point, as the page draws the boundaries) against
/// `zero` or a `line`: `[area, below (counts x s), height, apex RT (min)]`; `None` for `Edges` (the page's own line) or an empty trace.
pub fn peak_area(rt: &[f64], y: &[f64], a: f64, b: f64, base: Baseline) -> Option<[f64; 4]> {
    let n = rt.len().min(y.len());
    base.at(0.0)?;
    if n == 0 {
        return None;
    }
    let (rt, y) = (&rt[..n], &y[..n]);
    let (lo, hi) = (near_idx(rt, a.min(b)), near_idx(rt, a.max(b)));
    let r = integrate(rt, y, lo, hi, |t| base.at(t).unwrap_or(0.0));
    Some([r.area * 60.0, r.below * 60.0, r.height, rt[r.apex]])
}

/// What holding a `line` at 0 took out of the area between the times `a` and `b` (counts x s, same samples and trapezoids as `peak_area`):
/// 0 for `zero`, `edges` and a line that stays at or above 0 there, so the page and the CLI add it to the JSON only when it is above 0.
pub fn clamped_removed(rt: &[f64], a: f64, b: f64, base: Baseline) -> f64 {
    let n = rt.len();
    if n == 0 || !matches!(base, Baseline::Line { .. }) {
        return 0.0;
    }
    let (lo, hi) = (near_idx(rt, a.min(b)), near_idx(rt, a.max(b)));
    let d = |j: usize| base.at(rt[j]).unwrap_or(0.0) - base.line_at(rt[j]).unwrap_or(0.0);
    let cut: f64 = (lo + 1..=hi.min(n - 1))
        .map(|j| (d(j - 1) + d(j)) / 2.0 * (rt[j] - rt[j - 1]))
        .sum();
    cut * 60.0
}

/// `peak_area` as the JSON the page and `mzlab-cli peak area` both print: `{"baseline","area","below","height","rt"[,"clamped"]}` (counts x s, counts, min;
/// `clamped` = what `clamped_removed` took out, present only when above 0).
pub fn peak_area_json(rt: &[f64], y: &[f64], a: f64, b: f64, base: Baseline) -> Option<String> {
    let [area, below, height, at] = peak_area(rt, y, a, b, base)?;
    let cut = clamped_removed(&rt[..rt.len().min(y.len())], a, b, base);
    let clamped = if cut > 0.0 {
        format!(",\"clamped\":{cut}")
    } else {
        String::new()
    };
    Some(format!(
        "{{\"baseline\":\"{}\",\"area\":{area},\"below\":{below},\"height\":{height},\"rt\":{at}{clamped}}}",
        base.name()
    ))
}

#[cfg(test)]
#[path = "baseline_test.rs"]
mod tests;
