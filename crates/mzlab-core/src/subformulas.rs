//! All the sub-formulas of a formula with the exact m/z of their ions (`mzlab/chem/subformulas.py`): a search tool, never an answer.

use crate::elements::{mass_of, parse_formula, Formula};
use crate::error::{Error, Result};

const ELECTRON: f64 = 0.00054858;
pub const MAX_COMBINATIONS: u64 = 300_000;

fn total(f: &Formula) -> Result<u64> {
    let mut t: u64 = 1;
    for (_, n) in f {
        t = t.saturating_mul(n + 1);
    }
    if t > MAX_COMBINATIONS {
        return Err(Error::new("err.subxic.many").with("n", t));
    }
    Ok(t)
}

/// Sorted m/z of every sub-formula of the ion `formula` (without the names).
pub fn submasses(formula: &str, z: i32, min_mz: f64) -> Result<Vec<f64>> {
    let mut f = parse_formula(formula)?;
    let z = if z == 0 { 1 } else { z };
    f.sort_by(|a, b| a.0.cmp(&b.0));
    total(&f)?;
    let mut mass = vec![0.0f64];
    for (e, n) in &f {
        let m = mass_of(e).unwrap();
        mass = mass
            .iter()
            .flat_map(|&b| (0..=*n).map(move |j| b + m * j as f64))
            .collect();
    }
    let mut mz: Vec<f64> = mass[1..]
        .iter()
        .map(|&m| (m - z as f64 * ELECTRON) / (z.abs() as f64))
        .filter(|&v| v >= min_mz)
        .collect();
    mz.sort_by(f64::total_cmp);
    Ok(mz)
}

/// (parsed formula, [(sub-formula, ion m/z)]) for every sub-formula whose ion m/z is at least `min_mz`, sorted by m/z (stable).
pub fn subformulas(formula: &str, z: i32, min_mz: f64) -> Result<(Formula, Vec<(String, f64)>)> {
    let f = parse_formula(formula)?;
    let z = if z == 0 { 1 } else { z };
    let mut els: Vec<(String, u64)> = f.clone();
    els.sort_by(|a, b| ((a.0 != "C" && a.0 != "H"), &a.0).cmp(&((b.0 != "C" && b.0 != "H"), &b.0)));
    let tot = total(&els)? as usize;
    let k = els.len();
    let mut rows: Vec<(f64, Vec<u64>)> = Vec::with_capacity(tot);
    let mut idx = vec![0u64; k];
    for _ in 0..tot {
        // sum((grid_i * mass_i) for i), added in the order of the elements, starting from 0
        let mut mass = 0.0f64;
        for (i, (e, _)) in els.iter().enumerate() {
            mass += idx[i] as f64 * mass_of(e).unwrap();
        }
        if idx.iter().sum::<u64>() > 0 {
            let mz = (mass - z as f64 * ELECTRON) / (z.abs() as f64);
            if mz >= min_mz {
                rows.push((mz, idx.clone()));
            }
        }
        for d in (0..k).rev() {
            if idx[d] < els[d].1 {
                idx[d] += 1;
                break;
            }
            idx[d] = 0;
        }
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let out = rows
        .into_iter()
        .map(|(mz, ix)| {
            let g: Formula = els
                .iter()
                .zip(&ix)
                .filter(|(_, &c)| c > 0)
                .map(|((e, _), &c)| (e.clone(), c))
                .collect();
            (crate::elements::fmt(&g), mz)
        })
        .collect();
    Ok((f, out))
}
