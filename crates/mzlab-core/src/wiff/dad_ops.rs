//! Subtraction of one DAD matrix from another (`dad subtract`, `/api/pdasub`): B is resampled by linear interpolation onto the grid of A (retention time and
//! wavelength), the part of A that B does not cover is dropped, and every step taken (overlap, interpolation, shift, scale) is reported, never silent.

use super::dad::DadData;
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubOpts {
    pub scale: Scale,
    pub shift_min: f64,
}

impl Default for SubOpts {
    fn default() -> Self {
        SubOpts {
            scale: Scale::None,
            shift_min: 0.0,
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
    pub warnings: Vec<&'static str>,
}

fn range(v: &[f64]) -> (f64, f64) {
    (
        v.first().copied().unwrap_or(0.0),
        v.last().copied().unwrap_or(0.0),
    )
}

fn median_step(v: &[f64]) -> f64 {
    let mut d: Vec<f64> = v.windows(2).map(|w| w[1] - w[0]).collect();
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
    let w = (x - ax[i]) / (ax[i + 1] - ax[i]);
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

/// `A - scale * B` on the grid of A restricted to the rows and channels that B covers. `Err("err.pda.nooverlap")` when nothing overlaps, `err.pda.none` for an empty matrix.
pub fn dad_subtract(a: &DadData, b: &DadData, o: &SubOpts) -> Result<(DadData, SubReport)> {
    let (na, nb) = (a.points(), b.points());
    if na == 0
        || nb == 0
        || a.channels() == 0
        || b.channels() == 0
        || a.matrix_mau.len() < na * a.channels()
        || b.matrix_mau.len() < nb * b.channels()
    {
        return Err(Error::new("err.pda.none"));
    }
    let shift = if o.shift_min.is_finite() {
        o.shift_min
    } else {
        0.0
    };
    let (rt_step_a, rt_step_b) = (median_step(&a.time_min), median_step(&b.time_min));
    let (nm_step_a, nm_step_b) = (
        median_step(&a.wavelengths_nm),
        median_step(&b.wavelengths_nm),
    );
    let rows: Vec<(usize, (usize, f64))> = a
        .time_min
        .iter()
        .enumerate()
        .filter_map(|(i, &t)| Some((i, locate(&b.time_min, t - shift, rt_step_b / 2.0)?)))
        .collect();
    let cols: Vec<(usize, (usize, f64))> = a
        .wavelengths_nm
        .iter()
        .enumerate()
        .filter_map(|(j, &w)| Some((j, locate(&b.wavelengths_nm, w, nm_step_b / 2.0)?)))
        .collect();
    if rows.is_empty() || cols.is_empty() {
        return Err(Error::new("err.pda.nooverlap"));
    }
    let interp_rt = rows.iter().any(|(_, (_, w))| *w != 0.0)
        || rows
            .iter()
            .any(|(i, (k, _))| (a.time_min[*i] - shift - b.time_min[*k]).abs() > 1e-9);
    let interp_nm = cols.iter().any(|(_, (_, w))| *w != 0.0)
        || cols
            .iter()
            .any(|(j, (k, _))| (a.wavelengths_nm[*j] - b.wavelengths_nm[*k]).abs() > 1e-9);
    // B on the grid of A, then the scale on that overlap
    let bv: Vec<f64> = rows
        .iter()
        .flat_map(|(_, r)| cols.iter().map(move |(_, c)| (*r, *c)))
        .map(|(r, c)| sample(b, r, c))
        .collect();
    let av: Vec<f64> = rows
        .iter()
        .flat_map(|(i, _)| cols.iter().map(move |(j, _)| at(a, *i, *j)))
        .collect();
    let ratio = |n: f64, d: f64| if d.abs() > 0.0 { n / d } else { 1.0 };
    let factor = match o.scale {
        Scale::None => 1.0,
        Scale::Factor(f) => f,
        Scale::Max => ratio(
            av.iter().fold(0.0f64, |m, v| m.max(v.abs())),
            bv.iter().fold(0.0f64, |m, v| m.max(v.abs())),
        ),
        Scale::Tic => ratio(av.iter().sum(), bv.iter().sum()),
        Scale::Fit => ratio(
            av.iter().zip(&bv).map(|(x, y)| x * y).sum(),
            bv.iter().map(|y| y * y).sum(),
        ),
    };
    let nc = cols.len();
    let mut matrix = Vec::with_capacity(av.len());
    let mut twc = Vec::with_capacity(rows.len());
    for (r, _) in rows.iter().enumerate() {
        let mut sum = 0.0;
        for c in 0..nc {
            let v = (av[r * nc + c] - factor * bv[r * nc + c]) as f32;
            sum += f64::from(v);
            matrix.push(v);
        }
        twc.push(sum);
    }
    let time_min: Vec<f64> = rows.iter().map(|(i, _)| a.time_min[*i]).collect();
    let wavelengths_nm: Vec<f64> = cols.iter().map(|(j, _)| a.wavelengths_nm[*j]).collect();
    let (wl_start_nm, wl_stop_nm) = range(&wavelengths_nm);
    let out = DadData {
        duration_min: time_min.last().copied().unwrap_or(0.0)
            - time_min.first().copied().unwrap_or(0.0),
        wl_step_nm: nm_step_a,
        time_min,
        twc_mau: twc,
        wavelengths_nm,
        wl_start_nm,
        wl_stop_nm,
        matrix_mau: matrix,
    };
    let mut w = Vec::new();
    if rows.len() < na {
        w.push("rt_range_partial");
    }
    if cols.len() < a.channels() {
        w.push("nm_range_partial");
    }
    if interp_rt {
        w.push("rt_interpolated");
    }
    if interp_nm {
        w.push("nm_interpolated");
    }
    let differs = |x: f64, y: f64| x > 0.0 && y > 0.0 && (x / y > 1.05 || y / x > 1.05);
    if differs(rt_step_a, rt_step_b) {
        w.push("rt_step_differs");
    }
    if differs(nm_step_a, nm_step_b) {
        w.push("nm_step_differs");
    }
    if shift != 0.0 {
        w.push("shift_applied");
    }
    if factor != 1.0 {
        w.push("scale_applied");
    }
    let rep = SubReport {
        a_rt: range(&a.time_min),
        b_rt: range(&b.time_min),
        a_nm: range(&a.wavelengths_nm),
        b_nm: range(&b.wavelengths_nm),
        rt_step_a,
        rt_step_b,
        nm_step_a,
        nm_step_b,
        rows: (rows.len(), na),
        cols: (cols.len(), a.channels()),
        shift_min: shift,
        scale: o.scale.name(),
        factor,
        interp_rt,
        interp_nm,
        warnings: w,
    };
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
        s.push_str("]}");
        s
    }
}

#[cfg(test)]
#[path = "dad_ops_tests.rs"]
mod tests;
