//! Subtraction of one DAD matrix from another (`dad subtract`, `/api/pdasub`): B is resampled by linear interpolation onto the grid of A (retention time and
//! wavelength), the part of A that B does not cover is dropped, and every step taken (overlap, interpolation, shift, scale) is reported, never silent.

use super::dad::DadData;
use super::dad_align::{dad_align, Align};
use crate::error::{Error, Result};
use std::fmt::Write;

/// How B is scaled before it is subtracted. `Max`, `Tic` and `Fit` are computed on the overlap, after B is resampled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scale {
    None,
    Max,
    Tic,
    Fit,
    Factor(f64),
}

impl Scale {
    pub fn parse(s: &str) -> Option<Scale> {
        match s {
            "none" => Some(Scale::None),
            "max" => Some(Scale::Max),
            "tic" => Some(Scale::Tic),
            "fit" => Some(Scale::Fit),
            _ => s
                .parse::<f64>()
                .ok()
                .filter(|f| f.is_finite())
                .map(Scale::Factor),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Scale::None => "none",
            Scale::Max => "max",
            Scale::Tic => "tic",
            Scale::Fit => "fit",
            Scale::Factor(_) => "factor",
        }
    }
}

/// `shift_min`: B is read at `t - shift_min` (a positive shift moves B later); `scale`: see [`Scale`].
/// `auto_shift` replaces `shift_min` with the shift estimated by [`dad_align`] (on `focus`, the time window being read, when it is given).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubOpts {
    pub scale: Scale,
    pub shift_min: f64,
    pub auto_shift: bool,
    pub focus: Option<(f64, f64)>,
}

impl Default for SubOpts {
    fn default() -> Self {
        SubOpts {
            scale: Scale::None,
            shift_min: 0.0,
            auto_shift: false,
            focus: None,
        }
    }
}

/// What the subtraction did, for the user to see: ranges, steps, kept cells, the scale used and the warning codes.
#[derive(Debug, Clone, PartialEq)]
pub struct SubReport {
    pub a_rt: (f64, f64),
    pub b_rt: (f64, f64),
    pub a_nm: (f64, f64),
    pub b_nm: (f64, f64),
    pub rt_step_a: f64,
    pub rt_step_b: f64,
    pub nm_step_a: f64,
    pub nm_step_b: f64,
    pub rows: (usize, usize),
    pub cols: (usize, usize),
    pub shift_min: f64,
    pub scale: &'static str,
    pub factor: f64,
    pub interp_rt: bool,
    pub interp_nm: bool,
    /// The estimate behind `shift_min` when it was asked for (`auto_shift`); `None` for a shift given by hand.
    pub align: Option<Align>,
    pub warnings: Vec<&'static str>,
}

fn range(v: &[f64]) -> (f64, f64) {
    (
        v.first().copied().unwrap_or(0.0),
        v.last().copied().unwrap_or(0.0),
    )
}

fn median_step(v: &[f64]) -> f64 {
    let mut d: Vec<f64> = v.iter().zip(v.iter().skip(1)).map(|(a, b)| b - a).collect();
    crate::sortc::slice_by(&mut d, f64::total_cmp);
    d.get(d.len() / 2).copied().unwrap_or(0.0)
}

/// Position of `x` on the axis `ax` (ascending): the index of the left neighbour and the weight of the right one; `None` outside the axis by more than half a step.
fn locate(ax: &[f64], x: f64, half: f64) -> Option<(usize, f64)> {
    let (lo, hi) = range(ax);
    if !x.is_finite() || x < lo - half || x > hi + half {
        return None;
    }
    if ax.len() < 2 || x <= lo {
        return Some((0, 0.0));
    }
    if x >= hi {
        return Some((ax.len() - 1, 0.0));
    }
    let i = ax.partition_point(|&t| t <= x) - 1;
    let (x0, x1) = (ax.get(i)?, ax.get(i + 1)?);
    let w = (x - x0) / (x1 - x0);
    Some((i, if w == 0.0 { 0.0 } else { w }))
}

fn at(d: &DadData, i: usize, j: usize) -> f64 {
    d.matrix_mau
        .get(i * d.channels() + j)
        .map_or(0.0, |&v| f64::from(v))
}

/// Value of B at the fractional cell `(ri, rw)`, `(ci, cw)`; weight 0 reads the cell itself, so equal grids give B bit for bit.
fn sample(b: &DadData, (ri, rw): (usize, f64), (ci, cw): (usize, f64)) -> f64 {
    let row = |i: usize| {
        let v0 = at(b, i, ci);
        if cw == 0.0 {
            v0
        } else {
            v0 + (at(b, i, ci + 1) - v0) * cw
        }
    };
    let r0 = row(ri);
    if rw == 0.0 {
        r0
    } else {
        r0 + (row(ri + 1) - r0) * rw
    }
}

type Axis = Vec<(usize, (usize, f64))>;

/// The positions of `a_ax` (shifted by `shift`) on `b_ax`: the kept indices of A with their place in B, and whether any needed interpolation.
fn align(a_ax: &[f64], b_ax: &[f64], shift: f64) -> (Axis, bool) {
    let half = median_step(b_ax) / 2.0;
    let kept: Axis = a_ax
        .iter()
        .enumerate()
        .filter_map(|(i, &t)| Some((i, locate(b_ax, t - shift, half)?)))
        .collect();
    let interp = kept
        .iter()
        .any(|(i, (k, w))| *w != 0.0 || matches!((a_ax.get(*i), b_ax.get(*k)), (Some(x), Some(y)) if (x - shift - y).abs() > 1e-9));
    (kept, interp)
}

/// The factor of B: 1, the number given, or the ratio of the maxima, of the sums or of the least squares fit, on the overlap.
fn scale_factor(s: Scale, av: &[f64], bv: &[f64]) -> f64 {
    let ratio = |n: f64, d: f64| if d.abs() > 0.0 { n / d } else { 1.0 };
    let top = |v: &[f64]| v.iter().fold(0.0f64, |m, x| m.max(x.abs()));
    match s {
        Scale::None => 1.0,
        Scale::Factor(f) => f,
        Scale::Max => ratio(top(av), top(bv)),
        Scale::Tic => ratio(av.iter().sum(), bv.iter().sum()),
        Scale::Fit => ratio(
            av.iter().zip(bv).map(|(x, y)| x * y).sum(),
            bv.iter().map(|y| y * y).sum(),
        ),
    }
}

impl SubReport {
    /// The warning codes the numbers of the report call for.
    fn derive_warnings(&self) -> Vec<&'static str> {
        let differs = |x: f64, y: f64| x > 0.0 && y > 0.0 && (x / y > 1.05 || y / x > 1.05);
        [
            (self.rows.0 < self.rows.1, "rt_range_partial"),
            (self.cols.0 < self.cols.1, "nm_range_partial"),
            (self.interp_rt, "rt_interpolated"),
            (self.interp_nm, "nm_interpolated"),
            (differs(self.rt_step_a, self.rt_step_b), "rt_step_differs"),
            (differs(self.nm_step_a, self.nm_step_b), "nm_step_differs"),
            (self.shift_min != 0.0, "shift_applied"),
            (self.factor != 1.0, "scale_applied"),
        ]
        .into_iter()
        .filter_map(|(on, w)| on.then_some(w))
        .collect()
    }
}

/// The shift to apply: the estimate (with its evidence) when it was asked for, else the number given (0 when it is not finite).
fn chosen_shift(a: &DadData, b: &DadData, o: &SubOpts) -> (f64, Option<Align>) {
    if o.auto_shift {
        let f = dad_align(a, b, o.focus);
        return (f.shift_min, Some(f));
    }
    (
        if o.shift_min.is_finite() {
            o.shift_min
        } else {
            0.0
        },
        None,
    )
}

/// `A - scale * B` on the grid of A restricted to the rows and channels that B covers. `Err("err.pda.nooverlap")` when nothing overlaps, `err.pda.none` for an empty matrix.
#[allow(clippy::cast_possible_truncation)] // the matrix is f32
pub fn dad_subtract(a: &DadData, b: &DadData, o: &SubOpts) -> Result<(DadData, SubReport)> {
    let ok = |d: &DadData| {
        d.points() > 0 && d.channels() > 0 && d.matrix_mau.len() >= d.points() * d.channels()
    };
    if !ok(a) || !ok(b) {
        return Err(Error::new("err.pda.none"));
    }
    let (shift, align_found) = chosen_shift(a, b, o);
    let ((rows, interp_rt), (cols, interp_nm)) = (
        align(&a.time_min, &b.time_min, shift),
        align(&a.wavelengths_nm, &b.wavelengths_nm, 0.0),
    );
    if rows.is_empty() || cols.is_empty() {
        return Err(Error::new("err.pda.nooverlap"));
    }
    // B on the grid of A, then the scale on that overlap
    let bv: Vec<f64> = rows
        .iter()
        .flat_map(|(_, r)| cols.iter().map(move |(_, c)| sample(b, *r, *c)))
        .collect();
    let av: Vec<f64> = rows
        .iter()
        .flat_map(|(i, _)| cols.iter().map(move |(j, _)| at(a, *i, *j)))
        .collect();
    let factor = scale_factor(o.scale, &av, &bv);
    let matrix: Vec<f32> = av
        .iter()
        .zip(&bv)
        .map(|(x, y)| (x - factor * y) as f32)
        .collect();
    let twc = matrix
        .chunks(cols.len())
        .map(|r| r.iter().map(|&v| f64::from(v)).sum())
        .collect();
    let time_min: Vec<f64> = rows
        .iter()
        .filter_map(|(i, _)| a.time_min.get(*i).copied())
        .collect();
    let wavelengths_nm: Vec<f64> = cols
        .iter()
        .filter_map(|(j, _)| a.wavelengths_nm.get(*j).copied())
        .collect();
    let (wl_start_nm, wl_stop_nm) = range(&wavelengths_nm);
    let nm_step_a = median_step(&a.wavelengths_nm);
    let out = DadData {
        duration_min: range(&time_min).1 - range(&time_min).0,
        wl_step_nm: nm_step_a,
        time_min,
        twc_mau: twc,
        wavelengths_nm,
        wl_start_nm,
        wl_stop_nm,
        matrix_mau: matrix,
    };
    let mut rep = SubReport {
        a_rt: range(&a.time_min),
        b_rt: range(&b.time_min),
        a_nm: range(&a.wavelengths_nm),
        b_nm: range(&b.wavelengths_nm),
        rt_step_a: median_step(&a.time_min),
        rt_step_b: median_step(&b.time_min),
        nm_step_a,
        nm_step_b: median_step(&b.wavelengths_nm),
        rows: (rows.len(), a.points()),
        cols: (cols.len(), a.channels()),
        shift_min: shift,
        scale: o.scale.name(),
        factor,
        interp_rt,
        interp_nm,
        align: align_found,
        warnings: vec![],
    };
    rep.warnings = rep.derive_warnings();
    if let Some(f) = &rep.align {
        rep.warnings.extend(f.warnings.iter().copied());
    }
    Ok((out, rep))
}

fn num(x: f64) -> String {
    let r = (x * 1e6).round() / 1e6;
    if r.is_finite() {
        format!("{r}")
    } else {
        "null".into()
    }
}

impl SubReport {
    /// The report as JSON with stable keys (`dad subtract --json`, `/api/pdasub`).
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        let pair = |p: (f64, f64)| format!("[{},{}]", num(p.0), num(p.1));
        let _ = write!(
            s,
            "{{\"a_rt\":{},\"b_rt\":{},\"a_nm\":{},\"b_nm\":{},\"rt_step_a\":{},\"rt_step_b\":{},\"nm_step_a\":{},\"nm_step_b\":{},\"rows\":[{},{}],\"cols\":[{},{}],\"shift_min\":{},\"scale\":\"{}\",\"factor\":{},\"interp_rt\":{},\"interp_nm\":{},\"warnings\":[",
            pair(self.a_rt), pair(self.b_rt), pair(self.a_nm), pair(self.b_nm),
            num(self.rt_step_a), num(self.rt_step_b), num(self.nm_step_a), num(self.nm_step_b),
            self.rows.0, self.rows.1, self.cols.0, self.cols.1,
            num(self.shift_min), self.scale, num(self.factor), self.interp_rt, self.interp_nm
        );
        s.push_str(
            &self
                .warnings
                .iter()
                .map(|w| format!("\"{w}\""))
                .collect::<Vec<_>>()
                .join(","),
        );
        s.push(']');
        if let Some(f) = &self.align {
            let _ = write!(s, ",\"align\":{}", f.to_json());
        }
        s.push('}');
        s
    }
}

#[cfg(test)]
#[path = "dad_ops_tests.rs"]
mod tests;
