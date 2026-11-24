//! Roles and times of the files of a series, `samples/1` (`docs/schemi/samples-1.json`): the one model of the Data view and of mzFinder, with the physical
//! sample, polarity, matrix, batch, date, group and the free typed columns. `propose` reads the names (one parser, times never confirmed), `validate` gives stable
//! codes (the pages put the words), `per_tp` gives the indices by role and the minutes of `feature_pipeline/3`; `json3` is the canonical form (`table_to_json`),
//! `csv`, `model` (name models, `regex`) and `from_file` (what the file says about itself) read and fill it.

mod blank_words;
mod columns;
mod csv;
mod csv_fields;
#[cfg(test)]
mod csv_tests;
mod file_names;
mod from_file;
mod import;
mod json;
mod json3;
mod model;
mod names;
mod order;
#[cfg(test)]
mod order_tests;
mod output_variables;
mod positions;
mod regex;
mod relink;
mod sample_id;
mod time_token;
mod validate;
mod validate3;
mod variable_classes;
mod variables;

pub use columns::{Cell, Column, ColumnType, FileData, Table};
pub use csv::{csv_read, csv_write};
pub use file_names::{clean_sample_name, default_names, mzml_names, Item as NameItem, Named};
pub use from_file::{from_file, summary_from_scans, FileSummary};
pub use import::import;
pub use json::{digest, from_json, from_value, to_json};
pub use json3::{table_digest, table_from_json, table_from_value, table_to_json};
pub use model::{apply_model, Applied};
pub use names::{propose, propose_explained, role_and_minutes, table_kind_of_name};
pub use order::order_like;
pub use positions::{proposals_json, Position, Proposal};
pub use relink::{relink, relink_json, RelinkMatch};
pub use validate::{per_tp, per_tp_json, validate, validate_json, PerTp, Warning};
pub use validate3::validate3;

use crate::error::{Error, Result};

pub const SCHEMA: &str = "samples/1";
/// Roles of a file, in the order of the schema.
pub const TYPES: [&str; 9] = [
    "sample",
    "t0",
    "blank",
    "dark",
    "infusion",
    "std",
    "calibration",
    "qc",
    "pool",
];
/// Kinds of blank (optional: a solvent and a procedure blank are subtracted the same way; a wash only cleans the system and is no reference).
pub const BLANK_TYPES: [&str; 3] = ["solvent", "procedure", "wash"];
/// Units of a time as written: seconds, minutes, hours, days.
pub const TIME_UNITS: [&str; 4] = ["s", "min", "h", "d"];

/// One file of the series.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sample {
    pub file: String,
    /// The sample inside a file that holds several (index or inner name); empty for one sample per file.
    pub sub: String,
    /// A re-injection of a sample (ID suffix `-R`): a distinct injection that the kinetics leave out.
    pub reinjection: bool,
    /// SHA-256 of the raw file: the key (the name can change).
    pub sha256: String,
    /// One of `TIPI`.
    pub kind: String,
    /// One of `BLANK_TYPES`, only for a blank; None when not said.
    pub blank_kind: Option<String>,
    /// The time as written or confirmed, in `time_unit`.
    pub time: Option<f64>,
    /// One of `TIME_UNITS`.
    pub time_unit: String,
    /// Derived from `time` and `time_unit`: the only time used in the computations.
    pub time_min: Option<f64>,
    /// False for a time proposed from the name.
    pub time_confirmed: bool,
    /// The experiment the file belongs to ("" = the only one).
    pub series: String,
    /// 1-based among the files with the same series, role, time and concentration.
    pub replicate: u32,
    pub conc: Option<f64>,
    pub unit: Option<String>,
    /// 1-based position of the injection.
    pub order: u32,
    pub istd: bool,
    pub note: String,
    /// For an infusion: formula or name of the parent (filled by mzFinder).
    pub compound: String,
    // `samples/1` (docs/agenti/samples.md): sample, +1/-1 polarity, matrix, batch (design/1), ISO date and time, group, free columns (sorted by name), what the file says
    pub sample_id: String,
    pub polarity: Option<i8>,
    pub matrix: String,
    pub batch: String,
    pub acquired: Option<String>,
    pub group: String,
    pub extra: Vec<(String, Cell)>,
    pub file_data: Option<FileData>,
}

impl Sample {
    /// An empty `sample` for `file` at position `order` (1-based).
    pub fn new(file: &str, sha256: &str, order: u32) -> Self {
        Sample {
            file: file.to_string(),
            sha256: sha256.to_string(),
            kind: "sample".into(),
            time_unit: "min".into(),
            replicate: 1,
            order,
            ..Default::default()
        }
    }

    /// Sets the time as written and its minutes; a `t0` always has 0 minutes.
    pub fn set_time(&mut self, time: Option<f64>, unit: &str) {
        self.time = time;
        self.time_unit = unit.to_string();
        self.time_min = time.map(|t| t * minutes_per(unit));
    }

    /// The role used by the computations: a `sample` at 0 minutes is a time zero.
    pub fn role(&self) -> &str {
        if self.kind == "sample" && self.time_min == Some(0.0) {
            "t0"
        } else {
            &self.kind
        }
    }
}

/// Minutes in one `unit` (`s`, `min`, `h`, `d`).
pub fn minutes_per(unit: &str) -> f64 {
    match unit {
        "s" => 1.0 / 60.0,
        "h" => 60.0,
        "d" => 1440.0,
        _ => 1.0,
    }
}

/// Removes the repeated values, keeping the first of each in its place (no sort: the size of the WASM).
fn keep_first<T: PartialEq>(v: &mut Vec<T>) {
    let mut i = 0;
    while i < v.len() {
        if v[..i].contains(&v[i]) {
            v.remove(i);
        } else {
            i += 1;
        }
    }
}

type NumberKey<'a> = (
    &'a str,
    &'a str,
    Option<u64>,
    Option<u64>,
    Option<i8>,
    &'a str,
);

/// The replicates counted again: 1-based among the files with the same series, role, minutes, concentration, polarity and matrix.
pub fn number_replicates(cs: &mut [Sample]) {
    // NaN never equals itself, so those files keep the pairwise comparison
    if cs
        .iter()
        .any(|c| c.time_min.is_some_and(f64::is_nan) || c.conc.is_some_and(f64::is_nan))
    {
        return number_replicates_pairwise(cs);
    }
    // 0.0 and -0.0 are equal, hence the same key
    let bits = |x: Option<f64>| x.map(|v| if v == 0.0 { 0 } else { v.to_bits() });
    let mut seen: std::collections::HashMap<NumberKey, u32> =
        std::collections::HashMap::with_capacity(cs.len());
    // the keys borrow the files: the counts are collected first and written after
    let counts: Vec<u32> = cs
        .iter()
        .map(|c| {
            let key = (
                c.series.as_str(),
                c.role(),
                bits(c.time_min),
                bits(c.conc),
                c.polarity,
                c.matrix.as_str(),
            );
            let n = seen.entry(key).or_insert(0);
            *n += 1;
            *n
        })
        .collect();
    for (c, n) in cs.iter_mut().zip(counts) {
        c.replicate = n;
    }
}

fn number_replicates_pairwise(cs: &mut [Sample]) {
    for i in 0..cs.len() {
        let c = &cs[i];
        let same = cs[..i]
            .iter()
            .filter(|o| {
                o.series == c.series
                    && o.role() == c.role()
                    && o.time_min == c.time_min
                    && o.conc == c.conc
                    && o.polarity == c.polarity
                    && o.matrix == c.matrix
            })
            .count();
        cs[i].replicate = same as u32 + 1;
    }
}

/// The bindings (a `samples/1` document is read by every operation that takes one; `route_all` adds `model`, `from_file`, `csv_read`, `csv_write` and `mzml_names`): `op` is
/// `propose` or `propose_explained` (`{"files":[{"name","sha256","kind"}]}`; the second adds the spans of the name), `validate`, `per_tp`, `canonical` or `digest`
/// (a `samples/1`) or `relink` (`{"samples","file":[{"path","sha256"}]}`); JSON in, JSON out. The desktop app serves all of them; the `table` .wasm links only `route_finder`.
pub fn route(op: &str, input: &str) -> Result<String> {
    match op {
        "propose_explained" => {
            let files = files_of(input)?;
            Ok(proposals_json(&propose_explained(&files)))
        }
        "relink" => {
            let v = crate::json::parse(input.as_bytes())?;
            let bad = || Error::new("error.badSamples");
            let cs = from_value(v.get("samples").ok_or_else(bad)?)?;
            let cand: Vec<(String, String)> = v
                .get("file")
                .map(|a| a.arr())
                .unwrap_or_default()
                .iter()
                .map(|f| {
                    let t = |k: &str| f.get(k).and_then(|x| x.str()).unwrap_or("").to_string();
                    (t("path"), t("sha256"))
                })
                .collect();
            Ok(relink_json(&relink(&cs, &cand)))
        }
        _ => route_finder(op, input),
    }
}

/// The `{"files":[{"name","sha256","kind"}]}` of `propose` and its variants.
fn files_of(input: &str) -> Result<Vec<(String, String, String)>> {
    let v = crate::json::parse(input.as_bytes())?;
    let s =
        |f: &crate::json::Value, k: &str| f.get(k).and_then(|x| x.str()).unwrap_or("").to_string();
    Ok(v.get("files")
        .map(|a| a.arr())
        .unwrap_or_default()
        .iter()
        .map(|f| (s(f, "name"), s(f, "sha256"), s(f, "kind")))
        .collect())
}

/// `canonical` and `digest`: the table read from the document, written back (or hashed) in its canonical form.
fn canonical(op: &str, input: &str) -> Result<String> {
    let t = table_from_value(&crate::json::parse(input.as_bytes())?)?;
    Ok(if op == "digest" {
        table_digest(&t)
    } else {
        table_to_json(&t)
    })
}

/// The operations mzFinder calls through the `table` .wasm (`TP_Mine/files/finder-ruoli.js`): `propose`, `validate`, `per_tp`, `canonical`, `digest`.
/// Its .wasm has a cap, so the rest of `route` (`propose_explained`, `relink`) is served by `rotte` in the browser; same bytes, same code.
pub fn route_finder(op: &str, input: &str) -> Result<String> {
    match op {
        "propose" => Ok(to_json(&propose(&files_of(input)?))),
        "validate" => Ok(validate_json(&validate3(&table_from_json(
            input.as_bytes(),
        )?))),
        "per_tp" => Ok(per_tp_json(&per_tp(&from_json(input.as_bytes())?))),
        "canonical" | "digest" => canonical(op, input),
        _ => Err(Error::new("error.badSamples").with("op", op)),
    }
}

/// The operations that read the names and the files (`model`, `from_file`, `csv_read`, `csv_write`) and everything `route` does: the engine of the Data view
/// serves this one. `route_finder` is what mzFinder links (its .wasm has a cap: the name models, the CSV and the rarer operations are not its business).
pub fn route_all(op: &str, input: &str) -> Result<String> {
    match op {
        "from_file" => from_file(input),
        "model" => apply_model(input),
        "csv_read" => csv_read(input),
        "csv_write" => csv_write(input),
        "import" => import(input),
        "mzml_names" => file_names::route(input),
        "variables" => variables::route_variables(input),
        "group_name" => variables::route_group_name(input),
        // the experimental design (`design/1`) rides the same route: `design_propose`, `design_validate`, `design_canonical`, `design_digest`
        _ if op.starts_with("design_") => crate::design::route(&op["design_".len()..], input),
        _ => route(op, input),
    }
}
