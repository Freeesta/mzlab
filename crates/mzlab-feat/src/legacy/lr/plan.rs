//! What the low-resolution Dig reads from the page: every file of `api.run` (with the kind `classify` gave it), the parent and the settings.

use mzlab_core::elements::{ion_mz, mass};
use mzlab_core::json::{self, Out, Value};

use super::super::sample::guess;
use super::super::series::{self, FileIn};
use super::super::Res;

/// One file of the page: name, label, time, type (`sample`, `blank`, `control`, `standard`) and kind (`full`, `ms2`, `mrm`, `empty`).
#[derive(Debug, Clone)]
pub struct FileLr {
    pub name: String,
    pub label: String,
    pub time: Option<f64>,
    pub kind_type: String,
    pub kind: String,
}

/// `DEFAULT_SETTINGS` of the Python engine, with what the page changed.
#[derive(Debug, Clone)]
pub struct Settings {
    pub tol_da: f64,
    pub rt_tol_min: f64,
    pub max_steps: f64,
    pub rt_min: f64,
    pub discover: bool,
    pub max_unexpected: usize,
    pub auto_offset: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            tol_da: 0.35,
            rt_tol_min: 0.25,
            max_steps: 2.0,
            rt_min: 0.5,
            discover: true,
            max_unexpected: 40,
            auto_offset: true,
        }
    }
}

/// The parent: neutral formula (empty when only its m/z is known), neutral mass and [M+H]+.
#[derive(Debug, Clone)]
pub struct ParentLr {
    pub formula: String,
    pub neutral_mass: f64,
    pub mz: f64,
}

/// The files of the series that are columns of the table (full scan samples, blanks and controls), in the order of the table.
pub fn columns(files: &[FileLr]) -> Vec<FileIn> {
    files
        .iter()
        .filter(|f| {
            f.kind == "full" && matches!(f.kind_type.as_str(), "sample" | "blank" | "control")
        })
        .map(|f| {
            let mut x = FileIn {
                name: f.name.clone(),
                label: f.label.clone(),
                kind: f.kind_type.clone(),
                time: f.time,
                t: f64::NAN,
            };
            x.t = match f.time {
                Some(t) => t,
                None if x.is_ref() => -1.0,
                None => f64::NAN,
            };
            x
        })
        .collect()
}

/// The files as the page passes them to `api.run` (`kind` from `classify`; a file without one counts as full scan).
pub fn files(files_json: &str) -> Res<Vec<FileLr>> {
    let v = json::parse(files_json.as_bytes()).map_err(|e| e.to_string())?;
    let text = |f: &Value, k: &str| f.get(k).and_then(Value::str).map(String::from);
    Ok(v.arr()
        .iter()
        .map(|f| {
            let name = text(f, "name").unwrap_or_default();
            let g = guess(&name);
            FileLr {
                label: text(f, "label")
                    .filter(|s| !s.is_empty())
                    .unwrap_or(g.label),
                time: match f.get("time") {
                    Some(t) => t.f64(),
                    None => g.time,
                },
                kind_type: text(f, "type").unwrap_or_else(|| g.kind.to_string()),
                kind: text(f, "kind").unwrap_or_else(|| "full".into()),
                name,
            }
        })
        .collect())
}

/// The parent of `api.run`: formula or SMILES ([M+H]+), or only the m/z of its ion.
pub fn parent(parent_json: &str) -> Res<ParentLr> {
    if let Ok(p) = series::parent(parent_json) {
        return Ok(ParentLr {
            formula: p.formula(),
            neutral_mass: mass(&p.neutral),
            mz: p.mz,
        });
    }
    let v = json::parse(parent_json.as_bytes()).map_err(|e| e.to_string())?;
    let mz = v
        .get("mz")
        .and_then(|x| {
            x.f64()
                .or_else(|| x.str().and_then(|s| s.trim().parse().ok()))
        })
        .filter(|m| *m > 0.0)
        .ok_or(
            "the neutral molecular formula, the SMILES or the m/z of the parent ion is required",
        )?;
    let shift = ion_mz(0.0, "[M+H]+").unwrap_or(1.007276);
    Ok(ParentLr {
        formula: String::new(),
        neutral_mass: mz - shift,
        mz,
    })
}

/// The settings of the page over the defaults.
pub fn settings(settings_json: &str) -> Settings {
    let mut s = Settings::default();
    let Ok(v) = json::parse(settings_json.as_bytes()) else {
        return s;
    };
    let num = |k: &str| v.get(k).and_then(Value::f64);
    let flag = |k: &str| v.get(k).and_then(Value::bool);
    s.tol_da = num("tol_da").filter(|x| *x > 0.0).unwrap_or(s.tol_da);
    s.rt_tol_min = num("rt_tol_min")
        .filter(|x| *x > 0.0)
        .unwrap_or(s.rt_tol_min);
    s.max_steps = num("max_steps").unwrap_or(s.max_steps);
    s.rt_min = num("rt_min").unwrap_or(s.rt_min);
    s.discover = flag("discover").unwrap_or(s.discover);
    s.max_unexpected = num("max_unexpected").map_or(s.max_unexpected, |x| x.max(0.0) as usize);
    s.auto_offset = flag("auto_offset").unwrap_or(s.auto_offset);
    s
}

pub fn settings_json(o: &mut Out, s: &Settings) {
    o.raw("{\"tol_da\":")
        .num(s.tol_da)
        .raw(",\"rt_tol_min\":")
        .num(s.rt_tol_min);
    o.raw(",\"max_steps\":")
        .num(s.max_steps)
        .raw(",\"rt_min\":")
        .num(s.rt_min);
    o.raw(",\"discover\":")
        .raw(if s.discover { "true" } else { "false" });
    o.raw(",\"max_unexpected\":").int(s.max_unexpected as i64);
    o.raw(",\"auto_offset\":")
        .raw(if s.auto_offset { "true" } else { "false" });
    o.raw("}");
}

/// What the worker needs before the pipeline: `{lc: [names of the columns], minutes: [time | null], formula}`; `rust: false` when this series
/// is not one the Rust Dig serves yet (a file that is not full scan, or a parent known only by its m/z).
pub fn plan_json(files_json: &str, parent_json: &str) -> Res<String> {
    let fs = files(files_json)?;
    let p = parent(parent_json)?;
    let lc = columns(&fs);
    let rust = !lc.is_empty() && !p.formula.is_empty() && fs.iter().all(|f| f.kind == "full");
    let mut o = Out::default();
    o.raw("{\"rust\":")
        .raw(if rust { "true" } else { "false" })
        .raw(",\"lc\":[");
    for (i, f) in lc.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(&f.name);
    }
    o.raw("],\"minutes\":[");
    for (i, f) in lc.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).opt(f.time);
    }
    o.raw("],\"formula\":").str(&p.formula).raw("}");
    Ok(o.0)
}
