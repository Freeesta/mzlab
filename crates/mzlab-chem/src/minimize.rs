//! L-BFGS minimiser over flat Cartesian coordinates (written for mzLab); the energy and gradient come from a closure.

/// Minimise `f` starting at `x`; `f(x, grad)` returns the energy and fills the gradient. Returns the final energy.
pub fn lbfgs<F: FnMut(&[f64], &mut [f64]) -> f64>(
    f: &mut F,
    x: &mut [f64],
    max_iter: usize,
) -> f64 {
    let n = x.len();
    let mut g = vec![0.0; n];
    let mut e = f(x, &mut g);
    let mut hist: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
    for _ in 0..max_iter {
        if norm(&g) < 1e-3 {
            break;
        }
        let mut d = direction(&g, &hist);
        let mut slope = dot(&d, &g);
        if slope >= 0.0 {
            d = g.iter().map(|v| -v).collect();
            slope = -dot(&g, &g);
            hist.clear();
        }
        let Some((xn, gn, en)) = line_search(f, x, &d, e, slope) else {
            if hist.is_empty() {
                break;
            }
            hist.clear();
            continue;
        };
        let s: Vec<f64> = xn.iter().zip(x.iter()).map(|(a, b)| a - b).collect();
        let y: Vec<f64> = gn.iter().zip(g.iter()).map(|(a, b)| a - b).collect();
        if dot(&s, &y) > 1e-12 {
            hist.push((s, y));
            if hist.len() > 8 {
                hist.remove(0);
            }
        }
        x.copy_from_slice(&xn);
        g = gn;
        e = en;
    }
    e
}

/// Armijo backtracking along `d`; the step is capped so that no atom moves more than 0.3 Å at once.
fn line_search<F: FnMut(&[f64], &mut [f64]) -> f64>(
    f: &mut F,
    x: &[f64],
    d: &[f64],
    e: f64,
    slope: f64,
) -> Option<(Vec<f64>, Vec<f64>, f64)> {
    let big = d.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let mut step = if big > 0.3 { 0.3 / big } else { 1.0 };
    for _ in 0..30 {
        let xn: Vec<f64> = x.iter().zip(d).map(|(a, b)| a + step * b).collect();
        let mut gn = vec![0.0; x.len()];
        let en = f(&xn, &mut gn);
        if en.is_finite() && en <= e + 1e-4 * step * slope {
            return Some((xn, gn, en));
        }
        step *= 0.5;
    }
    None
}

/// Two-loop recursion: approximate -H·g from the stored (s, y) pairs.
fn direction(g: &[f64], hist: &[(Vec<f64>, Vec<f64>)]) -> Vec<f64> {
    let mut q = g.to_vec();
    let mut alphas = Vec::with_capacity(hist.len());
    for (s, y) in hist.iter().rev() {
        let a = dot(s, &q) / dot(y, s);
        for (qi, yi) in q.iter_mut().zip(y) {
            *qi -= a * yi;
        }
        alphas.push(a);
    }
    if let Some((s, y)) = hist.last() {
        let gamma = dot(s, y) / dot(y, y);
        q.iter_mut().for_each(|v| *v *= gamma);
    }
    for ((s, y), a) in hist.iter().zip(alphas.iter().rev()) {
        let b = dot(y, &q) / dot(y, s);
        for (qi, si) in q.iter_mut().zip(s) {
            *qi += (a - b) * si;
        }
    }
    q.iter().map(|v| -v).collect()
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn norm(a: &[f64]) -> f64 {
    dot(a, a).sqrt()
}
