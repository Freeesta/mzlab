//! The kinetic descriptors of a row as mzFinder shows them (`TP_Mine/py/tpmine/hr/kinetics.py::descriptors`, `not_in_reference`; the unimodal fit
//! `mzlab/ionfamily.py::_isotonic_up_down`).

use mzlab_core::json::Out;

/// `descriptors(times, y)`: `None` when no treated point has an area (`{"ok": false}`).
#[derive(Debug, Clone, PartialEq)]
pub struct Descriptors {
    pub onset: f64,
    pub tmax: f64,
    pub residual_fraction: f64,
    pub unimodal_residual: f64,
    pub class: Vec<&'static str>,
    pub profile: Vec<f64>,
    pub times: Vec<f64>,
}

impl Descriptors {
    pub fn unimodal(&self) -> bool {
        self.unimodal_residual < 0.15
    }

    pub fn class_text(&self) -> String {
        self.class.join(" and ")
    }
}

/// Pool-adjacent-violators: the closest non-decreasing sequence.
fn pav(v: impl Iterator<Item = f64>) -> Vec<f64> {
    let (mut vals, mut wts): (Vec<f64>, Vec<usize>) = (vec![], vec![]);
    for x in v {
        vals.push(x);
        wts.push(1);
        while vals.len() > 1 && vals[vals.len() - 2] > vals[vals.len() - 1] {
            let n = vals.len();
            let tot = vals[n - 2] * wts[n - 2] as f64 + vals[n - 1] * wts[n - 1] as f64;
            let wt = wts[n - 2] + wts[n - 1];
            vals.truncate(n - 1);
            wts.truncate(n - 1);
            vals[n - 2] = tot / wt as f64;
            wts[n - 2] = wt;
        }
    }
    vals.iter()
        .zip(&wts)
        .flat_map(|(&v, &w)| std::iter::repeat_n(v, w))
        .collect()
}

fn argmax(v: &[f64]) -> usize {
    let mut k = 0;
    for (i, &x) in v.iter().enumerate() {
        if x > v[k] {
            k = i;
        }
    }
    k
}

/// `_isotonic_up_down`: non-decreasing up to the maximum, non-increasing after it.
pub fn isotonic_up_down(c: &[f64]) -> Vec<f64> {
    if c.is_empty() {
        return vec![];
    }
    let k = argmax(c);
    let left = pav(c[..=k].iter().copied());
    let mut right = pav(c[k..].iter().rev().copied());
    right.reverse();
    let mut out = left[..left.len() - 1].to_vec();
    out.push(left[left.len() - 1].max(right[0]));
    out.extend_from_slice(&right[1..]);
    out
}

/// `descriptors(times, y)`: onset (first time above 10 % of the maximum), tmax, fraction left at the last time, unimodal residual, class. Only the
/// points with a time ≥ 0 count; they are taken in order of time (stable for equal times).
pub fn descriptors(times: &[f64], y: &[f64]) -> Option<Descriptors> {
    let mut pts: Vec<(f64, f64)> = times
        .iter()
        .zip(y)
        .filter(|(t, _)| **t >= 0.0)
        .map(|(&t, &v)| (t, v))
        .collect();
    if pts.is_empty() || pts.iter().all(|p| p.1.is_nan() || p.1 <= 0.0) {
        return None;
    }
    crate::sortc::by(&mut pts, |a, b| a.0.total_cmp(&b.0));
    let (t, y): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
    let mx = y[argmax(&y)];
    let yn: Vec<f64> = y.iter().map(|v| v / mx).collect();
    let onset = yn.iter().position(|&v| v > 0.10).map_or(f64::NAN, |i| t[i]);
    let tmax = t[argmax(&y)];
    let last = yn[yn.len() - 1];
    let uni = isotonic_up_down(&yn);
    let ss: f64 = yn.iter().zip(&uni).map(|(a, b)| (a - b) * (a - b)).sum();
    let resid = (ss / yn.len() as f64).sqrt();
    let mut class = vec![if tmax <= 20.0 || onset <= 5.0 {
        "early"
    } else {
        "late"
    }];
    if last >= 0.5 {
        class.push("persistent");
    }
    Some(Descriptors {
        onset,
        tmax,
        residual_fraction: last,
        unimodal_residual: resid,
        class,
        profile: yn,
        times: t,
    })
}

/// `not_in_reference`: absent or tiny in the dark / t0 / blanks (their maximum ≤ the maximum of the profile / 5).
pub fn not_in_reference(y: &[f64], ref_max: f64) -> bool {
    ref_max * 5.0 <= y.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

/// The `kinetics` of `detail` (`descriptors` without `profile`, plus `absent_in_reference`).
pub fn detail_json(o: &mut Out, d: &Descriptors, absent: bool) {
    o.raw("{\"ok\":true,\"onset\":")
        .num(d.onset)
        .raw(",\"tmax\":")
        .num(d.tmax);
    o.raw(",\"residual_fraction\":").num(d.residual_fraction);
    o.raw(",\"unimodal\":")
        .raw(if d.unimodal() { "true" } else { "false" });
    o.raw(",\"unimodal_residual\":")
        .num(d.unimodal_residual)
        .raw(",\"class\":[");
    for (i, c) in d.class.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(c);
    }
    o.raw("],\"class_text\":")
        .str(&d.class_text())
        .raw(",\"times\":")
        .floats(&d.times);
    o.raw(",\"absent_in_reference\":")
        .raw(if absent { "true" } else { "false" })
        .raw("}");
}
