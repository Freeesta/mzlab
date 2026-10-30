//! Filters of the rows of the aligned table, a view over it: an ordered list of rules, each row is kept or rejected by the first rule it fails. The
//! `.mzfeat` is not changed; the list is what the `filtra` operation of the `.mzflow` records.
//!
//! Files that are blanks or dark are never counted as samples. A rule that cannot be evaluated (no blank in the series, no summaries in the table) is
//! an error, never a silent pass.

use crate::sortc::SortC;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};

use crate::aligned::{Aligned, CELL_DETECTED, NO_FILE};
use crate::filters_blank::{self as fb, BlankOpts, Labels};
pub use crate::filters_nts::{blank_fallback, replicate_groups, Extra};
use crate::filters_nts::{blank_and_samples, needed};
use crate::norm::{bad, decides, measured};
use crate::quality::REASON_BROAD;
use crate::samples::Sample;

#[derive(Debug, Clone, PartialEq)]
pub enum Rule {
    /// Peaks found by the detection in at least this many files that are not blanks.
    MinDetected(usize),
    /// The largest area over the files is at least this.
    MinArea(f64),
    /// The largest height over the files is at least this.
    MinHeight(f64),
    /// The largest area of a sample is at least `ratio` times the mean area of the blanks (a row with no peak in the blanks passes).
    Blank(f64),
    /// The blank rule with options (`filters_blank`): at least `.0` times (never under 1) the blank level, by the quantity, statistic and scope of
    /// `.1`; with `subtract` the view also shows the areas minus the blank.
    BlankWith(f64, BlankOpts),
    /// m/z of the reference ion in `[lo, hi]`.
    Mz(f64, f64),
    /// RT of the reference ion in `[lo, hi]`, minutes.
    Rt(f64, f64),
    /// An MS2 is linked to the peak.
    Ms2,
    /// CV (%) of the areas over the `qc` files is at most `.0`; the row needs a peak in at least `.1` of them, and the series needs that many `qc` files
    /// (`error.qc.tooFew` otherwise). The CV is the sample standard deviation over the mean of the cells `norm::measured` reads.
    QcCv(f64, usize),
    /// Like `QcCv`, but the files without a peak count as 0 and the row needs no minimum of them (`filters_blank::qc_cv_percent`).
    QcCvZero(f64, usize),
    /// Among rows within `.0` ppm in m/z and `.1` minutes in RT of each other only the one with the largest area is kept (ties: the first row);
    /// a row is judged against the rows kept before it, strongest first, so a chain of near rows does not drop the ones in the middle.
    Duplicates(f64, f64),
    /// A row with a broad-band peak (`REASON_BROAD`) needs the largest area of a sample to be at least `ratio` times the largest area of the blanks;
    /// every other row, and any series with no blank or no summaries, passes (the rule never errors).
    BroadBlank(f64),
    /// In at least one group of replicates (`replicate_groups`) the row is detected in at least this fraction of the files, rounded to a whole file and at
    /// least one.
    MinReplicates(f64),
}

/// QC files needed when the rule does not say.
pub const QC_MIN_FILES: usize = 3;

/// Names of the rules in the `filtra` operation, in the order of `Rule`.
const NAMES: [&str; 11] = [
    "min_rilevati",
    "min_area",
    "min_altezza",
    "bianco",
    "mz",
    "rt",
    "ms2",
    "qc_cv_max",
    "duplicati",
    "bianco_larga",
    "repliche_min",
];

impl Rule {
    pub fn name(&self) -> &'static str {
        NAMES[match self {
            Rule::MinDetected(_) => 0,
            Rule::MinArea(_) => 1,
            Rule::MinHeight(_) => 2,
            Rule::Blank(_) | Rule::BlankWith(..) => 3,
            Rule::Mz(..) => 4,
            Rule::Rt(..) => 5,
            Rule::Ms2 => 6,
            Rule::QcCv(..) | Rule::QcCvZero(..) => 7,
            Rule::Duplicates(..) => 8,
            Rule::BroadBlank(_) => 9,
            Rule::MinReplicates(_) => 10,
        }]
    }
}

/// What the rules share, computed once per table: the blank, QC and sample files, the duplicates, the comparison groups of each blank rule, the groups of
/// replicates and the row of the internal standard.
struct Ctx {
    blank: Vec<usize>,
    qc: Vec<usize>,
    dup: Vec<bool>,
    /// For each `BlankWith` rule, in order, its comparison groups.
    blank_groups: Vec<Vec<fb::Group>>,
    replicates: Vec<Vec<usize>>,
    istd_row: Option<usize>,
}

/// Whether `row` passes the rule (the blank mean is computed once by the caller).
fn passes(
    t: &Aligned,
    samples: &[Sample],
    row: usize,
    rule: &Rule,
    cx: &Ctx,
    nth_blank: usize,
    factors: Option<&[f64]>,
) -> bool {
    let (blank, qc, dup) = (&cx.blank[..], &cx.qc[..], &cx.dup[..]);
    let files = || (0..t.n_files).filter(|&k| decides(&samples[k]));
    match *rule {
        Rule::MinDetected(n) => {
            files()
                .filter(|&k| t.state[row * t.n_files + k] == CELL_DETECTED)
                .count()
                >= n
        }
        Rule::MinArea(x) => {
            files()
                .filter_map(|k| measured(t, row, k))
                .fold(0.0, f64::max)
                >= x
        }
        Rule::MinHeight(x) => {
            files()
                .filter(|&k| measured(t, row, k).is_some())
                .map(|k| f64::from(t.height[row * t.n_files + k]))
                .fold(0.0, f64::max)
                >= x
        }
        Rule::Blank(ratio) => {
            let b = blank
                .iter()
                .map(|&k| measured(t, row, k).unwrap_or(0.0))
                .sum::<f64>()
                / blank.len() as f64;
            files()
                .filter_map(|k| measured(t, row, k))
                .fold(0.0, f64::max)
                >= ratio * b
        }
        Rule::BlankWith(fold, ref o) => {
            (o.keep_istd && cx.istd_row == Some(row))
                || fb::passes_groups(t, factors, o, fold, row, &cx.blank_groups[nth_blank])
                    .unwrap_or(false)
        }
        Rule::MinReplicates(f) => cx.replicates.iter().any(|g| {
            let seen = g
                .iter()
                .filter(|&&k| t.state[row * t.n_files + k] == CELL_DETECTED)
                .count();
            seen >= needed(f, g.len())
        }),
        Rule::QcCvZero(max, _) => fb::qc_cv_percent(t, row, qc).is_some_and(|cv| cv <= max),
        Rule::Mz(lo, hi) => (lo..=hi).contains(&t.mz[row]),
        Rule::Rt(lo, hi) => (lo..=hi).contains(&t.rt[row]),
        Rule::Ms2 => t.meta.ms2_file[row] != NO_FILE,
        Rule::QcCv(max, min_files) => qc_cv(t, row, qc, min_files).is_some_and(|cv| cv <= max),
        Rule::Duplicates(..) => !dup[row],
        Rule::BroadBlank(ratio) => {
            let broad = t.meta.len() == t.len() && t.meta.get(row).qreason & REASON_BROAD != 0;
            if !broad || blank.is_empty() {
                return true;
            }
            let top = |ks: &mut dyn Iterator<Item = usize>| {
                ks.filter_map(|k| measured(t, row, k)).fold(0.0, f64::max)
            };
            top(&mut files()) >= ratio * top(&mut blank.iter().copied())
        }
    }
}

/// CV (%) of the areas of `row` over the QC files, `None` when fewer than `min_files` of them (and fewer than 2) have a peak or the mean is not positive.
fn qc_cv(t: &Aligned, row: usize, qc: &[usize], min_files: usize) -> Option<f64> {
    let a: Vec<f64> = qc.iter().filter_map(|&k| measured(t, row, k)).collect();
    if a.len() < min_files.max(2) {
        return None;
    }
    let mean = a.iter().sum::<f64>() / a.len() as f64;
    let var = a.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (a.len() - 1) as f64;
    (mean > 0.0).then(|| 100.0 * var.sqrt() / mean)
}

/// Rows dropped by `Rule::Duplicates`: strongest first, a row is dropped when a kept row lies in its m/z and RT window.
fn duplicates(t: &Aligned, samples: &[Sample], ppm: f64, rt_min: f64) -> Vec<bool> {
    let n = t.len();
    let peak = |row: usize| {
        (0..t.n_files)
            .filter(|&k| decides(&samples[k]))
            .filter_map(|k| measured(t, row, k))
            .fold(0.0, f64::max)
    };
    let strength: Vec<f64> = (0..n).map(peak).collect();
    let mut by_mz: Vec<usize> = (0..n).collect();
    by_mz.sortc_by(|&a, &b| t.mz[a].total_cmp(&t.mz[b]).then(a.cmp(&b)));
    let mut pos = vec![0usize; n];
    for (i, &r) in by_mz.iter().enumerate() {
        pos[r] = i;
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sortc_by(|&a, &b| strength[b].total_cmp(&strength[a]).then(a.cmp(&b)));
    let (mut kept, mut dropped) = (vec![false; n], vec![false; n]);
    for &row in &order {
        let tol = t.mz[row].abs() * ppm * 1e-6;
        let near = |j: usize| {
            let o = by_mz[j];
            kept[o] && (t.mz[o] - t.mz[row]).abs() <= tol && (t.rt[o] - t.rt[row]).abs() <= rt_min
        };
        let lo = (0..pos[row])
            .rev()
            .take_while(|&j| t.mz[row] - t.mz[by_mz[j]] <= tol)
            .any(near);
        let hi = (pos[row] + 1..n)
            .take_while(|&j| t.mz[by_mz[j]] - t.mz[row] <= tol)
            .any(near);
        if lo || hi {
            dropped[row] = true;
        } else {
            kept[row] = true;
        }
    }
    dropped
}

/// For every row 0 when it is kept, else the 1-based position in `rules` of the first rule it fails.
pub fn apply(t: &Aligned, samples: &[Sample], rules: &[Rule]) -> Result<Vec<u8>> {
    apply_with(t, samples, rules, None)
}

/// `apply` with the factors of the file normalisation, which the blank rule needs when it compares normalised areas.
pub fn apply_with(
    t: &Aligned,
    samples: &[Sample],
    rules: &[Rule],
    factors: Option<&[f64]>,
) -> Result<Vec<u8>> {
    apply_extra(t, samples, rules, factors, &Extra::default())
}

/// The errors a list of rules can raise before any row is judged: no blank, too few QC files, no summaries, missing factors or standard, a bad fraction.
fn check_rules(t: &Aligned, rules: &[Rule], factors: Option<&[f64]>, extra: &Extra, (blank, qc): (&[usize], &[usize])) -> Result<()> {
    if rules
        .iter()
        .any(|r| matches!(r, Rule::Blank(_) | Rule::BlankWith(..)))
        && blank.is_empty()
    {
        return Err(Error::new("error.filterNoBlank"));
    }
    if rules
        .iter()
        .any(|r| matches!(r, Rule::QcCv(_, n) | Rule::QcCvZero(_, n) if qc.len() < (*n).max(2)))
    {
        return Err(Error::new("error.qc.tooFew").with("have", qc.len()));
    }
    if rules.contains(&Rule::Ms2) && t.meta.len() != t.len() {
        return Err(Error::new("error.filterNoSummaries"));
    }
    if factors.is_none()
        && rules
            .iter()
            .any(|r| matches!(r, Rule::BlankWith(_, o) if o.quantity == fb::Quantity::NormArea))
    {
        return Err(bad());
    }
    if rules
        .iter()
        .any(|r| matches!(r, Rule::BlankWith(_, o) if o.keep_istd))
        && extra.istd_row.is_none()
    {
        return Err(bad());
    }
    if rules
        .iter()
        .any(|r| matches!(r, Rule::MinReplicates(f) if !(0.0..=1.0).contains(f)))
    {
        return Err(bad());
    }
    Ok(())
}

/// `apply_with` with the labels and the internal standard that the matched blank, replicate and standard-keeping options read.
pub fn apply_extra(
    t: &Aligned,
    samples: &[Sample],
    rules: &[Rule],
    factors: Option<&[f64]>,
    extra: &Extra,
) -> Result<Vec<u8>> {
    if samples.len() != t.n_files || rules.len() > 250 {
        return Err(bad());
    }
    let empty = Labels::default();
    let labels = extra.labels.unwrap_or(&empty);
    let (blank, sample) = blank_and_samples(samples);
    let qc: Vec<usize> = (0..t.n_files)
        .filter(|&k| samples[k].kind == "qc")
        .collect();
    check_rules(t, rules, factors, extra, (&blank, &qc))?;
    let dup = rules.iter().find_map(|r| match *r {
        Rule::Duplicates(ppm, rt) => Some(duplicates(t, samples, ppm, rt)),
        _ => None,
    });
    let cx = Ctx {
        blank_groups: rules
            .iter()
            .filter_map(|r| match r {
                Rule::BlankWith(_, o) => {
                    Some(fb::blank_groups(o.matching, labels, &sample, &blank, t.n_files).0)
                }
                _ => None,
            })
            .collect(),
        replicates: replicate_groups(samples, labels),
        dup: dup.unwrap_or_default(),
        blank,
        qc,
        istd_row: extra.istd_row,
    };
    let nth: Vec<usize> = rules
        .iter()
        .scan(0usize, |n, r| {
            let here = *n;
            *n += usize::from(matches!(r, Rule::BlankWith(..)));
            Some(here)
        })
        .collect();
    Ok((0..t.len())
        .map(|row| {
            rules
                .iter()
                .enumerate()
                .position(|(i, r)| !passes(t, samples, row, r, &cx, nth[i], factors))
                .map_or(0, |i| i as u8 + 1)
        })
        .collect())
}

/// The `filtra` operation: `[{"regola":"min_rilevati","valore":3}, {"regola":"mz","da":100,"a":500}, ...]`.
pub fn to_json(rules: &[Rule]) -> String {
    let mut j = Out::default();
    j.raw("[");
    for (i, r) in rules.iter().enumerate() {
        j.raw(if i > 0 {
            ",{\"regola\":"
        } else {
            "{\"regola\":"
        })
        .str(r.name());
        match *r {
            Rule::MinDetected(n) => j.raw(",\"valore\":").int(n as i64),
            Rule::MinArea(x)
            | Rule::MinHeight(x)
            | Rule::Blank(x)
            | Rule::BroadBlank(x)
            | Rule::MinReplicates(x) => j.raw(",\"valore\":").num(x),
            Rule::Mz(a, b) | Rule::Rt(a, b) => j.raw(",\"da\":").num(a).raw(",\"a\":").num(b),
            Rule::BlankWith(x, ref o) => {
                j.raw(",\"valore\":").num(x);
                fb::write_json(&mut j, o);
                &mut j
            }
            Rule::Ms2 => &mut j,
            Rule::QcCv(x, n) => j
                .raw(",\"valore\":")
                .num(x)
                .raw(",\"qc_min_files\":")
                .int(n as i64),
            Rule::QcCvZero(x, n) => j
                .raw(",\"valore\":")
                .num(x)
                .raw(",\"qc_min_files\":")
                .int(n as i64)
                .raw(",\"qc_zero\":true"),
            Rule::Duplicates(ppm, rt) => j.raw(",\"ppm\":").num(ppm).raw(",\"rt_min\":").num(rt),
        };
        j.raw("}");
    }
    j.raw("]");
    j.0
}

pub fn from_json(v: &Value) -> Result<Vec<Rule>> {
    let num = |o: &Value, k: &str| {
        o.get(k)
            .and_then(Value::f64)
            .filter(|x| x.is_finite())
            .ok_or_else(bad)
    };
    v.arr()
        .iter()
        .map(|o| {
            Ok(match o.get("regola").and_then(Value::str) {
                Some("min_rilevati") => Rule::MinDetected(num(o, "valore")?.max(0.0) as usize),
                Some("min_area") => Rule::MinArea(num(o, "valore")?),
                Some("min_altezza") => Rule::MinHeight(num(o, "valore")?),
                Some("bianco") => {
                    let opts = fb::read_json(o)?;
                    let fold = num(o, "valore")?;
                    if opts == BlankOpts::default() {
                        Rule::Blank(fold)
                    } else {
                        Rule::BlankWith(fold.max(1.0), opts)
                    }
                }
                Some("bianco_larga") => Rule::BroadBlank(num(o, "valore")?),
                Some("repliche_min") => Rule::MinReplicates(num(o, "valore")?),
                Some("mz") => Rule::Mz(num(o, "da")?, num(o, "a")?),
                Some("rt") => Rule::Rt(num(o, "da")?, num(o, "a")?),
                Some("ms2") => Rule::Ms2,
                Some("qc_cv_max") => {
                    let n = o
                        .get("qc_min_files")
                        .map_or(Ok(QC_MIN_FILES as f64), |_| num(o, "qc_min_files"))?;
                    let (max, n) = (num(o, "valore")?.max(0.0), n.max(0.0) as usize);
                    match o.get("qc_zero") {
                        None => Rule::QcCv(max, n),
                        Some(z) if z.bool() == Some(false) => Rule::QcCv(max, n),
                        Some(z) if z.bool() == Some(true) => Rule::QcCvZero(max, n),
                        Some(_) => return Err(bad()),
                    }
                }
                Some("duplicati") => {
                    Rule::Duplicates(num(o, "ppm")?.max(0.0), num(o, "rt_min")?.max(0.0))
                }
                _ => return Err(bad()),
            })
        })
        .collect()
}
