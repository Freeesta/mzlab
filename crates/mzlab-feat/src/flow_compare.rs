//! The texts of `table.compare@1` (`compare/1`) and of its plan (`compare-plan/1`, `table compare --dry-run`): a feature table (a `.mzfeat`, or the mzmine
//! "full" CSV `features` writes) against its reference and blank columns (`mzlab_core::statcmp`). Bytes in, text out: `flow run`, `mzlab-cli table compare` and
//! the browser module `flowstat` all call these, so they write the same bytes. A parameter that is not given takes the engine's default and says so in `choices`.

use crate::import::strip_mzml;
#[cfg(not(target_arch = "wasm32"))]
use crate::import::read_full;
use crate::samples::samples_of;
use crate::store::{origin_from_meta, read_table};
use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;
use mzlab_core::jsonb::write;
use mzlab_core::statcmp::{compare, plan, CmpCol, CmpIn, CmpOpts};
use mzlab_core::statistics::StatErr;

/// A table as the engine reads it: one row per feature, one column per file.
struct Loaded {
    ids: Vec<String>,
    mz: Vec<f64>,
    rt: Vec<f64>,
    matrix: Vec<Vec<f64>>,
    /// The names the selectors see (no `.mzML`), the names the file gave and their SHA-256 (empty for a CSV).
    names: Vec<String>,
    given: Vec<String>,
    shas: Vec<String>,
}

fn param<'a>(params: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    params.iter().find(|(k, _)| k == name).map(|(_, v)| v).filter(|v| !v.is_null())
}

fn number(params: &[(String, Value)], name: &str) -> Option<f64> {
    param(params, name).and_then(Value::f64)
}

fn selector(params: &[(String, Value)], name: &'static str) -> Result<String> {
    match param(params, name).and_then(Value::str).map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Ok(s.to_string()),
        None => Err(Error::new("err.compare.param").with("detail", format!("--{name} is needed"))),
    }
}

/// A table or a samples file that cannot be read: input (exit 4), with what the reader said.
fn unreadable(detail: impl ToString) -> Error {
    Error::new("err.compare.input").with("detail", detail)
}

fn stat_error(e: StatErr) -> Error {
    Error::new(e.key.unwrap_or("error.badStatInput")).with("detail", e.text)
}

fn quantity(v: f64) -> f64 {
    if v.is_finite() && v > 0.0 { v } else { 0.0 }
}

fn load(table: &[u8], name: &str, quantity_name: &str) -> Result<Loaded> {
    if table.starts_with(b"MZFEAT") {
        let (t, meta) = read_table(table).map_err(|e| unreadable(e.key))?;
        let inputs = origin_from_meta(&meta).inputs;
        if inputs.len() != t.n_files || t.n_files == 0 {
            return Err(unreadable("the table does not name one file per column"));
        }
        let cell = |r: usize, f: usize| {
            let at = r * t.n_files + f;
            quantity(if quantity_name == "height" { f64::from(t.height[at]) } else { t.area[at] })
        };
        return Ok(Loaded {
            ids: (1..=t.len()).map(|i| i.to_string()).collect(),
            mz: t.mz.clone(),
            rt: t.rt.clone(),
            matrix: (0..t.len()).map(|r| (0..t.n_files).map(|f| cell(r, f)).collect()).collect(),
            names: inputs.iter().map(|(n, _)| strip_mzml(n)).collect(),
            given: inputs.iter().map(|(n, _)| n.clone()).collect(),
            shas: inputs.into_iter().map(|(_, s)| s).collect(),
        });
    }
    if quantity_name == "height" {
        return Err(Error::new("err.compare.quantity"));
    }
    csv(table, name)
}

/// The mzmine "full" CSV of the command line. The page holds `.mzfeat` tables only, so the `.wasm` leaves this reader out (it is ~130 KB of the module).
#[cfg(target_arch = "wasm32")]
fn csv(_table: &[u8], _name: &str) -> Result<Loaded> {
    Err(unreadable("the browser reads a .mzfeat table, not a CSV"))
}

#[cfg(not(target_arch = "wasm32"))]
fn csv(table: &[u8], name: &str) -> Result<Loaded> {
    let text = std::str::from_utf8(table).map_err(|_| unreadable("the table is neither a .mzfeat nor a text CSV"))?;
    let t = read_full(text, name, 0, 0).map_err(|e| unreadable(e.msg))?;
    if t.samples.is_empty() {
        return Err(unreadable("no datafile:<name>:area column"));
    }
    let col = |s: &str| t.samples.iter().position(|x| x == s);
    let matrix = t.features.iter().map(|f| {
        let mut row = vec![0.0; t.samples.len()];
        f.areas.iter().filter_map(|(s, a)| col(s).map(|i| (i, a))).for_each(|(i, a)| row[i] = quantity(*a));
        row
    });
    Ok(Loaded {
        ids: t.features.iter().map(|f| f.id.clone()).collect(),
        mz: t.features.iter().map(|f| f.mz).collect(),
        rt: t.features.iter().map(|f| f.rt).collect(),
        matrix: matrix.collect(),
        names: t.samples.clone(),
        given: t.samples.clone(),
        shas: vec![String::new(); t.samples.len()],
    })
}

/// The role and time of each column from a `samples/1` document, by name (with or without `.mzML`), else by SHA-256.
fn meta_of(l: &Loaded, samples: &[u8]) -> Result<Vec<Option<CmpCol>>> {
    let all = samples_of(samples).map_err(|e| unreadable(e.key))?;
    Ok((0..l.names.len())
        .map(|i| {
            let by_sha = |s: &&crate::samples::Sample| !l.shas[i].is_empty() && s.sha256 == l.shas[i];
            let found = all.iter().find(|s| s.file == l.given[i] || strip_mzml(&s.file) == l.names[i]).or_else(|| all.iter().find(by_sha));
            found.map(|s| CmpCol { name: l.names[i].clone(), role: Some(s.kind.clone()), time: s.time_min })
        })
        .collect())
}

fn run(
    params: &[(String, Value)],
    name: &str,
    table: &[u8],
    samples: Option<&[u8]>,
    go: impl FnOnce(&CmpIn, &CmpOpts) -> std::result::Result<Value, StatErr>,
) -> Result<String> {
    let q = param(params, "quantity").and_then(Value::str).unwrap_or("area").to_string();
    let opts = CmpOpts {
        ref_sel: selector(params, "ref")?,
        blank_sel: selector(params, "blank")?,
        quantity: param(params, "quantity").and_then(Value::str).map(String::from),
        factor: number(params, "factor"),
        pseudo: number(params, "pseudocount"),
        floor: number(params, "floor"),
        rho_min: number(params, "rho_min"),
    };
    let l = load(table, name, &q)?;
    let meta = samples.map(|s| meta_of(&l, s)).transpose()?;
    let sha = sha256_hex(table);
    let inp = CmpIn { path: name, sha256: &sha, ids: &l.ids, mz: &l.mz, rt: &l.rt, matrix: &l.matrix, names: &l.names, meta: meta.as_deref() };
    go(&inp, &opts).map(|v| write(&v)).map_err(stat_error)
}

/// The `compare/1` text of `table` (named `name`, a file name) under `params`; `samples` is an optional `samples/1` document.
pub fn compare_text(params: &[(String, Value)], name: &str, table: &[u8], samples: Option<&[u8]>) -> Result<String> {
    run(params, name, table, samples, compare)
}

/// The `compare-plan/1` text: the columns of every role and the parameters, with nothing computed.
pub fn plan_text(params: &[(String, Value)], name: &str, table: &[u8], samples: Option<&[u8]>) -> Result<String> {
    run(params, name, table, samples, plan)
}
