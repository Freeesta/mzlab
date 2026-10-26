//! Roles and times of the files of a series, `campioni/2` (`docs/schemi/campioni-2.json`): the one model of the Data view and of mzFinder.
//! `proponi` reads the names (one parser, times never confirmed), `valida` gives stable codes (the pages put the words), `da_legacy` migrates the
//! notebook v1, the QA/QC roles, `campioni/1` and the files of a `.mzflow`, `per_tp` gives the indices by role and the minutes of `feature_pipeline/1`.
//! `campioni/3` (`docs/schemi/campioni-3.json`) adds the physical sample, polarity, matrix, batch, date, group and free typed columns: `colonne`, `json3`
//! (canonical form, migration of a `/2`), `valida3`, `csv`, `modello` (name models, `regex`) and `dal_file` (what the file says about itself).

mod colonne;
mod csv;
mod dal_file;
mod json;
mod json3;
mod modello;
mod nomi;
mod posizioni;
mod regex;
mod ricollega;
mod valida;
mod valida3;

pub use colonne::{Cell, Colonna, DatiFile, Tabella, TipoColonna};
pub use csv::{csv_leggi, csv_scrivi};
pub use dal_file::{dal_file, riassunto_da_scansioni, RiassuntoFile};
pub use json::{da_legacy, digest, from_json, from_value, to_json};
pub use json3::{digest3, tabella_da_json, tabella_da_value, to_json3, SCHEMA3};
pub use modello::{applica_modello, Applicato};
pub use nomi::{proponi, proponi_spiegato, ruolo_e_minuti, tipo_v1};
pub use posizioni::{proposte_json, Posizione, Proposta};
pub use ricollega::{ricollega, ricollega_json, Abbinamento};
pub use valida::{per_tp, per_tp_json, valida, valida_json, Avviso, PerTp};
pub use valida3::valida3;

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
#[derive(Debug, Clone, PartialEq, Default)]
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
    // `campioni/3` (docs/agenti/campioni.md): sample, +1/-1 polarity, matrix, batch (disegno/1), ISO date and time, group, free columns (sorted by name), what the file says
    pub sample_id: String,
    pub polarity: Option<i8>,
    pub matrix: String,
    pub batch: String,
    pub acquired: Option<String>,
    pub group: String,
    pub extra: Vec<(String, Cell)>,
    pub file_data: Option<DatiFile>,
}

impl Campione {
    /// An empty `sample` for `file` at position `order` (1-based).
    pub fn new(file: &str, sha256: &str, order: u32) -> Self {
        Campione {
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

/// The replicates counted again: 1-based among the files with the same series, role, minutes and concentration.
pub fn number_replicates(cs: &mut [Campione]) {
    // NaN never equals itself, so those files keep the pairwise comparison
    if cs
        .iter()
        .any(|c| c.time_min.is_some_and(f64::is_nan) || c.conc.is_some_and(f64::is_nan))
    {
        return number_replicates_pairwise(cs);
    }
    // 0.0 and -0.0 are equal, hence the same key
    let bits = |x: Option<f64>| x.map(|v| if v == 0.0 { 0 } else { v.to_bits() });
    let mut seen: std::collections::HashMap<(String, String, Option<u64>, Option<u64>), u32> =
        std::collections::HashMap::with_capacity(cs.len());
    for c in cs.iter_mut() {
        let key = (
            c.series.clone(),
            c.role().to_string(),
            bits(c.time_min),
            bits(c.conc),
        );
        let n = seen.entry(key).or_insert(0);
        *n += 1;
        c.replicate = *n;
    }
}

fn number_replicates_pairwise(cs: &mut [Campione]) {
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

/// The bindings (a `campioni/2` or `/3` document is read by every operation that takes one; `canonico` and `digest` keep its schema, `migra` makes a `/3`;
/// `valida` also gives the findings of `/3`; `route_tutto` adds `modello`, `dal_file`, `csv_leggi` and `csv_scrivi`): `op` is `proponi` or `proponi_spiegato` (`{"files":[{"name","sha256","kind"}]}`; the second adds the spans of the name), `valida`, `per_tp`, `canonico` or `digest` (a `campioni/2`), `ricollega` (`{"campioni","file":[{"percorso","sha256"}]}`) or `da_legacy`
/// (`{"session","qaqc","campioni1","mzflow"}`, see `json::da_legacy`); JSON in, JSON out.
pub fn route(op: &str, input: &str) -> Result<String> {
    match op {
        "proponi" | "proponi_spiegato" => {
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
            if op == "proponi_spiegato" {
                return Ok(proposte_json(&proponi_spiegato(&files)));
            }
            Ok(to_json(&proponi(&files)))
        }
        "valida" => Ok(valida_json(&valida3(&tabella_da_json(input.as_bytes())?))),
        "per_tp" => Ok(per_tp_json(&per_tp(&from_json(input.as_bytes())?))),
        "da_legacy" => Ok(to_json(&da_legacy(input.as_bytes())?)),
        "ricollega" => {
            let v = crate::json::parse(input.as_bytes())?;
            let bad = || Error::new("error.badCampioni");
            let cs = from_value(v.get("campioni").ok_or_else(bad)?)?;
            let cand: Vec<(String, String)> = v
                .get("file")
                .map(|a| a.arr())
                .unwrap_or_default()
                .iter()
                .map(|f| {
                    let t = |k: &str| f.get(k).and_then(|x| x.str()).unwrap_or("").to_string();
                    (t("percorso"), t("sha256"))
                })
                .collect();
            Ok(ricollega_json(&ricollega(&cs, &cand)))
        }
        "canonico" | "digest" | "migra" => {
            let doc = crate::json::parse(input.as_bytes())?;
            let t = tabella_da_value(&doc)?;
            let v3 = op == "migra" || doc.get("schema").and_then(|v| v.str()) == Some(SCHEMA3);
            Ok(match (op, v3) {
                ("digest", true) => digest3(&t),
                ("digest", false) => digest(&t.campioni),
                (_, true) => to_json3(&t),
                _ => to_json(&t.campioni),
            })
        }
        _ => Err(Error::new("error.badCampioni").with("op", op)),
    }
}

/// The operations that read the names and the files (`modello`, `dal_file`, `csv_leggi`, `csv_scrivi`) and everything `route` does: the engine of the Data view
/// serves this one. `route` alone is what mzFinder links (its .wasm has a cap: the name models and the CSV are not its business).
pub fn route_tutto(op: &str, input: &str) -> Result<String> {
    match op {
        "dal_file" => dal_file(input),
        "modello" => applica_modello(input),
        "csv_leggi" => csv_leggi(input),
        "csv_scrivi" => csv_scrivi(input),
        _ => route(op, input),
    }
}
