//! Options of the blank rule and of the QC CV rule of the filters (`bianco`, `qc_cv_max`): the quantity compared, the blank statistic, the scope of
//! the comparison, the subtraction of the blank and the CV that counts the files without a peak as 0. With every option off the rules are the plain
//! `Rule::Blank` and `Rule::QcCv`; `filters` and `view` use this module.

use mzlab_core::error::Result;
use mzlab_core::json::{Out, Value};

use crate::aligned::Aligned;
use crate::norm::{bad, measured};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    Area,
    Height,
    /// Area times the factor of its file (needs the factors of the view).
    NormArea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stat {
    /// Mean over the blank files; a file without a peak counts as 0.
    Mean,
    /// Largest value over the blank files.
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The row is judged once, by its best sample value.
    MaxFeature,
    /// The row is kept only if every sample file with a peak reaches the threshold.
    EachFeature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlankOpts {
    pub quantity: Quantity,
    pub stat: Stat,
    pub scope: Scope,
    /// The view shows `max(0, value - fold * blank)` for the files that are not blanks (needs `Area` or `NormArea`).
    pub subtract: bool,
}

impl Default for BlankOpts {
    fn default() -> Self {
        BlankOpts {
            quantity: Quantity::Area,
            stat: Stat::Mean,
            scope: Scope::MaxFeature,
            subtract: false,
        }
    }
}

fn value(
    t: &Aligned,
    factors: Option<&[f64]>,
    q: Quantity,
    row: usize,
    k: usize,
) -> Result<Option<f64>> {
    let Some(a) = measured(t, row, k) else {
        return Ok(None);
    };
    Ok(match q {
        Quantity::Area => Some(a),
        Quantity::Height => Some(f64::from(t.height[row * t.n_files + k])),
        Quantity::NormArea => {
            let f = factors.ok_or_else(bad)?[k];
            f.is_finite().then_some(a * f)
        }
    })
}

/// The blank level of a row in the quantity of `o` (0 with no blank file).
pub fn blank_level(
    t: &Aligned,
    factors: Option<&[f64]>,
    o: &BlankOpts,
    row: usize,
    blank: &[usize],
) -> Result<f64> {
    let mut v = Vec::with_capacity(blank.len());
    for &k in blank {
        v.push(value(t, factors, o.quantity, row, k)?.unwrap_or(0.0));
    }
    Ok(match o.stat {
        Stat::Mean if !v.is_empty() => v.iter().sum::<f64>() / v.len() as f64,
        Stat::Max => v.iter().copied().fold(0.0, f64::max),
        Stat::Mean => 0.0,
    })
}

/// Whether the row clears `fold` times its blank (comparison `>=`); `fold` is at least 1.
pub fn passes(
    t: &Aligned,
    factors: Option<&[f64]>,
    o: &BlankOpts,
    fold: f64,
    row: usize,
    (samples, blank): (&[usize], &[usize]),
) -> Result<bool> {
    let limit = fold.max(1.0) * blank_level(t, factors, o, row, blank)?;
    let mut vals = Vec::with_capacity(samples.len());
    for &k in samples {
        if let Some(x) = value(t, factors, o.quantity, row, k)? {
            vals.push(x);
        }
    }
    Ok(match o.scope {
        Scope::MaxFeature => vals.iter().copied().fold(0.0, f64::max) >= limit,
        Scope::EachFeature => {
            vals.iter().all(|&x| x >= limit) && (!vals.is_empty() || limit <= 0.0)
        }
    })
}

/// The blank subtraction of a view: the level of every row and the files that are blanks (left as they are).
#[derive(Debug, Clone, PartialEq)]
pub struct Subtraction {
    pub opts: BlankOpts,
    pub fold: f64,
    pub level: Vec<f64>,
    pub is_blank: Vec<bool>,
}

impl Subtraction {
    /// The cell as the view shows it: the area (normalised) minus `fold` times the blank level, not below 0.
    pub fn cell(&self, t: &Aligned, factors: &[f64], row: usize, file: usize) -> Option<f64> {
        let a = measured(t, row, file)?;
        let f = factors[file];
        if self.is_blank[file] {
            return Some(a * f);
        }
        let cut = self.fold * self.level[row];
        Some(match self.opts.quantity {
            Quantity::NormArea => (a * f - cut).max(0.0),
            _ => (a - cut).max(0.0) * f,
        })
    }
}

/// The subtraction a list of rules asks for, if any (the first `BlankWith` with `subtract`); an error with `Height`.
pub fn subtraction(
    t: &Aligned,
    factors: &[f64],
    rules: &[crate::filters::Rule],
    is_blank: &[bool],
) -> Result<Option<Subtraction>> {
    let Some((fold, o)) = rules.iter().find_map(|r| match r {
        crate::filters::Rule::BlankWith(f, o) if o.subtract => Some((*f, *o)),
        _ => None,
    }) else {
        return Ok(None);
    };
    if o.quantity == Quantity::Height {
        return Err(bad());
    }
    let blank: Vec<usize> = (0..t.n_files).filter(|&k| is_blank[k]).collect();
    let mut level = Vec::with_capacity(t.len());
    for row in 0..t.len() {
        level.push(blank_level(t, Some(factors), &o, row, &blank)?);
    }
    Ok(Some(Subtraction {
        opts: o,
        fold: fold.max(1.0),
        level,
        is_blank: is_blank.to_vec(),
    }))
}

/// CV (%) of the areas of `row` over the QC files with the files without a peak counted as 0: sample standard deviation over the mean. `None`
/// (an empty cell, never 0) with fewer than 2 QC files or a mean that is not positive.
pub fn qc_cv_percent(t: &Aligned, row: usize, qc: &[usize]) -> Option<f64> {
    if qc.len() < 2 {
        return None;
    }
    let a: Vec<f64> = qc
        .iter()
        .map(|&k| measured(t, row, k).unwrap_or(0.0))
        .collect();
    let mean = a.iter().sum::<f64>() / a.len() as f64;
    let var = a.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (a.len() - 1) as f64;
    (mean > 0.0).then(|| 100.0 * var.sqrt() / mean)
}

const QUANTITY: [(&str, Quantity); 3] = [
    ("area", Quantity::Area),
    ("altezza", Quantity::Height),
    ("area_norm", Quantity::NormArea),
];
const STAT: [(&str, Stat); 2] = [("media", Stat::Mean), ("massimo", Stat::Max)];
const SCOPE: [(&str, Scope); 2] = [("max", Scope::MaxFeature), ("ciascuna", Scope::EachFeature)];

fn name<T: PartialEq + Copy>(table: &[(&'static str, T)], x: T) -> &'static str {
    table.iter().find(|(_, v)| *v == x).map_or("", |(n, _)| n)
}

fn pick<T: Copy>(table: &[(&str, T)], v: &Value, key: &str, default: T) -> Result<T> {
    match v.get(key) {
        None => Ok(default),
        Some(s) => table
            .iter()
            .find(|(n, _)| s.str() == Some(*n))
            .map(|(_, x)| *x)
            .ok_or_else(bad),
    }
}

/// The fields of the options after `valore`, all of them.
pub fn write_json(j: &mut Out, o: &BlankOpts) {
    j.raw(",\"grandezza\":")
        .str(name(&QUANTITY, o.quantity))
        .raw(",\"statistica\":")
        .str(name(&STAT, o.stat))
        .raw(",\"ambito\":")
        .str(name(&SCOPE, o.scope));
    j.raw(if o.subtract {
        ",\"sottrai\":true"
    } else {
        ",\"sottrai\":false"
    });
}

/// The options of a `bianco` rule (absent fields keep the defaults).
pub fn read_json(v: &Value) -> Result<BlankOpts> {
    let d = BlankOpts::default();
    Ok(BlankOpts {
        quantity: pick(&QUANTITY, v, "grandezza", d.quantity)?,
        stat: pick(&STAT, v, "statistica", d.stat)?,
        scope: pick(&SCOPE, v, "ambito", d.scope)?,
        subtract: v
            .get("sottrai")
            .map_or(Ok(false), |b| b.bool().ok_or_else(bad))?,
    })
}
