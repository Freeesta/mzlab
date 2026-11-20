//! Geometry helpers of the picture: the pieces and box of an atom label, box tests and the free angles around an atom.

use super::draw_font::{ADVANCE, CAP};
use mzlab_mol::trig::{angle, len, unit, V};
use mzlab_mol::Mol;

/// The atom's own text and the rest of a custom label, which lies on the far side of it: the symbol and nothing else without a custom text,
/// else the first character (the last one when the text runs to the left) and the others.
fn split_custom(custom: Option<&str>, symbol: &str, left: bool) -> (String, String) {
    let Some(t) = custom else {
        return (symbol.to_string(), String::new());
    };
    let cs: Vec<char> = t.chars().collect();
    let cut = if left {
        cs.len().saturating_sub(1)
    } else {
        1.min(cs.len())
    };
    let (head, tail) = cs.split_at(cut);
    let (head, tail) = (
        head.iter().collect::<String>(),
        tail.iter().collect::<String>(),
    );
    if left {
        (tail, head)
    } else {
        (head, tail)
    }
}

/// A label's pieces `(text, size, rise)` in drawing order and its box. A `custom` text (an edit patch's `label`) replaces the symbol and the
/// hydrogens; it runs away from the bonds (to the left when `left`), the character at the atom being the one next to the bonds.
pub(super) fn label_of(
    m: &Mol,
    i: usize,
    left: bool,
    at: V,
    f: f64,
    custom: Option<&str>,
) -> (Vec<(String, V, f64)>, [f64; 4]) {
    let a = &m.atoms[i];
    let small = 0.7 * f;
    let mut pieces: Vec<(String, f64, f64)> = Vec::new();
    let mut hyd = Vec::new();
    if a.h > 0 && custom.is_none() {
        hyd.push(("H".to_string(), f, 0.0));
        if a.h > 1 {
            hyd.push((a.h.to_string(), small, -0.2 * f));
        }
    }
    let iso = (a.iso > 0).then(|| (a.iso.to_string(), small, 0.38 * f));
    let sign = if a.chg > 0 { "+" } else { "-" };
    let n = a.chg.unsigned_abs();
    let chg = (a.chg != 0).then(|| {
        (
            format!(
                "{}{sign}",
                if n > 1 { n.to_string() } else { String::new() }
            ),
            small,
            0.38 * f,
        )
    });
    let (text, rest) = split_custom(custom, &a.el, left);
    let el = (text, f, 0.0);
    let rest = (!rest.is_empty()).then_some((rest, f, 0.0));
    let k;
    if left {
        pieces.extend(hyd);
        pieces.extend(iso);
        pieces.extend(rest);
        k = pieces.len();
        pieces.push(el);
    } else {
        pieces.extend(iso);
        k = pieces.len();
        pieces.push(el);
        pieces.extend(rest);
        pieces.extend(hyd);
    }
    pieces.extend(chg);
    let width = |p: &(String, f64, f64)| ADVANCE * p.1 * p.0.chars().count() as f64;
    let before: f64 = pieces[..k].iter().map(width).sum();
    let x0 = at[0] - before - width(&pieces[k]) / 2.0;
    let (mut x, mut bx) = (x0, [f64::MAX, f64::MAX, f64::MIN, f64::MIN]);
    let mut runs = Vec::new();
    for p in &pieces {
        let base = at[1] + CAP * f / 2.0 - p.2;
        let w = width(p);
        bx = [
            bx[0].min(x),
            bx[1].min(base - CAP * p.1),
            bx[2].max(x + w),
            bx[3].max(base),
        ];
        runs.push((p.0.clone(), [x, base], p.1));
        x += w;
    }
    (runs, bx)
}

pub(super) fn overlap(a: &[f64; 4], b: &[f64; 4], pad: f64) -> bool {
    a[0] < b[2] + pad && b[0] < a[2] + pad && a[1] < b[3] + pad && b[1] < a[3] + pad
}

/// How far from `at` (inside `bx`) the ray `d` leaves the box grown by `pad`.
pub(super) fn exit(bx: &[f64; 4], at: V, d: V, pad: f64) -> f64 {
    let t = |lo: f64, hi: f64, p: f64, v: f64| {
        if v > 1e-9 {
            (hi + pad - p) / v
        } else if v < -1e-9 {
            (lo - pad - p) / v
        } else {
            f64::MAX
        }
    };
    t(bx[0], bx[2], at[0], d[0]).min(t(bx[1], bx[3], at[1], d[1]))
}

/// Whether the segment `p`-`q` crosses the box (Liang-Barsky).
pub(super) fn seg_hits(p: V, q: V, bx: &[f64; 4]) -> bool {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let d = [q[0] - p[0], q[1] - p[1]];
    for (pk, qk) in [
        (-d[0], p[0] - bx[0]),
        (d[0], bx[2] - p[0]),
        (-d[1], p[1] - bx[1]),
        (d[1], bx[3] - p[1]),
    ] {
        if pk == 0.0 {
            if qk < 0.0 {
                return false;
            }
        } else {
            let r = qk / pk;
            if pk < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    t0 <= t1
}

pub(super) fn norm(v: V) -> V {
    let l = len(v).max(1e-9);
    [v[0] / l, v[1] / l]
}

/// The bisectors of the angles between the bonds `ds`, the widest first (the angle around a lone atom is a full turn).
pub(super) fn gaps(ds: &[V]) -> Vec<V> {
    if ds.is_empty() {
        return vec![[0.0, -1.0]];
    }
    // insertion sorts: the lists are a few entries long (no `sort_by` in the `.wasm`)
    let mut a: Vec<f64> = Vec::new();
    for d in ds {
        let x = angle(*d);
        let k = a.iter().position(|y| *y > x).unwrap_or(a.len());
        a.insert(k, x);
    }
    let tau = mzlab_mol::trig::TAU;
    let mut out: Vec<(f64, f64)> = Vec::new();
    for k in 0..a.len() {
        let next = if k + 1 < a.len() {
            a[k + 1]
        } else {
            a[0] + tau
        };
        let g = (next - a[k], a[k] + (next - a[k]) / 2.0);
        let at = out.iter().position(|o| o.0 < g.0).unwrap_or(out.len());
        out.insert(at, g);
    }
    out.into_iter().map(|(_, ang)| unit(ang)).collect()
}
