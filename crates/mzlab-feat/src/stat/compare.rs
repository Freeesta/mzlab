//! The comparison of conditions: Welch's t per feature for each requested pair, a one-way ANOVA over all the conditions, Benjamini-Hochberg q-values
//! inside each family of tests, and log2FC with a pseudocount. The test works on log2(area + pseudocount) (a missing area is 0), so a feature that is
//! absent in one condition still has a finite effect size. Fewer than `MIN_REPLICHE` files in a condition refuse the comparison with a reason: no number.

use super::special::{f_upper, t_two_sided};
use super::{Layout, Matrix};
use mzlab_core::disegno::MIN_REPLICHE;
use mzlab_core::error::{Error, Result};

/// Smallest pseudocount (area units), also the one used when no file gives a noise level.
pub const FLOOR: f64 = 1000.0;
/// Pseudocount = this x the median of the positive areas of the blank files.
pub const NOISE_FACTOR: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatsOpts {
    /// `None` = estimated from the noise of the blanks (else of the reference, else `FLOOR`).
    pub pseudo: Option<f64>,
    pub floor: f64,
}

impl Default for StatsOpts {
    fn default() -> Self {
        StatsOpts {
            pseudo: None,
            floor: FLOOR,
        }
    }
}

/// One feature in one comparison (`b` against `a`).
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub feature: usize,
    pub mean_a: f64,
    pub mean_b: f64,
    /// log2((mean_b + pseudo) / (mean_a + pseudo)) on the raw areas.
    pub log2fc: f64,
    pub t: f64,
    pub df: f64,
    pub p: f64,
    pub q: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub a: usize,
    pub b: usize,
    /// Why there is no number (`fewReplicates`); then `rows` is empty.
    pub refused: Option<&'static str>,
    pub rows: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnovaRow {
    pub feature: usize,
    pub f: f64,
    pub df1: f64,
    pub df2: f64,
    pub p: f64,
    pub q: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anova {
    pub refused: Option<&'static str>,
    pub rows: Vec<AnovaRow>,
}

/// A refusal in words of the page: the condition and how many files it has.
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub code: &'static str,
    pub group: usize,
    pub n: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatsOut {
    pub pseudo: f64,
    pub pseudo_source: &'static str,
    pub n_features: usize,
    pub comparisons: Vec<Comparison>,
    pub anova: Anova,
    pub warnings: Vec<Warning>,
    /// Conditions of the heat map (the ones with enough files) and, per feature, the z-score of the mean of log2(area + pseudocount) in each.
    pub heat_groups: Vec<usize>,
    pub heat: Vec<Vec<f64>>,
}

fn median(mut v: Vec<f64>) -> f64 {
    crate::sortc::by(&mut v, f64::total_cmp);
    match v.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

/// Area of a cell as the test sees it: finite and positive, else 0 (not detected).
fn area(m: &Matrix, feature: usize, file: usize) -> f64 {
    let a = m[feature].get(file).copied().unwrap_or(0.0);
    if a.is_finite() && a > 0.0 {
        a
    } else {
        0.0
    }
}

/// The pseudocount and where it comes from: the given one, else 3 x the median of the positive areas of the blanks, else of the reference, else the floor.
pub fn pseudocount(m: &Matrix, lay: &Layout, o: &StatsOpts) -> Result<(f64, &'static str)> {
    if let Some(p) = o.pseudo {
        if p.is_nan() || p < 0.0 || o.floor.is_nan() || o.floor < 0.0 {
            return Err(Error::new("error.stats.param"));
        }
        return Ok((p, "given"));
    }
    if o.floor.is_nan() || o.floor < 0.0 {
        return Err(Error::new("error.stats.param"));
    }
    let positives = |files: Vec<usize>| -> Vec<f64> {
        (0..m.len())
            .flat_map(|r| files.iter().map(move |&f| (r, f)))
            .map(|(r, f)| area(m, r, f))
            .filter(|&a| a > 0.0)
            .collect()
    };
    let blanks: Vec<usize> = (0..lay.blank.len()).filter(|&i| lay.blank[i]).collect();
    let reference: Vec<usize> = lay.reference.map(|g| lay.files_of(g)).unwrap_or_default();
    for (files, src) in [(blanks, "blank"), (reference, "reference")] {
        let pos = positives(files);
        if !pos.is_empty() {
            return Ok((o.floor.max(NOISE_FACTOR * median(pos)), src));
        }
    }
    Ok((o.floor, "floor"))
}

fn mean_var(y: &[f64]) -> (f64, f64) {
    let n = y.len() as f64;
    let mean = y.iter().sum::<f64>() / n;
    let var = y.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1.0);
    (mean, var)
}

/// log2(area + pseudo) of a feature over some files.
fn logs(m: &Matrix, r: usize, files: &[usize], pseudo: f64) -> Vec<f64> {
    files
        .iter()
        .map(|&f| libm::log2(area(m, r, f) + pseudo))
        .collect()
}

/// Welch's t of `b` against `a` on the log scale: (t, degrees of freedom, p); NaN when neither group varies.
fn welch(a: &[f64], b: &[f64]) -> (f64, f64, f64) {
    let ((ma, va), (mb, vb)) = (mean_var(a), mean_var(b));
    let (sa, sb) = (va / a.len() as f64, vb / b.len() as f64);
    let se2 = sa + sb;
    if se2.is_nan() || se2 <= 0.0 {
        return if mb == ma {
            (0.0, f64::NAN, 1.0)
        } else {
            (f64::NAN, f64::NAN, f64::NAN)
        };
    }
    let t = (mb - ma) / libm::sqrt(se2);
    let df = se2 * se2 / (sa * sa / (a.len() as f64 - 1.0) + sb * sb / (b.len() as f64 - 1.0));
    (t, df, t_two_sided(t, df))
}

/// Benjamini-Hochberg adjusted p-values; a NaN stays NaN and is not counted among the tests.
pub fn benjamini_hochberg(p: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..p.len()).filter(|&i| p[i].is_finite()).collect();
    crate::sortc::by(&mut idx, |&x, &y| p[x].total_cmp(&p[y]));
    let m = idx.len() as f64;
    let mut q = vec![f64::NAN; p.len()];
    let mut running = 1.0_f64;
    for (rank, &i) in idx.iter().enumerate().rev() {
        running = running.min(p[i] * m / (rank as f64 + 1.0));
        q[i] = running;
    }
    q
}

fn compare_pair(m: &Matrix, lay: &Layout, a: usize, b: usize, pseudo: f64) -> Comparison {
    let (fa, fb) = (lay.files_of(a), lay.files_of(b));
    let few = fa.len().min(fb.len()) < MIN_REPLICHE as usize;
    if few {
        return Comparison {
            a,
            b,
            refused: Some("fewReplicates"),
            rows: Vec::new(),
        };
    }
    let mut rows: Vec<Row> = (0..m.len())
        .map(|r| {
            let (ya, yb) = (logs(m, r, &fa, pseudo), logs(m, r, &fb, pseudo));
            let (t, df, p) = welch(&ya, &yb);
            let raw = |files: &[usize]| {
                files.iter().map(|&f| area(m, r, f)).sum::<f64>() / files.len() as f64
            };
            let (mean_a, mean_b) = (raw(&fa), raw(&fb));
            Row {
                feature: r,
                mean_a,
                mean_b,
                log2fc: libm::log2((mean_b + pseudo) / (mean_a + pseudo)),
                t,
                df,
                p,
                q: f64::NAN,
            }
        })
        .collect();
    let q = benjamini_hochberg(&rows.iter().map(|r| r.p).collect::<Vec<_>>());
    rows.iter_mut().zip(q).for_each(|(r, q)| r.q = q);
    Comparison {
        a,
        b,
        refused: None,
        rows,
    }
}

/// One-way ANOVA over every condition that has files.
fn anova(m: &Matrix, lay: &Layout, groups: &[usize], pseudo: f64) -> Anova {
    let files: Vec<Vec<usize>> = groups.iter().map(|&g| lay.files_of(g)).collect();
    if groups.len() < 2 || files.iter().any(|f| f.len() < MIN_REPLICHE as usize) {
        return Anova {
            refused: Some("fewReplicates"),
            rows: Vec::new(),
        };
    }
    let n_total: usize = files.iter().map(Vec::len).sum();
    let (df1, df2) = ((groups.len() - 1) as f64, (n_total - groups.len()) as f64);
    let mut rows: Vec<AnovaRow> = (0..m.len())
        .map(|r| {
            let ys: Vec<Vec<f64>> = files.iter().map(|f| logs(m, r, f, pseudo)).collect();
            let grand = ys.iter().flatten().sum::<f64>() / n_total as f64;
            let (mut ssb, mut ssw) = (0.0, 0.0);
            for y in &ys {
                let (mean, var) = mean_var(y);
                ssb += y.len() as f64 * (mean - grand) * (mean - grand);
                ssw += var * (y.len() as f64 - 1.0);
            }
            let f = if ssw > 0.0 {
                (ssb / df1) / (ssw / df2)
            } else {
                f64::NAN
            };
            AnovaRow {
                feature: r,
                f,
                df1,
                df2,
                p: f_upper(f, df1, df2),
                q: f64::NAN,
            }
        })
        .collect();
    let q = benjamini_hochberg(&rows.iter().map(|r| r.p).collect::<Vec<_>>());
    rows.iter_mut().zip(q).for_each(|(r, q)| r.q = q);
    Anova {
        refused: None,
        rows,
    }
}

/// Welch's t for each comparison `(a, b)` of conditions of the layout, the ANOVA over all the conditions with files, BH inside each family.
pub fn compare(
    m: &Matrix,
    lay: &Layout,
    comparisons: &[(usize, usize)],
    o: &StatsOpts,
) -> Result<StatsOut> {
    let (pseudo, pseudo_source) = pseudocount(m, lay, o)?;
    let groups: Vec<usize> = (0..lay.names.len())
        .filter(|&g| !lay.files_of(g).is_empty())
        .collect();
    let mut warnings = Vec::new();
    for &g in &groups {
        let n = lay.files_of(g).len();
        if n < MIN_REPLICHE as usize {
            warnings.push(Warning {
                code: "fewReplicates",
                group: g,
                n,
            });
        }
    }
    let comparisons = comparisons
        .iter()
        .map(|&(a, b)| compare_pair(m, lay, a, b, pseudo))
        .collect();
    let heat_groups: Vec<usize> = groups
        .iter()
        .copied()
        .filter(|&g| lay.files_of(g).len() >= MIN_REPLICHE as usize)
        .collect();
    Ok(StatsOut {
        heat: heatmap(m, lay, &heat_groups, pseudo),
        heat_groups,
        pseudo,
        pseudo_source,
        n_features: m.len(),
        comparisons,
        anova: anova(m, lay, &groups, pseudo),
        warnings,
    })
}

/// Per feature, the mean of log2(area + pseudocount) in each condition, centred and divided by the sd over the conditions (0 when it does not vary).
fn heatmap(m: &Matrix, lay: &Layout, groups: &[usize], pseudo: f64) -> Vec<Vec<f64>> {
    if groups.len() < 2 {
        return Vec::new();
    }
    (0..m.len())
        .map(|r| {
            let means: Vec<f64> = groups
                .iter()
                .map(|&g| {
                    let y = logs(m, r, &lay.files_of(g), pseudo);
                    y.iter().sum::<f64>() / y.len() as f64
                })
                .collect();
            let (mean, var) = mean_var(&means);
            let sd = libm::sqrt(var);
            means
                .iter()
                .map(|v| if sd > 0.0 { (v - mean) / sd } else { 0.0 })
                .collect()
        })
        .collect()
}
