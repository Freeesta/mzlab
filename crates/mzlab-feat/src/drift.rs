//! Correction of the drift of the areas, a view over the aligned table: one factor per cell (row and file), like `norm.rs` but along the order of injection.
//! The areas of the `qc` files of each batch (`disegno/1`) are fitted against the position of the injection with a line or a local line (LOESS, tricube
//! weights); a cell's factor is the reference level of the row over the curve at the injection of its file. The `.mzfeat` and the measured areas are never changed.
//!
//! The correction is refused, with a reason and no number, when a batch has fewer than `MIN_QC` QC files or when the first or the last injected file of a batch
//! (blanks and dark files apart) is not a QC: a curve that is extrapolated at either end is not a correction. A row with too few QC peaks in a batch has NaN
//! factors there (no guess). Sums run in a fixed order, so equal inputs give equal bytes. Reference: `tools/deriva_riferimento.py` (golden `deriva.json`).

use mzlab_core::disegno::{Disegno, LOTTO_PREDEFINITO};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{round, Out};

use crate::aligned::Aligned;
use crate::norm::{bad, measured};
use crate::samples::Sample;

pub const SCHEMA: &str = "deriva/1";
/// QC files a batch needs.
pub const MIN_QC: usize = 5;
/// Points of a row in a batch that a line needs, and that a LOESS needs.
const MIN_POINTS_LINE: usize = 3;
const MIN_POINTS_LOESS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    Linear,
    /// Share of the QC points of the batch in the neighbourhood of each fit, in `(0, 1]`.
    Loess(f64),
}

/// The correction of a table: per cell (`row * n_files + file`) the factor and the curve, per row the reference level and the CV of the QC areas.
#[derive(Debug, Clone, PartialEq)]
pub struct Drift {
    pub method: Method,
    pub n_files: usize,
    /// Batch label of each file.
    pub batch: Vec<String>,
    pub factor: Vec<f64>,
    pub curve: Vec<f64>,
    /// Median of the measured QC areas of the row (all batches): the level the curve is brought to.
    pub reference: Vec<f64>,
    /// CV (%) of the measured QC areas of the row before and after; NaN with fewer than 3 QC peaks or a NaN factor on one of them.
    pub cv_before: Vec<f64>,
    pub cv_after: Vec<f64>,
}

impl Drift {
    /// `deriva/1`: the method, the batches with their QC files, the rows corrected and the median CV of the QC before and after. Numbers only.
    pub fn summary_json(&self, samples: &[Sample]) -> String {
        let n = self.n_files;
        let mut labels: Vec<&String> = self.batch.iter().collect();
        labels.sort();
        labels.dedup();
        let mut o = Out::default();
        o.raw("{\"schema\":").str(SCHEMA).raw(",\"method\":");
        match self.method {
            Method::Linear => o.str("linear").raw(",\"span\":null"),
            Method::Loess(s) => o.str("loess").raw(",\"span\":").num(s),
        };
        o.raw(",\"batches\":[");
        for (i, b) in labels.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            let files = (0..n).filter(|&k| self.batch[k] == **b);
            let qc = files.clone().filter(|&k| samples[k].kind == "qc").count();
            o.raw("{\"batch\":")
                .str(b)
                .raw(",\"files\":")
                .int(files.count() as i64)
                .raw(",\"qc\":")
                .int(qc as i64)
                .raw("}");
        }
        let rows = self.reference.len();
        let corrected = (0..rows)
            .filter(|&r| (0..n).any(|k| self.factor[r * n + k].is_finite()))
            .count();
        let both: Vec<usize> = (0..rows)
            .filter(|&r| self.cv_before[r].is_finite() && self.cv_after[r].is_finite())
            .collect();
        let med = |v: Vec<f64>| median(v);
        o.raw("],\"rows\":")
            .int(rows as i64)
            .raw(",\"rows_corrected\":")
            .int(corrected as i64);
        o.raw(",\"rows_with_cv\":").int(both.len() as i64);
        o.raw(",\"median_cv_before\":").num(round(
            med(both.iter().map(|&r| self.cv_before[r]).collect()),
            4,
        ));
        o.raw(",\"median_cv_after\":").num(round(
            med(both.iter().map(|&r| self.cv_after[r]).collect()),
            4,
        ));
        o.raw("}");
        o.0
    }
}

/// The batch of every file: the one the design gives (by SHA-256 when known, else by name), else the default batch.
pub fn batches_of(samples: &[Sample], d: &Disegno) -> Vec<String> {
    samples
        .iter()
        .map(|s| {
            d.batches
                .iter()
                .find(|b| {
                    (!s.sha256.is_empty() && b.sha256 == s.sha256)
                        || (b.sha256.is_empty() && b.file == s.file)
                })
                .map_or_else(|| LOTTO_PREDEFINITO.to_string(), |b| b.batch.clone())
        })
        .collect()
}

fn median(mut v: Vec<f64>) -> f64 {
    crate::sortc::by(&mut v, f64::total_cmp);
    match v.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

impl Drift {
    /// The curve of one row, for the chart: one point per file with its batch, injection position, QC flag, measured area, curve and factor (`null` where there is none).
    pub fn curve_json(&self, t: &Aligned, samples: &[Sample], row: usize) -> String {
        let n = self.n_files;
        let mut o = Out::default();
        o.raw("{\"schema\":\"deriva_curva/1\",\"row\":")
            .int(row as i64)
            .raw(",\"reference\":")
            .num(round(self.reference[row], 6));
        o.raw(",\"cv_before\":")
            .num(round(self.cv_before[row], 4))
            .raw(",\"cv_after\":")
            .num(round(self.cv_after[row], 4))
            .raw(",\"points\":[");
        for (k, (batch, sample)) in self.batch.iter().zip(samples).enumerate() {
            if k > 0 {
                o.raw(",");
            }
            o.raw("{\"file\":")
                .int(k as i64)
                .raw(",\"batch\":")
                .str(batch)
                .raw(",\"order\":")
                .int(i64::from(sample.order));
            o.raw(",\"qc\":")
                .raw(if sample.kind == "qc" { "true" } else { "false" });
            o.raw(",\"area\":")
                .opt(measured(t, row, k))
                .raw(",\"curve\":")
                .num(self.curve[row * n + k])
                .raw(",\"factor\":")
                .num(self.factor[row * n + k])
                .raw("}");
        }
        o.raw("]}");
        o.0
    }
}

/// CV (%) of the values (sample standard deviation over the mean), NaN with fewer than 3 or a mean that is not positive.
fn cv(v: &[f64]) -> f64 {
    if v.len() < 3 {
        return f64::NAN;
    }
    let m = v.iter().sum::<f64>() / v.len() as f64;
    let var = v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (v.len() - 1) as f64;
    if m > 0.0 {
        100.0 * var.sqrt() / m
    } else {
        f64::NAN
    }
}

/// Least squares line through `(x, y)` evaluated at `at`; the points have at least two different `x`.
fn line_at(x: &[f64], y: &[f64], at: f64) -> f64 {
    wline_at(x, y, &vec![1.0; x.len()], at)
}

/// Weighted least squares line evaluated at `at`; the weighted mean when the abscissas are all the same.
fn wline_at(x: &[f64], y: &[f64], w: &[f64], at: f64) -> f64 {
    let (mut sw, mut sx, mut sy) = (0.0, 0.0, 0.0);
    for ((&xi, &yi), &wi) in x.iter().zip(y).zip(w) {
        sw += wi;
        sx += wi * xi;
        sy += wi * yi;
    }
    let (mx, my) = (sx / sw, sy / sw);
    let (mut sxx, mut sxy) = (0.0, 0.0);
    for ((&xi, &yi), &wi) in x.iter().zip(y).zip(w) {
        sxx += wi * (xi - mx) * (xi - mx);
        sxy += wi * (xi - mx) * (yi - my);
    }
    if sxx <= 1e-12 * sw {
        return my;
    }
    my + sxy / sxx * (at - mx)
}

/// Local line at `at`: the `ceil(span * n)` (at least 3) nearest points, tricube weights with the distance of the farthest of them as the bandwidth.
fn loess_at(x: &[f64], y: &[f64], span: f64, at: f64) -> f64 {
    let n = x.len();
    let k = ((span * n as f64).ceil() as usize).clamp(3.min(n), n);
    let mut d: Vec<f64> = x.iter().map(|v| (v - at).abs()).collect();
    crate::sortc::by(&mut d, f64::total_cmp);
    let h = d[k - 1].max(1e-12);
    // a point at the bandwidth has weight 0; the farthest of the k keeps a tiny one so that k points always count
    let w: Vec<f64> = x
        .iter()
        .map(|v| {
            let u = (v - at).abs() / (h * (1.0 + 1e-9));
            if u < 1.0 {
                (1.0 - u * u * u).powi(3)
            } else {
                0.0
            }
        })
        .collect();
    wline_at(x, y, &w, at)
}

/// Checks the QC files of each batch (`error.drift.tooFewQc`, `error.drift.qcNotAtEnds`).
fn check(samples: &[Sample], batch: &[String]) -> Result<()> {
    let mut labels: Vec<&String> = batch.iter().collect();
    labels.sort();
    labels.dedup();
    for b in labels {
        let mut files: Vec<&Sample> = (0..samples.len())
            .filter(|&k| batch[k] == *b)
            .map(|k| &samples[k])
            .collect();
        crate::sortc::by(&mut files, |p, q| p.order.cmp(&q.order));
        let n_qc = files.iter().filter(|s| s.kind == "qc").count();
        if n_qc < MIN_QC {
            return Err(Error::new("error.drift.tooFewQc")
                .with("batch", b)
                .with("have", n_qc)
                .with("need", MIN_QC));
        }
        let injected: Vec<&&Sample> = files
            .iter()
            .filter(|s| !matches!(s.kind.as_str(), "blank" | "dark"))
            .collect();
        let ends = injected.first().is_some_and(|s| s.kind == "qc")
            && injected.last().is_some_and(|s| s.kind == "qc");
        if !ends {
            return Err(Error::new("error.drift.qcNotAtEnds").with("batch", b));
        }
    }
    Ok(())
}

impl Drift {
    /// The corrected area of a cell, `None` where the file has no peak or the cell has no factor.
    pub fn area(&self, t: &Aligned, row: usize, file: usize) -> Option<f64> {
        let f = self.factor[row * self.n_files + file];
        measured(t, row, file)
            .filter(|_| f.is_finite())
            .map(|a| a * f)
    }
}

/// What `correct` reads for every row: the table, the files, the QC files of each batch in the order of injection.
struct Ctx<'a> {
    t: &'a Aligned,
    samples: &'a [Sample],
    batch: &'a [String],
    labels: Vec<&'a String>,
    qc: Vec<Vec<usize>>,
    method: Method,
}

/// Fits the QC peaks of row `r` batch by batch and writes the curve, the factors and the CVs of the row.
fn fit_row(c: &Ctx, r: usize, out: &mut Drift) {
    let (t, n) = (c.t, c.t.n_files);
    let min_points = match c.method {
        Method::Linear => MIN_POINTS_LINE,
        Method::Loess(_) => MIN_POINTS_LOESS,
    };
    let all: Vec<f64> =
        c.qc.iter()
            .flatten()
            .filter_map(|&k| measured(t, r, k))
            .collect();
    let reference = median(all.clone());
    out.reference[r] = reference;
    out.cv_before[r] = cv(&all);
    for (bi, files) in c.qc.iter().enumerate() {
        let pts: Vec<(f64, f64)> = files
            .iter()
            .filter_map(|&k| measured(t, r, k).map(|a| (f64::from(c.samples[k].order), a)))
            .collect();
        if pts.len() < min_points || !pts.windows(2).any(|w| w[0].0 != w[1].0) {
            continue;
        }
        let (x, y): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
        for k in (0..n).filter(|&k| c.batch[k] == *c.labels[bi]) {
            let at = f64::from(c.samples[k].order);
            let curve = match c.method {
                Method::Linear => line_at(&x, &y, at),
                Method::Loess(span) => loess_at(&x, &y, span, at),
            };
            out.curve[r * n + k] = curve;
            if curve > 0.0 && reference.is_finite() {
                out.factor[r * n + k] = reference / curve;
            }
        }
    }
    let after: Vec<f64> =
        c.qc.iter()
            .flatten()
            .filter_map(|&k| measured(t, r, k).map(|a| a * out.factor[r * n + k]))
            .collect();
    out.cv_after[r] = if after.iter().all(|v| v.is_finite()) {
        cv(&after)
    } else {
        f64::NAN
    };
}

/// The correction of `t` under `method`. `batch` has one label per file (`batches_of`).
pub fn correct(t: &Aligned, samples: &[Sample], batch: &[String], method: Method) -> Result<Drift> {
    let n = t.n_files;
    if samples.len() != n
        || batch.len() != n
        || matches!(method, Method::Loess(s) if !(s > 0.0 && s <= 1.0))
    {
        return Err(bad());
    }
    check(samples, batch)?;
    let mut labels: Vec<&String> = batch.iter().collect();
    labels.sort();
    labels.dedup();
    let qc: Vec<Vec<usize>> = labels
        .iter()
        .map(|b| {
            let mut v: Vec<usize> = (0..n)
                .filter(|&k| batch[k] == **b && samples[k].kind == "qc")
                .collect();
            crate::sortc::by(&mut v, |&p, &q| samples[p].order.cmp(&samples[q].order));
            v
        })
        .collect();
    let rows = t.len();
    let mut out = Drift {
        method,
        n_files: n,
        batch: batch.to_vec(),
        factor: vec![f64::NAN; rows * n],
        curve: vec![f64::NAN; rows * n],
        reference: vec![f64::NAN; rows],
        cv_before: vec![f64::NAN; rows],
        cv_after: vec![f64::NAN; rows],
    };
    let ctx = Ctx {
        t,
        samples,
        batch,
        labels,
        qc,
        method,
    };
    for r in 0..rows {
        fit_row(&ctx, r, &mut out);
    }
    Ok(out)
}
