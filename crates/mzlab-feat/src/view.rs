//! The view of a table: the rows that pass the filters and the normalisation factors of the files, with the exact parameters that made them. The
//! `.mzfeat` is never changed; the same table, samples and parameters give the same view and the same `digest`.

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

use crate::aligned::Aligned;
use crate::filters::{self, Rule};
use crate::filters_blank::{self, Labels, Subtraction};
use crate::norm::{self, Method};
use crate::samples::Sample;

#[derive(Debug, Clone, PartialEq)]
pub struct View {
    pub method: Method,
    pub rules: Vec<Rule>,
    /// One factor per file (`norm::factors`).
    pub factors: Vec<f64>,
    /// Per row of the table: 0 when kept, else the 1-based rule that rejected it.
    pub why: Vec<u8>,
    /// Rows kept, ascending.
    pub rows: Vec<u32>,
    /// The blank subtraction of a `Rule::BlankWith` that asks for it.
    pub sub: Option<Subtraction>,
    /// The labels of the files the matched blank and replicate rules read (empty when no rule does).
    pub labels: Labels,
    /// Per file, the samples that had no blank of their batch or matrix and were compared with all blanks; `None` when no rule matches blanks.
    pub fallback: Option<Vec<bool>>,
    /// Divisor offset of the replicate SD in the QC document: 1 (sample SD, n - 1, the default) or 0 (population SD, the old behaviour).
    pub cv_ddof: usize,
    /// The `blank` object of the counts: the evidence of every row the first blank rule with options dropped; `None` without such a rule.
    pub blank_evidence: Option<String>,
}

/// The view of `t` under a method and a list of rules.
pub fn build(t: &Aligned, samples: &[Sample], method: &Method, rules: &[Rule]) -> Result<View> {
    build_with(t, samples, method, rules, Labels::default())
}

/// `build` with the labels of the files (batch, matrix, series, group) that the matched blank and the replicate rules read.
pub fn build_with(t: &Aligned, samples: &[Sample], method: &Method, rules: &[Rule], labels: Labels) -> Result<View> {
    let factors = norm::factors(t, samples, method)?;
    let istd_row = match method {
        Method::Istd(r) => Some(norm::standard(t, r)?),
        _ => None,
    };
    let extra = filters::Extra { labels: Some(&labels), istd_row };
    let why = filters::apply_extra(t, samples, rules, Some(&factors), &extra)?;
    let rows = (0..why.len() as u32).filter(|&r| why[r as usize] == 0).collect();
    let sub = filters_blank::subtraction(t, &factors, rules, (samples, &labels))?;
    let fallback = filters::blank_fallback(samples, rules, &labels);
    let blank_evidence = blank_evidence(t, samples, rules, &factors, &why, &labels)?;
    Ok(View { method: method.clone(), rules: rules.to_vec(), factors, why, rows, sub, labels, fallback, cv_ddof: 1, blank_evidence })
}

/// The evidence of the rows the first `BlankWith` rule dropped: per row its id, the deciding value, the blank level of its group and the fold (`null` when
/// no blank level above 0 exists), so the user sees why every row went. `None` when no rule has blank options.
fn blank_evidence(t: &Aligned, samples: &[Sample], rules: &[Rule], factors: &[f64], why: &[u8], labels: &Labels) -> Result<Option<String>> {
    let Some((idx, fold, o)) = rules.iter().enumerate().find_map(|(i, r)| match r {
        Rule::BlankWith(f, o) => Some((i, *f, *o)),
        _ => None,
    }) else {
        return Ok(None);
    };
    let (blank, sample) = crate::filters_nts::blank_and_samples(samples);
    let (groups, _) = filters_blank::blank_groups(o.matching, labels, &sample, &blank, t.n_files);
    let mut j = Out::default();
    j.raw("{\"quantity\":").str(match o.quantity {
        filters_blank::Quantity::Area => "area",
        filters_blank::Quantity::Height => "height",
        filters_blank::Quantity::NormArea => "norm_area",
    });
    j.raw(",\"fold\":").num(fold.max(1.0)).raw(",\"groups\":").int(groups.len() as i64).raw(",\"dropped\":[");
    let mut first = true;
    for (row, &w) in why.iter().enumerate() {
        if w as usize != idx + 1 {
            continue;
        }
        j.raw(if first { "" } else { "," });
        first = false;
        j.raw("[").int(i64::from(t.id(row)));
        match filters_blank::evidence(t, Some(factors), &o, row, &groups)? {
            Some((x, level, f)) => j.raw(",").num(x).raw(",").num(level).raw(",").num(f),
            None => j.raw(",null,0,null"),
        };
        j.raw("]");
    }
    j.raw("]}");
    Ok(Some(j.0))
}

impl View {
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Normalised area of the cell `i` of the view (row `rows[i]`) in `file`.
    pub fn area(&self, t: &Aligned, i: usize, file: usize) -> Option<f64> {
        self.area_at(t, self.rows[i] as usize, file)
    }

    /// Normalised area of row `row` of the table (not of the view) in `file`.
    pub fn area_at(&self, t: &Aligned, row: usize, file: usize) -> Option<f64> {
        match &self.sub {
            Some(s) => s.cell(t, &self.factors, row, file),
            None => norm::normalised(t, &self.factors, row, file),
        }
    }

    /// Height of row `row` of the table in `file`: the measured one, or with the height subtraction the height minus the blank.
    pub fn height_at(&self, t: &Aligned, row: usize, file: usize) -> Option<f64> {
        match &self.sub {
            Some(s) => s.height_cell(t, row, file),
            None => crate::norm::measured(t, row, file).map(|_| f64::from(t.height[row * t.n_files + file])),
        }
    }

    /// The parameters of both operations: `{"normalize": ..., "filter": [...]}` (what the `.mzflow` records).
    pub fn spec_json(&self) -> String {
        let labels = if self.labels.is_empty() { String::new() } else { format!(",\"labels\":{}", self.labels.to_json()) };
        // written only when it is not the default, so the digest of every existing view stays the same
        let ddof = if self.cv_ddof == 1 { String::new() } else { format!(",\"cv_ddof\":{}", self.cv_ddof) };
        format!("{{\"normalize\":{},\"filter\":{}{labels}{ddof}}}", self.method.to_json(), filters::to_json(&self.rules))
    }

    /// SHA-256 of the parameters and of the ids of the rows kept: two equal views have the same digest.
    pub fn digest(&self, t: &Aligned) -> String {
        let mut b = self.spec_json().into_bytes();
        for &r in &self.rows {
            b.extend_from_slice(&t.id(r as usize).to_le_bytes());
        }
        sha256_hex(&b)
    }

    /// Counts for the report: rows in, rows kept and, for every rule, the rows it rejected first.
    pub fn counts_json(&self) -> String {
        let mut j = Out::default();
        j.raw("{\"rows\":").int(self.why.len() as i64).raw(",\"kept\":").int(self.rows.len() as i64).raw(",\"discarded\":[");
        for i in 0..self.rules.len() {
            let n = self.why.iter().filter(|&&w| w as usize == i + 1).count();
            j.raw(if i > 0 { "," } else { "" }).int(n as i64);
        }
        j.raw("]");
        if let Some(f) = &self.fallback {
            j.raw(",\"blank_fallback\":[");
            for (i, &b) in f.iter().enumerate() {
                j.raw(if i > 0 { "," } else { "" }).raw(if b { "true" } else { "false" });
            }
            j.raw("]");
        }
        if let Some(b) = &self.blank_evidence {
            j.raw(",\"blank\":").raw(b);
        }
        j.raw("}");
        j.0
    }
}

/// A view from the JSON of `spec_json` (both keys optional: no normalisation, no filters).
pub fn from_spec(t: &Aligned, samples: &[Sample], spec: &[u8]) -> Result<View> {
    let v = parse(spec)?;
    // a spec saved before the English keys: refuse it with a clear note instead of silently dropping its filters
    if ["normalizza", "filtra", "etichette"].iter().any(|k| v.get(k).is_some()) {
        return Err(Error::new("error.viewSpecOld"));
    }
    let method = match v.get("normalize") {
        Some(m) => Method::from_json(m)?,
        None => Method::None,
    };
    let rules = v.get("filter").map_or(Ok(Vec::new()), |f: &Value| filters::from_json(f))?;
    let labels = match v.get("labels") {
        Some(l) => Labels::from_json(l, t.n_files)?,
        None => Labels::default(),
    };
    let mut view = build_with(t, samples, &method, &rules, labels)?;
    if let Some(d) = v.get("cv_ddof") {
        view.cv_ddof = match d.f64() {
            Some(0.0) => 0,
            Some(1.0) => 1,
            _ => return Err(norm::bad()),
        };
    }
    Ok(view)
}
