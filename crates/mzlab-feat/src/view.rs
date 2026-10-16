//! The view of a table: the rows that pass the filters and the normalisation factors of the files, with the exact parameters that made them. The
//! `.mzfeat` is never changed; the same table, samples and parameters give the same view and the same `digest`.

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::Result;
use mzlab_core::json::{parse, Out, Value};

use crate::aligned::Aligned;
use crate::filters::{self, Rule};
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
}

/// The view of `t` under a method and a list of rules.
pub fn build(t: &Aligned, samples: &[Sample], method: &Method, rules: &[Rule]) -> Result<View> {
    let why = filters::apply(t, samples, rules)?;
    let rows = (0..why.len() as u32)
        .filter(|&r| why[r as usize] == 0)
        .collect();
    Ok(View {
        method: method.clone(),
        rules: rules.to_vec(),
        factors: norm::factors(t, samples, method)?,
        why,
        rows,
    })
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
        norm::normalised(t, &self.factors, self.rows[i] as usize, file)
    }

    /// Normalised area of row `row` of the table (not of the view) in `file`.
    pub fn area_at(&self, t: &Aligned, row: usize, file: usize) -> Option<f64> {
        norm::normalised(t, &self.factors, row, file)
    }

    /// The parameters of both operations: `{"normalizza": ..., "filtra": [...]}` (what the `.mzflow` records).
    pub fn spec_json(&self) -> String {
        format!(
            "{{\"normalizza\":{},\"filtra\":{}}}",
            self.method.to_json(),
            filters::to_json(&self.rules)
        )
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
        j.raw("{\"righe\":")
            .int(self.why.len() as i64)
            .raw(",\"tenute\":")
            .int(self.rows.len() as i64)
            .raw(",\"scartate\":[");
        for i in 0..self.rules.len() {
            let n = self.why.iter().filter(|&&w| w as usize == i + 1).count();
            j.raw(if i > 0 { "," } else { "" }).int(n as i64);
        }
        j.raw("]}");
        j.0
    }
}

/// A view from the JSON of `spec_json` (both keys optional: no normalisation, no filters).
pub fn from_spec(t: &Aligned, samples: &[Sample], spec: &[u8]) -> Result<View> {
    let v = parse(spec)?;
    let method = match v.get("normalizza") {
        Some(m) => Method::from_json(m)?,
        None => Method::None,
    };
    let rules = v
        .get("filtra")
        .map_or(Ok(Vec::new()), |f: &Value| filters::from_json(f))?;
    build(t, samples, &method, &rules)
}
