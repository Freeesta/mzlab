// Ported from Attributio reglists/screen.py.
//! The constants and row types of the B2 screening (suspects, features, evidence, hits, summary), re-exported by `screen`.

use std::collections::{BTreeMap, HashMap};

pub const PROTON: f64 = 1.007276;
pub const POS_ADDUCTS: [(&str, f64); 3] = [
    ("[M+H]+", PROTON),
    ("[M+NH4]+", 18.033823),
    ("[M+Na]+", 22.989218),
];
pub const NEG_ADDUCTS: [(&str, f64); 2] = [("[M-H]-", -PROTON), ("[M+Cl]-", 34.969402)];
pub const PPM: f64 = 5.0;
pub const RT_SD_K: f64 = 2.5;
pub const ISOLATED_RT_MIN: f64 = 0.15;
pub const YES: &str = "yes";
pub const NO: &str = "no";
pub const CLASSES: [&str; 5] = ["A", "B", "C", "D", "X"];

/// One suspect: the union row of a first block over the lists it belongs to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Suspect {
    pub skeleton: String,
    pub name: String,
    pub formula: String,
    pub mono_mass: f64,
    pub smiles: String,
    pub inchikey: String,
    pub lc_esi: String,
    pub xlogp: String,
    pub lists: String,
    pub list_labels: String,
    pub attributes: String,
    pub is_tp: String,
    pub parent: String,
    pub logp: Option<f64>,
    pub logp_source: String,
}

/// One feature of the screened sites brought to annotation.
#[derive(Clone, Debug, Default)]
pub struct FeatureRow {
    pub pol: String,
    pub id: i64,
    pub mz: f64,
    pub rt: f64,
    pub level: i64,
    pub xlogp: Option<f64>,
    /// SIRIUS molecular formula of the feature (empty without one).
    pub formula: String,
    /// Presence at each screened site, in `ScreenConfig::sites` order.
    pub in_site: Vec<bool>,
    /// Presence in each sorbent, in `ScreenConfig::sorbents` order.
    pub any_sorbent: Vec<bool>,
}

/// What is known about a feature besides its row: MS2 in a site/sorbent where it is detected and the «n/N» procedural blanks, and the identity it is already
/// annotated with (keys are `(polarity, id)`).
#[derive(Clone, Debug, Default)]
pub struct Evidence {
    pub ms2_blank: HashMap<(String, i64), (bool, String)>,
    pub annotated: HashMap<(String, i64), String>,
}

#[derive(Clone, Debug, Default)]
pub struct ScreenConfig {
    pub lists: Vec<String>,
    pub window: Option<(f64, f64)>,
    pub sites: Vec<String>,
    pub sorbents: Vec<String>,
    pub sorbent_labels: BTreeMap<String, String>,
}

/// One mass hit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hit {
    pub class: String,
    pub suspect: String,
    pub lists: String,
    pub list_labels: String,
    pub is_tp: String,
    pub parent: String,
    pub formula: String,
    pub polarity: String,
    pub feature_id: i64,
    pub adduct: String,
    pub mz_theor: f64,
    pub mz_obs: f64,
    pub ppm: f64,
    pub rt: f64,
    pub rt_pred: Option<f64>,
    pub rt_logp: String,
    pub logp: Option<f64>,
    pub logp_source: String,
    pub ms2: String,
    pub sirius_formula: String,
    pub formula_match: String,
    pub feature_level: i64,
    pub already_annotated_as: String,
    pub in_blanks: String,
    pub sites: String,
    pub sorbent: String,
    pub m2_theor: String,
    pub suspect_skeleton: String,
    pub suspect_inchikey: String,
    pub list_attributes: String,
    pub isolated_adduct: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SummaryRow {
    pub list_id: String,
    pub list: String,
    pub suspects_screened: usize,
    pub suspects_with_hits: usize,
    pub hits: usize,
    /// Hits and distinct suspects per class A, B, C, D, X.
    pub class_hits: [usize; 5],
    pub class_suspects: [usize; 5],
    pub suspects_with_ms2_hit: usize,
}

/// The RT ~ logP line: slope, intercept, robust SD and the number of features it was fitted on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RtModel {
    pub slope: f64,
    pub intercept: f64,
    pub sd: f64,
    pub n: usize,
}

#[derive(Clone, Debug)]
pub struct B2 {
    pub suspects: Vec<Suspect>,
    /// Suspects identified at B1 (same first block as a Level 1-3 identity) with that identity, not screened again.
    pub already: Vec<(Suspect, String)>,
    pub hits: Vec<Hit>,
    pub summary: Vec<SummaryRow>,
    pub rt_model: RtModel,
    pub lists: Vec<String>,
}
