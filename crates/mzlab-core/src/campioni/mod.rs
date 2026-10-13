//! Roles and times of the files of a series, `campioni/2` (`docs/schemi/campioni-2.json`): the one model of the Data view and of mzFinder.
//! `proponi` reads the names (one parser, times never confirmed), `valida` gives stable codes (the pages put the words), `da_legacy` migrates the
//! notebook v1, the QA/QC roles, `campioni/1` and the files of a `.mzflow`, `per_tp` gives the indices by role and the minutes of `feature_pipeline/1`.

mod json;
mod nomi;
mod valida;

pub use json::{da_legacy, digest, from_json, to_json};
pub use nomi::{proponi, tipo_v1};
pub use valida::{per_tp, per_tp_json, valida, valida_json, Avviso, PerTp};

use crate::error::{Error, Result};

pub const SCHEMA: &str = "campioni/2";
/// Roles of a file, in the order of the schema.
pub const TIPI: [&str; 8] = [
    "sample",
    "t0",
    "blank",
    "dark",
    "infusion",
    "std",
    "calibration",
    "qc",
];
/// Kinds of blank (optional: every blank is subtracted the same way).
pub const TIPI_BIANCO: [&str; 2] = ["solvent", "procedure"];
/// Units of a time as written: seconds, minutes, hours, days.
pub const UNITA_TEMPO: [&str; 4] = ["s", "min", "h", "d"];

/// One file of the series.
#[derive(Debug, Clone, PartialEq)]
pub struct Campione {
    pub file: String,
    /// SHA-256 of the raw file: the key (the name can change).
    pub sha256: String,
    /// One of `TIPI`.
    pub kind: String,
    /// One of `TIPI_BIANCO`, only for a blank; None when not said.
    pub blank_kind: Option<String>,
    /// The time as written or confirmed, in `time_unit`.
    pub time: Option<f64>,
    /// One of `UNITA_TEMPO`.
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
}

impl Campione {
    /// An empty `sample` for `file` at position `order` (1-based).
    pub fn new(file: &str, sha256: &str, order: u32) -> Self {
        Campione {
            file: file.to_string(),
            sha256: sha256.to_string(),
            kind: "sample".into(),
            blank_kind: None,
            time: None,
            time_unit: "min".into(),
            time_min: None,
            time_confirmed: false,
            series: String::new(),
            replicate: 1,
            conc: None,
            unit: None,
            order,
            istd: false,
            note: String::new(),
            compound: String::new(),
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

/// The replicates counted again: 1-based among the files with the same series, role, minutes and concentration.
pub fn number_replicates(cs: &mut [Campione]) {
    for i in 0..cs.len() {
        let c = &cs[i];
        let same = cs[..i]
            .iter()
            .filter(|o| {
                o.series == c.series
                    && o.role() == c.role()
                    && o.time_min == c.time_min
                    && o.conc == c.conc
            })
            .count();
        cs[i].replicate = same as u32 + 1;
    }
}

/// The bindings: `op` is `proponi` (`{"files":[{"name","sha256","kind"}]}`), `valida` or `per_tp` (a `campioni/2`) or `da_legacy`
/// (`{"session","qaqc","campioni1","mzflow"}`, see `json::da_legacy`); JSON in, JSON out.
pub fn route(op: &str, input: &str) -> Result<String> {
    match op {
        "proponi" => {
            let v = crate::json::parse(input.as_bytes())?;
            let s = |f: &crate::json::Value, k: &str| {
                f.get(k).and_then(|x| x.str()).unwrap_or("").to_string()
            };
            let files: Vec<(String, String, String)> = v
                .get("files")
                .map(|a| a.arr())
                .unwrap_or_default()
                .iter()
                .map(|f| (s(f, "name"), s(f, "sha256"), s(f, "kind")))
                .collect();
            Ok(to_json(&proponi(&files)))
        }
        "valida" => Ok(valida_json(&valida(&from_json(input.as_bytes())?))),
        "per_tp" => Ok(per_tp_json(&per_tp(&from_json(input.as_bytes())?))),
        "da_legacy" => Ok(to_json(&da_legacy(input.as_bytes())?)),
        _ => Err(Error::new("error.badCampioni").with("op", op)),
    }
}
