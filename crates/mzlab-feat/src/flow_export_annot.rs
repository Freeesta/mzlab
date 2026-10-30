//! The text of `export.annotations@1`: one CSV with, for every feature, the candidates of every identification method of the plan side by side (standard, library, in silico, suspect, formula)
//! with the level and the evidence `annot.level` decided. Bytes in, text out: `flow run` (`mzlab-cli`) and the browser module `flowtab` (`mzlab-feat-wasm`) both call it, so they write the same bytes.
//! It adds no identity and raises no level. A candidate joins its feature by id, the rule of `annot.level` (query id = feature id as text); a row that cannot be joined keeps an empty `feature_id`.

use crate::aligned::Aligned;
use crate::store::read_table;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Value};

/// A file the step reads: its name and its bytes.
pub type Named<'a> = (&'a str, &'a [u8]);

const LEVELS: &str = include_str!("../data/livelli-1.json");
const HEADER: [&str; 18] = [
    "feature_id",
    "mz",
    "rt_min",
    "level",
    "level_why",
    "method",
    "rank",
    "name",
    "inchikey",
    "formula",
    "adduct",
    "score",
    "score_name",
    "delta_ppm",
    "delta_rt_min",
    "matched_fragments",
    "listed_in",
    "source_file",
];
/// The methods in the order the rows are written.
const METHODS: [&str; 5] = ["standard", "library", "in silico", "suspect", "formula"];

/// The parameters of the step.
#[derive(Clone, Debug)]
pub struct Params {
    pub top_per_method: usize,
    /// Only features at or above this level are written; empty = all.
    pub min_level: String,
    pub separator: char,
}

impl Params {
    /// From the resolved parameters of the step (or the object the page sends); a missing one has the default of the descriptor.
    pub fn from_params(p: &[(String, Value)]) -> Params {
        let get = |n: &str| p.iter().find(|(k, _)| k == n).map(|(_, v)| v);
        Params {
            top_per_method: get("top_per_method")
                .and_then(Value::f64)
                .filter(|v| *v >= 1.0)
                .map_or(10, |v| v as usize),
            min_level: get("min_level")
                .and_then(Value::str)
                .unwrap_or("")
                .to_string(),
            separator: match get("separator").and_then(Value::str) {
                Some("semicolon") => ';',
                _ => ',',
            },
        }
    }
}

/// The files of the plan; every port but `table` may be empty.
#[derive(Default)]
pub struct Inputs<'a> {
    pub table: &'a [u8],
    /// `candidates/1` files: the method is read from `step` and `algorithm` of each.
    pub candidates: &'a [Named<'a>],
    pub screen: &'a [Named<'a>],
    pub formulas: &'a [Named<'a>],
    pub annot: Option<Named<'a>>,
}

/// The CSV and how many rows could not be joined to a feature of the table.
pub struct Made {
    pub csv: String,
    pub unjoined: usize,
}

/// One candidate before it is written.
#[derive(Default)]
struct Row {
    /// 1-based row of the table; 0 = not joined (the query id is in `source`).
    feature: usize,
    method: usize,
    name: String,
    inchikey: String,
    formula: String,
    adduct: String,
    score: Option<f64>,
    score_name: String,
    delta_ppm: Option<f64>,
    delta_rt: Option<f64>,
    fragments: Option<f64>,
    listed_in: String,
    source: String,
}

/// What `annot/1` decided for a feature.
struct Decision {
    level: String,
    why: String,
}

fn text(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::str).unwrap_or("").to_string()
}

fn num(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::f64).filter(|x| x.is_finite())
}

/// A number with `d` decimals, without trailing zeros; empty when absent.
fn fmt(v: Option<f64>, d: usize) -> String {
    v.filter(|x| x.is_finite())
        .map_or(String::new(), |x| format!("{}", round(x, d) + 0.0))
}

/// The feature a query id names: the id of a row of the table as text (`annot.level` keys its rows the same way); None for anything else.
fn feature_of(id: &str, n: usize) -> Option<usize> {
    id.parse::<usize>()
        .ok()
        .filter(|k| (1..=n).contains(k) && k.to_string() == id)
}

/// Position of `level` in the scale of `livelli-1.json` (the best first).
fn level_rank(level: &str) -> Option<usize> {
    json::parse(LEVELS.as_bytes())
        .ok()?
        .get("livelli")?
        .arr()
        .iter()
        .position(|l| l.str() == Some(level))
}

fn quote(s: &str, sep: char) -> String {
    if s.contains([sep, '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The decisions of the `annot/1` file by feature (index = feature - 1) and the standard rows it carries.
fn read_annot(file: Option<Named>, n: usize, rows: &mut Vec<Row>) -> Result<Vec<Option<Decision>>> {
    let mut out: Vec<Option<Decision>> = (0..n).map(|_| None).collect();
    let Some((name, bytes)) = file else {
        return Ok(out);
    };
    let doc = json::parse(bytes)?;
    for r in doc.get("rows").map(Value::arr).unwrap_or(&[]) {
        let id = text(r, "id");
        let Some(k) = feature_of(&id, n) else {
            continue;
        };
        out[k - 1] = Some(Decision {
            level: text(r, "level"),
            why: text(r, "why"),
        });
        if let Some(s) = r.get("standard") {
            let gate = |g: &str| {
                s.get("gates")
                    .map(Value::arr)
                    .unwrap_or(&[])
                    .iter()
                    .find(|x| {
                        x.get("id")
                            .and_then(Value::str)
                            .is_some_and(|i| i.starts_with(g))
                    })
                    .and_then(|x| num(x, "value"))
            };
            rows.push(Row {
                feature: k,
                method: 0,
                name: text(s, "name"),
                inchikey: if text(r, "why") == "standard" {
                    text(r, "entry")
                } else {
                    String::new()
                },
                adduct: text(s, "adduct"),
                score: gate("stdscore"),
                score_name: if gate("stdscore_cos").is_some() {
                    "cosine"
                } else {
                    "spectral_entropy"
                }
                .into(),
                delta_ppm: gate("stdppm"),
                delta_rt: gate("stdrt"),
                source: name.to_string(),
                ..Row::default()
            });
        }
    }
    Ok(out)
}

/// The hits of the `candidates/1` files: library or in silico by the `step` of the file, else by the `source` of the hit.
fn read_candidates(files: &[Named], n: usize, rows: &mut Vec<Row>) -> Result<()> {
    for (name, bytes) in files {
        let doc = json::parse(bytes)?;
        let file_silico = text(&doc, "step").starts_with("spectra.insilico");
        for h in doc.get("hits").map(Value::arr).unwrap_or(&[]) {
            let query = text(h, "query");
            let silico = file_silico || text(h, "source") == "insilico";
            let algorithm = match text(h, "algorithm") {
                a if !a.is_empty() => a,
                _ => text(&doc, "algorithm"),
            };
            let joined = feature_of(&query, n);
            rows.push(Row {
                feature: joined.unwrap_or(0),
                method: if silico { 2 } else { 1 },
                name: text(h, "name"),
                inchikey: text(h, "inchikey"),
                formula: text(h, "formula"),
                adduct: text(h, "adduct"),
                score: num(h, "score"),
                score_name: if silico {
                    "insilico".to_string()
                } else {
                    algorithm
                },
                delta_ppm: num(h, "mass_ppm"),
                delta_rt: num(h, "rt_delta_min"),
                fragments: num(h, "shared"),
                source: if joined.is_some() {
                    name.to_string()
                } else {
                    format!("{name}:{query}")
                },
                ..Row::default()
            });
        }
    }
    Ok(())
}

/// The candidates of the `screen/1` files: one row per structure of an isomer group, with the fragments found and the lists that name it.
fn read_screen(files: &[Named], n: usize, rows: &mut Vec<Row>) -> Result<()> {
    for (name, bytes) in files {
        let doc = json::parse(bytes)?;
        for h in doc.get("hits").map(Value::arr).unwrap_or(&[]) {
            let id = h
                .get("feature")
                .and_then(Value::f64)
                .map_or(String::new(), |f| format!("{}", f as i64));
            let joined = feature_of(&id, n);
            for c in h.get("candidates").map(Value::arr).unwrap_or(&[]) {
                let listed: Vec<String> = c
                    .get("memberships")
                    .map(Value::arr)
                    .unwrap_or(&[])
                    .iter()
                    .map(|m| text(m, "list"))
                    .collect();
                rows.push(Row {
                    feature: joined.unwrap_or(0),
                    method: 3,
                    name: text(c, "name"),
                    inchikey: text(c, "inchikey"),
                    formula: text(h, "formula"),
                    adduct: text(h, "adduct"),
                    score: num(c, "fragments_matched"),
                    score_name: "fragments".into(),
                    delta_ppm: num(h, "delta_ppm"),
                    delta_rt: num(c, "rt_diff_min"),
                    fragments: num(c, "fragments_matched"),
                    listed_in: listed.join("|"),
                    source: if joined.is_some() {
                        name.to_string()
                    } else {
                        format!("{name}:{id}")
                    },
                });
            }
        }
    }
    Ok(())
}

/// The candidate formulas of `formulas/1` files: `formula.ms1` (one object per feature) and `formula.ms2` (one per spectrum, each candidate naming its feature).
fn read_formulas(files: &[Named], n: usize, rows: &mut Vec<Row>) -> Result<()> {
    for (name, bytes) in files {
        let doc = json::parse(bytes)?;
        for f in doc.get("features").map(Value::arr).unwrap_or(&[]) {
            let id = text(f, "id");
            let joined = feature_of(&id, n);
            let list = f.get("result").and_then(|r| r.get("candidates"));
            for c in list.map(Value::arr).unwrap_or(&[]) {
                rows.push(Row {
                    feature: joined.unwrap_or(0),
                    method: 4,
                    formula: text(c, "neutral"),
                    adduct: text(f, "ion"),
                    score: num(c, "score"),
                    score_name: "formula_ms1".into(),
                    delta_ppm: num(c, "error_ppm"),
                    source: if joined.is_some() {
                        name.to_string()
                    } else {
                        format!("{name}:{id}")
                    },
                    ..Row::default()
                });
            }
        }
        for s in doc.get("spectra").map(Value::arr).unwrap_or(&[]) {
            for c in s.get("candidates").map(Value::arr).unwrap_or(&[]) {
                let id = text(c, "feature");
                let joined = feature_of(&id, n);
                rows.push(Row {
                    feature: joined.unwrap_or(0),
                    method: 4,
                    formula: text(c, "formula"),
                    adduct: text(c, "adduct"),
                    score: num(c, "score"),
                    score_name: "formula_ms2".into(),
                    delta_ppm: num(c, "ppm_ion"),
                    fragments: num(c, "n_explained"),
                    source: if joined.is_some() {
                        name.to_string()
                    } else {
                        format!("{name}:{}", text(s, "query"))
                    },
                    ..Row::default()
                });
            }
        }
    }
    Ok(())
}

/// The best first by the own score of the method; the order of the file breaks a tie.
fn best_first(a: &Row, b: &Row) -> std::cmp::Ordering {
    let s = |r: &Row| r.score.unwrap_or(f64::NEG_INFINITY);
    s(b).total_cmp(&s(a))
}

/// The 18 cells of one row, in the order of `HEADER`.
fn cells_of(r: &Row, d: Option<&Decision>, rank: usize, table: &Aligned) -> [String; 18] {
    let at = r.feature.checked_sub(1);
    let (mz, rt) = (at.map(|i| table.mz[i]), at.map(|i| table.rt[i]));
    [
        if r.feature > 0 {
            r.feature.to_string()
        } else {
            String::new()
        },
        fmt(mz, 6),
        fmt(rt, 4),
        d.map_or(String::new(), |d| d.level.clone()),
        d.map_or(String::new(), |d| d.why.clone()),
        METHODS[r.method].to_string(),
        rank.to_string(),
        r.name.clone(),
        r.inchikey.clone(),
        r.formula.clone(),
        r.adduct.clone(),
        fmt(r.score, 6),
        r.score_name.clone(),
        fmt(r.delta_ppm, 3),
        fmt(r.delta_rt, 3),
        fmt(r.fragments, 0),
        r.listed_in.clone(),
        r.source.clone(),
    ]
}

/// `export.annotations`: the CSV of the inputs under `p`.
pub fn export_made(p: &Params, inp: &Inputs) -> Result<Made> {
    let min =
        if p.min_level.is_empty() {
            None
        } else {
            Some(level_rank(&p.min_level).ok_or_else(|| {
                Error::new("err.flow.annotations.level").with("level", &p.min_level)
            })?)
        };
    let (table, _) = read_table(inp.table)?;
    let n = table.len();
    let mut rows: Vec<Row> = Vec::new();
    let decisions = read_annot(inp.annot, n, &mut rows)?;
    read_candidates(inp.candidates, n, &mut rows)?;
    read_screen(inp.screen, n, &mut rows)?;
    read_formulas(inp.formulas, n, &mut rows)?;
    // stable sort: feature, method, then the score; rank counts inside (feature, method)
    rows.sort_by(|a, b| {
        let key = |r: &Row| {
            (
                if r.feature == 0 {
                    usize::MAX
                } else {
                    r.feature
                },
                r.method,
            )
        };
        key(a).cmp(&key(b)).then_with(|| best_first(a, b))
    });
    let sep = p.separator;
    let mut csv = HEADER.join(&sep.to_string());
    csv.push('\n');
    let (mut unjoined, mut last, mut rank) = (0, (usize::MAX, usize::MAX, ""), 0);
    for r in &rows {
        let d = (r.feature > 0)
            .then(|| decisions[r.feature - 1].as_ref())
            .flatten();
        if r.feature > 0 {
            if let Some(m) = min {
                if !d.and_then(|d| level_rank(&d.level)).is_some_and(|k| k <= m) {
                    continue;
                }
            }
        } else if min.is_some() {
            continue;
        }
        let key = (
            r.feature,
            r.method,
            if r.feature == 0 {
                r.source.as_str()
            } else {
                ""
            },
        );
        rank = if key == last { rank + 1 } else { 1 };
        last = key;
        if rank > p.top_per_method {
            continue;
        }
        if r.feature == 0 {
            unjoined += 1;
        }
        let cells = cells_of(r, d, rank, &table);
        let line: Vec<String> = cells.iter().map(|c| quote(c, sep)).collect();
        csv.push_str(&line.join(&sep.to_string()));
        csv.push('\n');
    }
    Ok(Made { csv, unjoined })
}

/// The CSV text for the page: `params_json` (the resolved parameters), the table, and the other files packed as `manifest` (a JSON list of `{"port","name","len"}`, ports `candidates`,
/// `screen`, `formulas`, `annot`) over the concatenated bytes `blob`, so that the names reach `source_file` as they do in `flow run`.
pub fn export_text_packed(
    params_json: &str,
    table: &[u8],
    manifest: &str,
    blob: &[u8],
) -> Result<String> {
    let params = Params::from_params(&crate::flow_ms2::json_params(params_json)?);
    let bad = || Error::new("err.flow.annotations.manifest");
    let list = json::parse(manifest.as_bytes())?;
    let (mut at, mut files) = (0usize, Vec::new());
    for f in list.arr() {
        let len = num(f, "len").ok_or_else(bad)? as usize;
        let bytes = blob.get(at..at + len).ok_or_else(bad)?;
        files.push((text(f, "port"), text(f, "name"), bytes));
        at += len;
    }
    let of = |port: &str| -> Vec<Named> {
        files
            .iter()
            .filter(|f| f.0 == port)
            .map(|f| (f.1.as_str(), f.2))
            .collect()
    };
    let (candidates, screen, formulas, annot) =
        (of("candidates"), of("screen"), of("formulas"), of("annot"));
    let inputs = Inputs {
        table,
        candidates: &candidates,
        screen: &screen,
        formulas: &formulas,
        annot: annot.first().copied(),
    };
    Ok(export_made(&params, &inputs)?.csv)
}

#[cfg(test)]
#[path = "flow_export_annot_tests.rs"]
mod tests;
