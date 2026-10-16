//! What the low-resolution Dig reads from the page: every file of `api.run` (with the kind `classify` gave it), the parent and the settings.

use mzlab_core::elements::{fmt, ion_mz, mass};
use mzlab_core::json::{self, Out, Value};

use super::super::sample::guess;
use super::super::series::{self, FileIn};
use super::super::Res;

/// One file of the page: name, label, time, type (`sample`, `blank`, `control`, `standard`) and kind (`full`, `ms2`, `mrm`, `empty`).
#[derive(Debug, Clone, Default)]
pub struct FileLr {
    pub name: String,
    pub label: String,
    pub time: Option<f64>,
    pub kind_type: String,
    pub kind: String,
    /// The condition (`serie` of `campioni/2`); empty = shared.
    pub cond: String,
    pub replica: Option<u32>,
    /// Position of the injection (`ordine_iniezione`) when `campioni/2` gave it, 0 when unknown.
    pub order: u32,
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
    pub min_files: usize,
    /// «Fragment ions found» lists the fragments of the representative MS2 scan from this % of the strongest (`ms2frag`).
    pub frag_min_rel: f64,
    /// A detected cell is a peak only with this S/N on its XIC (`peakchk`).
    pub min_snr: f64,
    /// ... at most this FWHM, seconds.
    pub max_fwhm_s: f64,
    /// ... at least this many scans above 5 % of its height.
    pub min_points: usize,
    /// ... and its apex after `dead_factor · rt_min` (`rt_min` = end of the dead volume).
    pub dead_factor: f64,
    /// Products are proposed only when the parent is a chromatographic peak whose area falls over time (`parent_check`).
    pub parent_decay: bool,
    /// Options of the comparison of conditions (`gruppi/1`) from the settings key `gruppi`; the defaults of the module when absent.
    pub groups: crate::gruppi::GroupOpts,
}

impl Settings {
    pub fn persist(&self) -> super::labels::Persist {
        super::labels::Persist {
            min_files: self.min_files,
            rt_tol: self.rt_tol_min,
        }
    }
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
            min_files: 3,
            frag_min_rel: super::ms2frag::MIN_REL,
            min_snr: 5.0,
            max_fwhm_s: 60.0,
            min_points: 5,
            dead_factor: 1.5,
            parent_decay: true,
            groups: crate::gruppi::GroupOpts::default(),
        }
    }
}

/// The parent: neutral formula (empty when only its m/z is known), neutral mass and [M+H]+.
#[derive(Debug, Clone)]
pub struct ParentLr {
    pub formula: String,
    pub neutral_mass: f64,
    pub mz: f64,
    /// Formula of the [M+H]+ ion (`None` for another adduct or without a formula).
    pub ion_formula: Option<String>,
    /// The SMILES of the parent, for the fragmentation tree.
    pub smiles: Option<String>,
}

/// The files of the series that are columns of the table (full scan samples, blanks and controls), in the order of the table.
/// The standard of the parent is a column only when the series compares conditions (it tells whether an ion is already in the parent).
pub fn columns(files: &[FileLr]) -> Vec<FileIn> {
    let multi = super::cond::count(
        files
            .iter()
            .map(|f| (f.kind_type.as_str(), f.cond.as_str())),
    ) > 1;
    files
        .iter()
        .filter(|f| {
            f.kind == "full"
                && (matches!(f.kind_type.as_str(), "sample" | "blank" | "control")
                    || (multi && f.kind_type == "standard"))
        })
        .map(|f| {
            let mut x = FileIn {
                name: f.name.clone(),
                label: f.label.clone(),
                kind: f.kind_type.clone(),
                time: f.time,
                t: f64::NAN,
                cond: f.cond.clone(),
                replica: f.replica,
                order: f.order,
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

/// The files as the page passes them to `api.run` (`kind` from `classify`; a file without one counts as full scan): a list, or
/// `{"files": [...], "campioni": <campioni/2>}` when the page keeps the roles, times, conditions and replicates in the canonical model
/// (they then win over the fields `type`, `time`, `cond` and `replica` of each file).
pub fn files(files_json: &str) -> Res<Vec<FileLr>> {
    let v = json::parse(files_json.as_bytes()).map_err(|e| e.to_string())?;
    let (list, camp) = match v.get("files") {
        Some(l) => (l, v.get("campioni")),
        None => (&v, None),
    };
    let camp = match camp {
        Some(c) if !c.is_null() => {
            crate::gruppi::campioni_from_value(c).map_err(|e| e.to_string())?
        }
        _ => Vec::new(),
    };
    let text = |f: &Value, k: &str| f.get(k).and_then(Value::str).map(String::from);
    Ok(list
        .arr()
        .iter()
        .map(|f| {
            let name = text(f, "name").unwrap_or_default();
            let g = guess(&name);
            let mut x = FileLr {
                label: text(f, "label")
                    .filter(|s| !s.is_empty())
                    .unwrap_or(g.label),
                time: match f.get("time") {
                    Some(t) => t.f64(),
                    None => g.time,
                },
                kind_type: text(f, "type").unwrap_or_else(|| g.kind.to_string()),
                kind: text(f, "kind").unwrap_or_else(|| "full".into()),
                cond: text(f, "cond").unwrap_or_default(),
                replica: f.get("replica").and_then(Value::f64).map(|r| r as u32),
                order: 0,
                name,
            };
            if let Some(c) = camp.iter().find(|c| c.file == x.name) {
                apply_campione(&mut x, c);
            }
            x
        })
        .collect())
}

/// The role, time, condition and replicate of `campioni/2` over a file (a role the Dig does not use keeps the page's type).
fn apply_campione(x: &mut FileLr, c: &mzlab_core::campioni::Campione) {
    match c.role() {
        "sample" | "t0" => x.kind_type = "sample".into(),
        "blank" => x.kind_type = "blank".into(),
        "dark" => x.kind_type = "control".into(),
        "std" => x.kind_type = "standard".into(),
        _ => {}
    }
    x.time = c.time_min;
    x.cond = c.series.clone();
    x.replica = Some(c.replicate);
    x.order = c.order;
}

/// The parent of `api.run`: formula or SMILES ([M+H]+), or only the m/z of its ion.
pub fn parent(parent_json: &str) -> Res<ParentLr> {
    if let Ok(p) = series::parent(parent_json) {
        return Ok(ParentLr {
            formula: p.formula(),
            neutral_mass: mass(&p.neutral),
            mz: p.mz,
            ion_formula: (p.adduct == "[M+H]+").then(|| fmt(&p.ion)),
            smiles: p.smiles,
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
        ion_formula: None,
        smiles: None,
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
    s.min_files = num("min_files").map_or(s.min_files, |x| x.max(0.0) as usize);
    s.frag_min_rel = num("frag_min_rel").map_or(s.frag_min_rel, |x| x.clamp(0.0, 100.0));
    s.min_snr = num("min_snr").map_or(s.min_snr, |x| x.max(0.0));
    s.max_fwhm_s = num("max_fwhm_s")
        .filter(|x| *x > 0.0)
        .unwrap_or(s.max_fwhm_s);
    s.min_points = num("min_points").map_or(s.min_points, |x| x.max(1.0) as usize);
    s.dead_factor = num("dead_factor").map_or(s.dead_factor, |x| x.max(0.0));
    s.parent_decay = flag("parent_decay").unwrap_or(s.parent_decay);
    if let Some(o) = v
        .get("gruppi")
        .and_then(|g| crate::gruppi::opts_from_value(g).ok())
    {
        s.groups = o;
    }
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
    o.raw(",\"min_files\":").int(s.min_files as i64);
    o.raw(",\"auto_offset\":")
        .raw(if s.auto_offset { "true" } else { "false" });
    o.raw(",\"min_snr\":")
        .num(s.min_snr)
        .raw(",\"max_fwhm_s\":")
        .num(s.max_fwhm_s);
    o.raw(",\"min_points\":")
        .int(s.min_points as i64)
        .raw(",\"dead_factor\":")
        .num(s.dead_factor);
    o.raw(",\"parent_decay\":")
        .raw(if s.parent_decay { "true" } else { "false" });
    o.raw("}");
}

/// What the worker needs before the pipeline: `{lc: [names of the columns], ms2: [names], mrm: [names], minutes: [time | null], formula}`; `rust: false` when this series
/// is not one the Rust Dig serves yet (a file that is not full scan, or a parent known only by its m/z).
pub fn plan_json(files_json: &str, parent_json: &str) -> Res<String> {
    let fs = files(files_json)?;
    let p = parent(parent_json)?;
    let lc = columns(&fs);
    let ms2: Vec<&FileLr> = fs.iter().filter(|f| f.kind == "ms2").collect();
    let mrm: Vec<&FileLr> = fs.iter().filter(|f| f.kind == "mrm").collect();
    let rust = !lc.is_empty()
        && !p.formula.is_empty()
        && fs
            .iter()
            .all(|f| matches!(f.kind.as_str(), "full" | "ms2" | "mrm"));
    let mut o = Out::default();
    o.raw("{\"rust\":")
        .raw(if rust { "true" } else { "false" })
        .raw(",\"lc\":[");
    for (i, f) in lc.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(&f.name);
    }
    o.raw("],\"ms2\":[");
    for (i, f) in ms2.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(&f.name);
    }
    o.raw("],\"mrm\":[");
    for (i, f) in mrm.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(&f.name);
    }
    o.raw("],\"minutes\":[");
    for (i, f) in lc.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).opt(f.time);
    }
    o.raw("],\"formula\":").str(&p.formula).raw("}");
    Ok(o.0)
}
