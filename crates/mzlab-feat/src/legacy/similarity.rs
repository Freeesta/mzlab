//! The similarity of the MS2 of the parent and of a product in three components (`TP_Mine/py/tpmine/hr/similarity.py`: `prepare`, `entropy_weights`,
//! `_typed_matching`, `pair_components`): equal fragments, shifted fragments and common neutral losses, for the modified cosine and the entropy.

use crate::sortc::SortC;
use mzlab_core::json::Out;

/// Peaks kept per spectrum.
pub const K: usize = 32;
/// Tolerance of the matching, Da.
pub const TOL_HR: f64 = 0.005;
const LOSS_MAX: f64 = 65.0;
const LOSS_FRACTION: f64 = 0.35;
const TYPES: [&str; 3] = ["equal", "shifted", "loss"];

/// Peaks ready for comparison: m/z ascending and weights (sqrt of the intensities, unit L2 norm).
pub type Spec = (Vec<f64>, Vec<f64>);

/// `prepare`: precursor region removed, peaks below 1 % of the strongest left out, the 32 strongest kept, sqrt, unit norm, sorted by m/z.
pub fn prepare(mz: &[f64], inten: &[f64], precursor: f64) -> Spec {
    let keep: Vec<(f64, f64)> = mz
        .iter()
        .zip(inten)
        .filter(|&(&m, &i)| m < precursor - 1.5 && i > 0.0)
        .map(|(&m, &i)| (m, i))
        .collect();
    if keep.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let top = keep.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let mut v: Vec<(f64, f64)> = keep.into_iter().filter(|p| p.1 >= 0.01 * top).collect();
    v.sortc_by(|a, b| b.1.total_cmp(&a.1));
    v.truncate(K);
    let w: Vec<f64> = v.iter().map(|p| p.1.sqrt()).collect();
    let norm = w.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mut s: Vec<(f64, f64)> = v.iter().zip(&w).map(|(p, &x)| (p.0, x / norm)).collect();
    s.sortc_by(|a, b| a.0.total_cmp(&b.0));
    s.into_iter().unzip()
}

/// `entropy_weights` of one spectrum: the intensities (w²) as probabilities, flattened as p^(0.25 + 0.25 S) when the entropy S is below 3.
pub fn entropy_weights(w: &[f64]) -> Vec<f64> {
    let p: Vec<f64> = w.iter().map(|x| x * x).collect();
    let tot: f64 = p.iter().sum();
    let p: Vec<f64> = if tot > 0.0 {
        p.iter().map(|x| x / tot).collect()
    } else {
        vec![0.0; p.len()]
    };
    let s = -p
        .iter()
        .map(|&x| if x > 0.0 { x * x.ln() } else { 0.0 })
        .sum::<f64>();
    let e = if s < 3.0 { 0.25 + 0.25 * s } else { 1.0 };
    let q: Vec<f64> = p
        .iter()
        .map(|&x| if x > 0.0 { x.powf(e) } else { 0.0f64.powf(e) })
        .collect();
    let tot: f64 = q.iter().sum();
    if tot > 0.0 {
        q.iter().map(|x| x / tot).collect()
    } else {
        vec![0.0; q.len()]
    }
}

fn xlogx(x: f64) -> f64 {
    if x > 0.0 {
        x * x.ln()
    } else {
        0.0
    }
}

/// `_typed_matching`: global greedy one-to-one matching (heaviest first, ties by row-major position) of the peaks of A with those of B at the same m/z
/// or shifted by `delta`. Returns (i, j, type index, contribution) heaviest first.
fn typed_matching(
    a: (&[f64], &[f64]),
    b: (&[f64], &[f64]),
    delta: f64,
    prec_a: f64,
    tol: f64,
    weight: impl Fn(f64, f64) -> f64,
) -> Vec<(usize, usize, usize, f64)> {
    let (ma, wa) = a;
    let (mb, wb) = b;
    let mut cand: Vec<(usize, usize, f64, bool)> = Vec::new();
    for i in 0..ma.len() {
        for j in 0..mb.len() {
            let d = mb[j] - ma[i];
            let eq = d.abs() <= tol;
            if eq || (d - delta).abs() <= tol {
                let v = weight(wa[i], wb[j]);
                if v > 0.0 {
                    cand.push((i, j, v, eq));
                }
            }
        }
    }
    cand.sortc_by(|x, y| y.2.total_cmp(&x.2));
    let (mut ua, mut ub) = (vec![false; ma.len()], vec![false; mb.len()]);
    let mut out = Vec::new();
    for (i, j, v, eq) in cand {
        if ua[i] || ub[j] {
            continue;
        }
        ua[i] = true;
        ub[j] = true;
        let kind = if eq {
            0
        } else if prec_a - ma[i] <= LOSS_MAX.min(LOSS_FRACTION * prec_a) {
            2
        } else {
            1
        };
        out.push((i, j, kind, v));
    }
    out
}

/// The result of `pair_components`.
#[derive(Debug, Clone, Default)]
pub struct Components {
    pub delta: f64,
    pub cos: [f64; 3],
    pub ent: [f64; 3],
    pub n: [usize; 3],
    /// (m/z in A, m/z in B, type index, weight) of the cosine matching.
    pub pairs: Vec<(f64, f64, usize, f64)>,
}

impl Components {
    pub fn cos_total(&self) -> f64 {
        (self.cos[0] + self.cos[1] + self.cos[2]).min(1.0)
    }
    pub fn ent_total(&self) -> f64 {
        (self.ent[0] + self.ent[1] + self.ent[2]).min(1.0)
    }

    /// `{delta, cos{equal, shifted, loss, total}, ent{...}, n{...}, pairs[{a, b, type, w}]}` as the Python writes it.
    pub fn json(&self, o: &mut Out) {
        o.raw("{\"delta\":").num(self.delta);
        for (key, v, t) in [
            ("cos", &self.cos, self.cos_total()),
            ("ent", &self.ent, self.ent_total()),
        ] {
            o.raw(",\"").raw(key).raw("\":{");
            for (k, name) in TYPES.iter().enumerate() {
                o.raw("\"").raw(name).raw("\":").num(v[k]).raw(",");
            }
            o.raw("\"total\":").num(t).raw("}");
        }
        o.raw(",\"n\":{");
        for (k, name) in TYPES.iter().enumerate() {
            o.raw(if k > 0 { ",\"" } else { "\"" })
                .raw(name)
                .raw("\":")
                .int(self.n[k] as i64);
        }
        o.raw("},\"pairs\":[");
        for (k, p) in self.pairs.iter().enumerate() {
            o.raw(if k > 0 { ",{\"a\":" } else { "{\"a\":" })
                .num(p.0)
                .raw(",\"b\":")
                .num(p.1);
            o.raw(",\"type\":")
                .str(TYPES[p.2])
                .raw(",\"w\":")
                .num(p.3)
                .raw("}");
        }
        o.raw("]}");
    }
}

/// `pair_components(spec_a, prec_a, spec_b, prec_b)`: A is the parent, B the product (high resolution, `TOL_HR`).
pub fn pair_components(a: &Spec, prec_a: f64, b: &Spec, prec_b: f64) -> Components {
    pair_components_tol(a, prec_a, b, prec_b, TOL_HR)
}

/// `pair_components` with the matching tolerance `tol` in Da (0.5 for unit-resolution spectra).
pub fn pair_components_tol(a: &Spec, prec_a: f64, b: &Spec, prec_b: f64, tol: f64) -> Components {
    let mut r = Components {
        delta: prec_b - prec_a,
        ..Default::default()
    };
    if a.0.is_empty() || b.0.is_empty() {
        return r;
    }
    for (i, j, k, v) in typed_matching((&a.0, &a.1), (&b.0, &b.1), r.delta, prec_a, tol, |x, y| {
        x * y
    }) {
        r.cos[k] += v;
        r.n[k] += 1;
        r.pairs.push((a.0[i], b.0[j], k, v));
    }
    let (qa, qb) = (entropy_weights(&a.1), entropy_weights(&b.1));
    let two_ln2 = 2.0 * std::f64::consts::LN_2;
    let ent = |x: f64, y: f64| {
        if x > 0.0 && y > 0.0 {
            (xlogx(x + y) - xlogx(x) - xlogx(y)) / two_ln2
        } else {
            0.0
        }
    };
    for (_, _, k, v) in typed_matching((&a.0, &qa), (&b.0, &qb), r.delta, prec_a, tol, ent) {
        r.ent[k] += v;
    }
    r
}
