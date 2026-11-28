//! The inputs of `quant.kinetics@1`: the parameters, the areas CSV (`quant.targets`, `chrom.transitions` or one written by hand) and the UV peaks CSV
//! (`chrom.uv_peaks`). A cell that is not a number is an error with its line; nothing is guessed.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

use super::curve_read::{cell, Csv};
use super::quant::param;

/// Defaults of the parameters; the descriptor of the step (`flow_kinetics.rs`) reads the same constants.
pub const DEFAULT_UV_RT_TOL: f64 = 0.2;
pub const DEFAULT_UV_TOL: f64 = 10.0;
pub const DEFAULT_BOOT: i64 = 200;

pub(super) struct Set {
    pub(super) ids: Vec<String>,
    pub(super) use_t0: bool,
    pub(super) zero_missing: bool,
    pub(super) normalise: bool,
    pub(super) uv_rt: f64,
    pub(super) uv_rt_tol: f64,
    pub(super) uv_tol: f64,
    pub(super) boot: usize,
    pub(super) seed: u32,
}

fn bad_param(name: &str, v: &Value) -> Error {
    Error::new("err.flow.kinetics.param")
        .with("name", name)
        .with("value", mzlab_core::jsonb::write(v))
}

fn number(params: &[(String, Value)], name: &str, default: f64, min: f64) -> Result<f64> {
    match param(params, name) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => v
            .f64()
            .filter(|x| x.is_finite() && *x >= min)
            .ok_or_else(|| bad_param(name, v)),
    }
}

fn choice(params: &[(String, Value)], name: &str, allowed: [&str; 2]) -> Result<bool> {
    match param(params, name) {
        None | Some(Value::Null) => Ok(true),
        Some(v) => match v.str() {
            Some(s) if s == allowed[0] => Ok(true),
            Some(s) if s == allowed[1] => Ok(false),
            _ => Err(bad_param(name, v)),
        },
    }
}

pub(super) fn settings(params: &[(String, Value)]) -> Result<Set> {
    let use_t0 = match param(params, "use_t0") {
        None | Some(Value::Null) => true,
        Some(v) => v.bool().ok_or_else(|| bad_param("use_t0", v))?,
    };
    let ids = param(params, "ids")
        .and_then(Value::str)
        .unwrap_or("")
        .split([';', ','])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    let boot = number(params, "boot", DEFAULT_BOOT as f64, 10.0)?;
    Ok(Set {
        ids,
        use_t0,
        zero_missing: !choice(params, "missing", ["skip", "zero"])?,
        normalise: choice(params, "uv_mode", ["normalise", "check"])?,
        uv_rt: number(params, "uv_rt", 0.0, 0.0)?,
        uv_rt_tol: number(params, "uv_rt_tol", DEFAULT_UV_RT_TOL, 0.0)?,
        uv_tol: number(params, "uv_tol", DEFAULT_UV_TOL, 0.0)?,
        boot: boot.round() as usize,
        seed: number(params, "seed", 0.0, 0.0)? as u32,
    })
}

/// One integrated peak: `area` is `None` when the row has no peak (empty cell or state `absent`).
pub(super) struct Area {
    pub(super) stem: String,
    pub(super) id: String,
    pub(super) area: Option<f64>,
    pub(super) state: String,
}

/// The name of a file without folders and without the extension of a raw or mzML file: how the areas and the samples name the same file.
pub(super) fn stem(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let low = base.to_lowercase();
    for e in [
        ".mzml", ".mzxml", ".cdf", ".raw", ".wiff2", ".wiff", ".d", ".mzidx", ".mgf",
    ] {
        if let Some(s) = low
            .ends_with(e)
            .then(|| base.get(..base.len().saturating_sub(e.len())))
            .flatten()
        {
            return s.to_string();
        }
    }
    base.to_string()
}

fn first(csv: &Csv, names: &[&str]) -> Option<usize> {
    names.iter().find_map(|n| csv.col(n))
}

fn row_err(file: &str, n: usize, what: &str) -> Error {
    Error::new("err.flow.kinetics.row")
        .with("file", file)
        .with("detail", format!("line {n}: {what}"))
}

/// The areas: columns `file` or `sample`, `id` (else `analyte`, `label`, `name`), `area`; `state` when there is one (`absent` is no peak).
pub(super) fn parse_areas(bytes: &[u8]) -> Result<Vec<Area>> {
    let bad = |d: String| Error::new("err.flow.kinetics.areas").with("detail", d);
    let csv = Csv::read(bytes, bad)?;
    let bad = |d: &str| Error::new("err.flow.kinetics.areas").with("detail", d);
    let file = first(&csv, &["file", "sample"]).ok_or_else(|| bad("no column file or sample"))?;
    let id = first(&csv, &["id", "analyte", "label", "name"])
        .ok_or_else(|| bad("no column id, analyte or name"))?;
    let area = csv.col("area").ok_or_else(|| bad("no column area"))?;
    let state = csv.col("state");
    let mut out: Vec<Area> = Vec::new();
    for (n, f) in &csv.rows {
        let (s, i) = (stem(cell(f, Some(file))), cell(f, Some(id)).to_string());
        if s.is_empty() || i.is_empty() {
            return Err(row_err("areas", *n, "no file or id"));
        }
        if out.iter().any(|r| r.stem == s && r.id == i) {
            return Err(row_err("areas", *n, &format!("{i} twice in {s}")));
        }
        let st = cell(f, state).to_string();
        let a = csv
            .num(cell(f, Some(area)))
            .map_err(|_| row_err("areas", *n, "area"))?;
        out.push(Area {
            area: a.filter(|_| st != "absent"),
            stem: s,
            id: i,
            state: st,
        });
    }
    if out.is_empty() {
        return Err(bad("no row"));
    }
    Ok(out)
}

/// One UV peak: apex RT (min) and area.
pub(super) struct Uv {
    pub(super) stem: String,
    pub(super) rt: f64,
    pub(super) area: f64,
}

/// The peaks of `chrom.uv_peaks`: `file`, `rt_apex_min`, `area_mau_s`.
pub(super) fn parse_uv(bytes: &[u8]) -> Result<Vec<Uv>> {
    let bad = |d: String| Error::new("err.flow.kinetics.uv").with("detail", d);
    let csv = Csv::read(bytes, bad)?;
    let need = |n: &str| {
        csv.col(n).ok_or_else(|| {
            Error::new("err.flow.kinetics.uv").with("detail", format!("no column {n}"))
        })
    };
    let (file, rt, area) = (need("file")?, need("rt_apex_min")?, need("area_mau_s")?);
    let mut out = Vec::new();
    for (n, f) in &csv.rows {
        let num = |c: usize, w: &str| {
            csv.num(cell(f, Some(c)))
                .ok()
                .flatten()
                .ok_or_else(|| row_err("uv", *n, w))
        };
        out.push(Uv {
            stem: stem(cell(f, Some(file))),
            rt: num(rt, "rt_apex_min")?,
            area: num(area, "area_mau_s")?,
        });
    }
    Ok(out)
}
