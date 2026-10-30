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

/// Which blank files a sample file is compared with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matching {
    /// All the blank files (one level for every sample).
    All,
    /// Only the blanks of the same batch; a sample with none falls back to all blanks.
    Batch,
    /// Only the blanks of the same matrix; the same fallback.
    Matrix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlankOpts {
    pub quantity: Quantity,
    pub stat: Stat,
    pub scope: Scope,
    /// The view shows `max(0, value - fold * blank)` for the files that are not blanks (needs `Area` or `NormArea`).
    pub subtract: bool,
    pub matching: Matching,
    /// The internal-standard row of the view's normalisation passes the rule (it is spiked in the blanks too).
    pub keep_istd: bool,
}

impl Default for BlankOpts {
    fn default() -> Self {
        BlankOpts {
            quantity: Quantity::Area,
            stat: Stat::Mean,
            scope: Scope::MaxFeature,
            subtract: false,
            matching: Matching::All,
            keep_istd: false,
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

/// Per-file labels of the series (batch, matrix, series, group) that the matched blank rule and the replicate rule read; empty when no rule needs them. They travel in
/// the spec of the view (`etichette`), so the digest covers them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Labels {
    pub batch: Vec<String>,
    pub matrix: Vec<String>,
    pub series: Vec<String>,
    pub group: Vec<String>,
}

impl Labels {
    pub fn is_empty(&self) -> bool {
        self.batch.is_empty()
            && self.matrix.is_empty()
            && self.series.is_empty()
            && self.group.is_empty()
    }

    fn of(list: &[String], k: usize) -> &str {
        list.get(k).map_or("", String::as_str)
    }

    pub fn series_of(&self, k: usize) -> &str {
        Self::of(&self.series, k)
    }

    pub fn group_of(&self, k: usize) -> &str {
        Self::of(&self.group, k)
    }

    pub fn to_json(&self) -> String {
        let mut j = Out::default();
        j.raw("{");
        for (i, (key, list)) in [
            ("lotto", &self.batch),
            ("matrice", &self.matrix),
            ("serie", &self.series),
            ("gruppo", &self.group),
        ]
        .into_iter()
        .enumerate()
        {
            j.raw(if i > 0 { "," } else { "" }).str(key).raw(":[");
            for (n, x) in list.iter().enumerate() {
                j.raw(if n > 0 { "," } else { "" }).str(x);
            }
            j.raw("]");
        }
        j.raw("}");
        j.0
    }

    /// The `etichette` object; every list is empty or has one entry per file.
    pub fn from_json(v: &Value, n_files: usize) -> Result<Labels> {
        let list = |k: &str| -> Result<Vec<String>> {
            let Some(a) = v.get(k) else {
                return Ok(Vec::new());
            };
            let out: Vec<String> = a
                .arr()
                .iter()
                .map(|x| x.str().map(str::to_string).ok_or_else(bad))
                .collect::<Result<_>>()?;
            if !out.is_empty() && out.len() != n_files {
                return Err(bad());
            }
            Ok(out)
        };
        Ok(Labels {
            batch: list("lotto")?,
            matrix: list("matrice")?,
            series: list("serie")?,
            group: list("gruppo")?,
        })
    }
}

/// The sample files that share one set of blank files, and that set.
pub type Group = (Vec<usize>, Vec<usize>);

/// The comparison groups of a blank rule: with `Matching::All` one group of all `samples` and all `blank` files; else the samples of each batch (or matrix)
/// with the blanks that carry the same label. A sample with no label or no blank of its label joins the last group, with all the blanks. The second value
/// is true per file (sized `n_files`) for the samples that took that fallback.
pub fn blank_groups(
    m: Matching,
    labels: &Labels,
    samples: &[usize],
    blank: &[usize],
    n_files: usize,
) -> (Vec<Group>, Vec<bool>) {
    let list = match m {
        Matching::All => {
            return (
                vec![(samples.to_vec(), blank.to_vec())],
                vec![false; n_files],
            )
        }
        Matching::Batch => &labels.batch,
        Matching::Matrix => &labels.matrix,
    };
    let label = |k: usize| Labels::of(list, k);
    let mut groups: Vec<(String, Group)> = Vec::new();
    let mut fallback = vec![false; n_files];
    let mut rest: Vec<usize> = Vec::new();
    for &k in samples {
        let key = label(k);
        let mine: Vec<usize> = blank.iter().copied().filter(|&b| label(b) == key).collect();
        if key.is_empty() || mine.is_empty() {
            fallback[k] = true;
            rest.push(k);
        } else if let Some(g) = groups.iter_mut().find(|(g, _)| g == key) {
            g.1 .0.push(k);
        } else {
            groups.push((key.to_string(), (vec![k], mine)));
        }
    }
    let mut out: Vec<Group> = groups.into_iter().map(|(_, g)| g).collect();
    if !rest.is_empty() {
        out.push((rest, blank.to_vec()));
    }
    (out, fallback)
}

/// `passes` over several comparison groups: a row is kept when a group with a peak clears its own limit (`MaxFeature`) or when every sample with a peak clears the limit
/// of its group (`EachFeature`). With one group it is `passes`.
pub fn passes_groups(
    t: &Aligned,
    factors: Option<&[f64]>,
    o: &BlankOpts,
    fold: f64,
    row: usize,
    groups: &[Group],
) -> Result<bool> {
    if let [(s, b)] = groups {
        return passes(t, factors, o, fold, row, (s, b));
    }
    let mut any = false;
    let mut seen = false;
    let mut all = true;
    let mut limits_zero = true;
    for (s, b) in groups {
        let limit = fold.max(1.0) * blank_level(t, factors, o, row, b)?;
        limits_zero &= limit <= 0.0;
        let mut top: Option<f64> = None;
        for &k in s {
            if let Some(x) = value(t, factors, o.quantity, row, k)? {
                seen = true;
                top = Some(top.map_or(x, |m| m.max(x)));
                all &= x >= limit;
            }
        }
        any |= top.is_some_and(|m| m >= limit);
    }
    Ok(match o.scope {
        Scope::MaxFeature => any || (!seen && limits_zero),
        Scope::EachFeature => all && (seen || limits_zero),
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
    if o.quantity == Quantity::Height || o.matching != Matching::All {
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
const MATCHING: [(&str, Matching); 3] = [
    ("tutti", Matching::All),
    ("lotto", Matching::Batch),
    ("matrice", Matching::Matrix),
];
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
    if o.matching != Matching::All {
        j.raw(",\"abbinamento\":").str(name(&MATCHING, o.matching));
    }
    if o.keep_istd {
        j.raw(",\"istd_esente\":true");
    }
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
        matching: pick(&MATCHING, v, "abbinamento", d.matching)?,
        keep_istd: v
            .get("istd_esente")
            .map_or(Ok(false), |b| b.bool().ok_or_else(bad))?,
    })
}
