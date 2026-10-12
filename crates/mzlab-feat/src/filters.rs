//! Filters of the rows of the aligned table, a view over it: an ordered list of rules, each row is kept or rejected by the first rule it fails. The
//! `.mzfeat` is not changed; the list is what the `filtra` operation of the `.mzflow` records.
//!
//! Files that are blanks or dark are never counted as samples. A rule that cannot be evaluated (no blank in the series, no summaries in the table) is
//! an error, never a silent pass.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};

use crate::aligned::{Aligned, CELL_DETECTED, NO_FILE};
use crate::norm::{bad, decides, measured};
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
    /// m/z of the reference ion in `[lo, hi]`.
    Mz(f64, f64),
    /// RT of the reference ion in `[lo, hi]`, minutes.
    Rt(f64, f64),
    /// An MS2 is linked to the peak.
    Ms2,
}

/// Names of the rules in the `filtra` operation, in the order of `Rule`.
const NAMES: [&str; 7] = ["min_rilevati", "min_area", "min_altezza", "bianco", "mz", "rt", "ms2"];

impl Rule {
    fn name(&self) -> &'static str {
        NAMES[match self {
            Rule::MinDetected(_) => 0,
            Rule::MinArea(_) => 1,
            Rule::MinHeight(_) => 2,
            Rule::Blank(_) => 3,
            Rule::Mz(..) => 4,
            Rule::Rt(..) => 5,
            Rule::Ms2 => 6,
        }]
    }
}

/// Whether `row` passes the rule (the blank mean is computed once by the caller).
fn passes(t: &Aligned, samples: &[Sample], row: usize, rule: &Rule, blank: &[usize]) -> bool {
    let files = || (0..t.n_files).filter(|&k| decides(&samples[k]));
    match *rule {
        Rule::MinDetected(n) => files().filter(|&k| t.state[row * t.n_files + k] == CELL_DETECTED).count() >= n,
        Rule::MinArea(x) => files().filter_map(|k| measured(t, row, k)).fold(0.0, f64::max) >= x,
        Rule::MinHeight(x) => files()
            .filter(|&k| measured(t, row, k).is_some())
            .map(|k| f64::from(t.height[row * t.n_files + k]))
            .fold(0.0, f64::max)
            >= x,
        Rule::Blank(ratio) => {
            let b = blank.iter().map(|&k| measured(t, row, k).unwrap_or(0.0)).sum::<f64>() / blank.len() as f64;
            files().filter_map(|k| measured(t, row, k)).fold(0.0, f64::max) >= ratio * b
        }
        Rule::Mz(lo, hi) => (lo..=hi).contains(&t.mz[row]),
        Rule::Rt(lo, hi) => (lo..=hi).contains(&t.rt[row]),
        Rule::Ms2 => t.meta.ms2_file[row] != NO_FILE,
    }
}

/// For every row 0 when it is kept, else the 1-based position in `rules` of the first rule it fails.
pub fn apply(t: &Aligned, samples: &[Sample], rules: &[Rule]) -> Result<Vec<u8>> {
    if samples.len() != t.n_files || rules.len() > 250 {
        return Err(bad());
    }
    let blank: Vec<usize> = (0..t.n_files).filter(|&k| samples[k].kind == "blank").collect();
    if rules.iter().any(|r| matches!(r, Rule::Blank(_))) && blank.is_empty() {
        return Err(Error::new("error.filterNoBlank"));
    }
    if rules.contains(&Rule::Ms2) && t.meta.len() != t.len() {
        return Err(Error::new("error.filterNoSummaries"));
    }
    Ok((0..t.len())
        .map(|row| {
            rules
                .iter()
                .position(|r| !passes(t, samples, row, r, &blank))
                .map_or(0, |i| i as u8 + 1)
        })
        .collect())
}

/// The `filtra` operation: `[{"regola":"min_rilevati","valore":3}, {"regola":"mz","da":100,"a":500}, ...]`.
pub fn to_json(rules: &[Rule]) -> String {
    let mut j = Out::default();
    j.raw("[");
    for (i, r) in rules.iter().enumerate() {
        j.raw(if i > 0 { ",{\"regola\":" } else { "{\"regola\":" }).str(r.name());
        match *r {
            Rule::MinDetected(n) => j.raw(",\"valore\":").int(n as i64),
            Rule::MinArea(x) | Rule::MinHeight(x) | Rule::Blank(x) => j.raw(",\"valore\":").num(x),
            Rule::Mz(a, b) | Rule::Rt(a, b) => j.raw(",\"da\":").num(a).raw(",\"a\":").num(b),
            Rule::Ms2 => &mut j,
        };
        j.raw("}");
    }
    j.raw("]");
    j.0
}

pub fn from_json(v: &Value) -> Result<Vec<Rule>> {
    let num = |o: &Value, k: &str| o.get(k).and_then(Value::f64).filter(|x| x.is_finite()).ok_or_else(bad);
    v.arr()
        .iter()
        .map(|o| {
            Ok(match o.get("regola").and_then(Value::str) {
                Some("min_rilevati") => Rule::MinDetected(num(o, "valore")?.max(0.0) as usize),
                Some("min_area") => Rule::MinArea(num(o, "valore")?),
                Some("min_altezza") => Rule::MinHeight(num(o, "valore")?),
                Some("bianco") => Rule::Blank(num(o, "valore")?),
                Some("mz") => Rule::Mz(num(o, "da")?, num(o, "a")?),
                Some("rt") => Rule::Rt(num(o, "da")?, num(o, "a")?),
                Some("ms2") => Rule::Ms2,
                _ => return Err(bad()),
            })
        })
        .collect()
}
