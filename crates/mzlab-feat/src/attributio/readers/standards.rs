// Ported from Attributio readers/standards.py, annotate.py, campaign.py.
//! Level 1: the match of features against an in-house library of reference standards (structure confirmed with a standard measured with the same method).
//! Three checks, all required: accurate mass (within `mass_tol_ppm` of one of the standard's ions), retention time (within `rt_tol_min` of the RT the standard
//! showed on the campaign's own instrument) and MS/MS (the feature spectrum matches the standard's spectrum of the SAME ion). What happens to each mass + RT
//! candidate is recorded in `status`: `confirmed`, `confirmed_manual` (Level 1); `review`, `no_ms2`, `no_reference_ion` (evidence below Level 1);
//! `mismatch`, `unobserved_ion`, `rejected_manual` (the name is NOT given to the feature). The spectral similarity is supplied by the caller.

use super::ms2::{RefLibrary, Spectrum};
use super::{num_at, read_table, text_at};
use crate::attributio::campaign::{expand, Campaign};
use crate::attributio::error::{py_list, CampaignError, Result};
use crate::attributio::frame::{to_num, Cell, Frame, Record};
use crate::attributio::io::Io;
use crate::attributio::toml::{Table, Value};
use std::collections::{BTreeSet, HashMap};

pub const DEFAULT_POS_ADDUCTS: [&str; 4] =
    ["mz_MH_pos", "mz_MNa_pos", "mz_MNH4_pos", "mz_M2H_pos2"];
pub const DEFAULT_NEG_ADDUCTS: [&str; 2] = ["mz_MH_neg", "mz_MHCOO_neg"];
pub const INTERNAL_STANDARD_COMMENT: &str = "Standard interno";
pub const PROTONATED: [&str; 2] = ["mz_MH_pos", "mz_MH_neg"];
pub const LEVEL1_STATUSES: [&str; 2] = ["confirmed", "confirmed_manual"];
pub const SUPPORTIVE_STATUSES: [&str; 3] = ["review", "no_ms2", "no_reference_ion"];
pub const REJECTED_STATUSES: [&str; 3] = ["mismatch", "unobserved_ion", "rejected_manual"];
/// A decision is found again by (standard, m/z, RT) when mzMine is re-run and the feature ids change: tolerances of that fallback.
pub const DECISION_MZ_PPM: f64 = 5.0;
pub const DECISION_RT_MIN: f64 = 0.10;

fn rank(status: &str) -> usize {
    LEVEL1_STATUSES
        .iter()
        .chain(&SUPPORTIVE_STATUSES)
        .chain(&REJECTED_STATUSES)
        .position(|s| *s == status)
        .unwrap_or(usize::MAX)
}

/// `[standards]` of the campaign with its defaults; `None` when the campaign has no reference-standard library.
#[derive(Clone, Debug, PartialEq)]
pub struct StandardsSettings {
    pub mass_tol_ppm: f64,
    pub rt_tol_min: f64,
    pub rt_column: [String; 2],
    pub adducts: [Vec<String>; 2],
    pub ms2_library: [Option<String>; 2],
    pub ms2_min_cosine: f64,
    pub ms2_min_matched: i64,
    pub ms2_tol_da: f64,
    pub precursor_tol_ppm: f64,
    pub decisions: Option<String>,
}

fn number(t: &Table, key: &str, default: f64) -> f64 {
    t.get(key).and_then(Value::as_f64).unwrap_or(default)
}

fn adduct_list(t: &Table, key: &str, default: &[&str]) -> Vec<String> {
    match t.get(key) {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => default.iter().map(|s| s.to_string()).collect(),
    }
}

impl StandardsSettings {
    pub fn index(pol: &str) -> usize {
        usize::from(pol != "POS")
    }
    pub fn from_campaign(c: &Campaign) -> Option<StandardsSettings> {
        let st = &c.standards;
        if st.0.is_empty() || c.library_csv.is_none() {
            return None;
        }
        let text = |k: &str| {
            st.get(k)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(|s| expand(s, &c.env))
        };
        let rt = st.get("rt_column").and_then(Value::as_str).unwrap_or("rt");
        let rt_pol = |k: &str| st.get(k).and_then(Value::as_str).unwrap_or(rt).to_string();
        Some(StandardsSettings {
            mass_tol_ppm: number(st, "mass_tol_ppm", 5.0),
            rt_tol_min: number(st, "rt_tol_min", 0.35),
            rt_column: [rt_pol("rt_column_pos"), rt_pol("rt_column_neg")],
            adducts: [
                adduct_list(st, "adducts_pos", &DEFAULT_POS_ADDUCTS),
                adduct_list(st, "adducts_neg", &DEFAULT_NEG_ADDUCTS),
            ],
            ms2_library: [text("ms2_library_pos"), text("ms2_library_neg")],
            ms2_min_cosine: number(st, "ms2_min_cosine", 0.70),
            ms2_min_matched: number(st, "ms2_min_matched", 2.0) as i64,
            ms2_tol_da: number(st, "ms2_tol_da", 0.01),
            precursor_tol_ppm: number(st, "precursor_tol_ppm", 10.0),
            decisions: text("decisions"),
        })
    }
}

/// The reference-standard library (semicolon-separated, one row per standard); an empty frame, with a warning, when the file is not there.
pub fn load_standards_library(c: &Campaign, io: &Io, warnings: &mut Vec<String>) -> Result<Frame> {
    let Some(path) = &c.library_csv else {
        return Ok(Frame::default());
    };
    match read_table(io, path, ';')? {
        Some(f) => Ok(f),
        None => {
            warnings.push(format!(
                "[{}] standards library not found: {path} -> no Level 1 in this run.",
                c.id
            ));
            Ok(Frame::default())
        }
    }
}

/// One decision of the chemist on a mass + RT candidate (`confirm` or `reject`).
#[derive(Clone, Debug, PartialEq)]
pub struct Level1Decision {
    pub pol: String,
    pub feature_id: i64,
    pub standard: String,
    pub decision: String,
    pub decided_on: String,
    pub note: String,
    pub mz: String,
    pub rt: String,
}

/// The chemist's Level 1 decisions (file `pol;feature_id;standard;decision;decided_on;note;mz;rt`); pending rows (empty decision) are dropped.
pub fn load_decisions(text: Option<&str>, path: &str) -> Result<Vec<Level1Decision>> {
    let Some(text) = text else {
        return Ok(Vec::new());
    };
    let d = Frame::parse(text, ';')?;
    for name in ["pol", "feature_id", "decision"] {
        if d.col(name).is_none() {
            return Err(CampaignError::value(format!("KeyError: '{name}'")));
        }
    }
    let get = |r: usize, n: &str| d.col(n).map_or(String::new(), |k| d.cell(r, k).to_string());
    let mut bad = BTreeSet::new();
    let mut out = Vec::new();
    for r in 0..d.len() {
        let decision = get(r, "decision").trim().to_lowercase();
        if !matches!(decision.as_str(), "confirm" | "reject" | "") {
            bad.insert(decision.clone());
        }
        let Some(fid) = to_num(&get(r, "feature_id")) else {
            continue;
        };
        if decision.is_empty() {
            continue;
        }
        out.push(Level1Decision {
            pol: get(r, "pol").trim().to_uppercase(),
            feature_id: fid as i64,
            standard: get(r, "standard"),
            decision,
            decided_on: get(r, "decided_on"),
            note: get(r, "note"),
            mz: get(r, "mz"),
            rt: get(r, "rt"),
        });
    }
    if !bad.is_empty() {
        let found: Vec<&String> = bad.iter().collect();
        return Err(CampaignError::value(format!(
            "{path}: decision must be 'confirm', 'reject' or empty (pending), found {}",
            py_list(&found)
        )));
    }
    Ok(out)
}

/// A feature as the matching sees it: id, m/z, RT.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeatureRef {
    pub id: i64,
    pub mz: Option<f64>,
    pub rt: Option<f64>,
}

/// A feature within mass and RT tolerance of a standard.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub pol: String,
    pub id: i64,
    pub name: String,
    pub formula: Option<String>,
    pub category: Option<String>,
    pub ppm: f64,
    pub drt: f64,
    pub adduct: String,
    pub status: String,
    pub cosine: Option<f64>,
    pub reverse: Option<f64>,
    pub matched: Option<i64>,
}

/// `round(x, n)` of Python (the correctly rounded decimal).
pub fn py_round(x: f64, n: usize) -> f64 {
    format!("{x:.n$}").parse().unwrap_or(x)
}

/// The mass and RT tolerances and columns of a polarity.
pub struct Tolerances<'a> {
    pub mass_tol_ppm: f64,
    pub rt_tol_min: f64,
    pub rt_column: &'a str,
    pub adducts: Option<&'a [String]>,
}

fn library_rows(lib: &Frame, pol: &str, rt_column: &str) -> Vec<(usize, f64)> {
    let esi = if pol == "POS" { "ESI_pos" } else { "ESI_neg" };
    (0..lib.len())
        .filter_map(|r| {
            num_at(lib, r, rt_column)
                .filter(|v| *v > 0.0)
                .map(|v| (r, v))
        })
        .filter(|(r, _)| {
            lib.col("comment")
                .is_none_or(|k| lib.cell(*r, k).trim() != INTERNAL_STANDARD_COMMENT)
        })
        .filter(|(r, _)| {
            lib.col(esi)
                .is_none_or(|k| lib.cell(*r, k).trim().to_uppercase() == "YES")
        })
        .collect()
}

fn best_adduct(lib: &Frame, r: usize, adducts: &[&String], mz: f64) -> Option<(f64, String)> {
    let mut best: Option<(f64, String)> = None;
    for a in adducts {
        let Some(v) = num_at(lib, r, a).filter(|v| *v > 0.0) else {
            continue;
        };
        let ppm = (mz - v).abs() / v * 1e6;
        if ppm < best.as_ref().map_or(f64::INFINITY, |b| b.0) {
            best = Some((ppm, a.to_string()));
        }
    }
    best
}

/// Every (feature, standard) pair within mass and RT tolerance, in library order and then feature order.
pub fn mass_rt_candidates(
    features: &[FeatureRef],
    pol: &str,
    lib: &Frame,
    tol: &Tolerances,
) -> Result<Vec<Candidate>> {
    if lib.is_empty() || lib.col(tol.rt_column).is_none() || features.is_empty() {
        return Ok(Vec::new());
    }
    let default: Vec<String> = (if pol == "POS" {
        DEFAULT_POS_ADDUCTS.to_vec()
    } else {
        DEFAULT_NEG_ADDUCTS.to_vec()
    })
    .into_iter()
    .map(String::from)
    .collect();
    let wanted = tol.adducts.unwrap_or(&default);
    let adducts: Vec<&String> = wanted.iter().filter(|a| lib.col(a).is_some()).collect();
    if lib.col("name").is_none() {
        return Err(CampaignError::value("KeyError: 'name'"));
    }
    let mut out = Vec::new();
    for (r, std_rt) in library_rows(lib, pol, tol.rt_column) {
        for f in features {
            let (Some(mz), Some(rt)) = (f.mz, f.rt) else {
                continue;
            };
            let drt = (rt - std_rt).abs();
            if drt > tol.rt_tol_min {
                continue;
            }
            let Some((ppm, adduct)) =
                best_adduct(lib, r, &adducts, mz).filter(|b| b.0 <= tol.mass_tol_ppm)
            else {
                continue;
            };
            out.push(Candidate {
                pol: pol.to_string(),
                id: f.id,
                name: lib.cell(r, lib.col("name").unwrap_or(0)).to_string(),
                formula: text_at(lib, r, "formula"),
                category: text_at(lib, r, "molecular_class"),
                ppm: py_round(ppm, 2),
                drt: py_round(drt, 3),
                adduct,
                status: String::new(),
                cosine: None,
                reverse: None,
                matched: None,
            });
        }
    }
    Ok(out)
}

/// The similarity of a feature spectrum to the reference spectra of one standard with the same precursor ion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Similarity {
    pub cosine: f64,
    pub reverse: f64,
    pub matched: usize,
}

/// `compare(sample, library, name, precursor_tol_ppm, tol_da)`: the best similarity, `None` when the library has no spectrum of that ion.
pub type Comparer<'a> = &'a dyn Fn(&Spectrum, &RefLibrary, &str, f64, f64) -> Option<Similarity>;

/// The thresholds and inputs of the MS/MS check.
pub struct Ms2Check<'a> {
    pub feature_spectra: &'a HashMap<i64, Spectrum>,
    pub library_spectra: &'a RefLibrary,
    pub compare: Comparer<'a>,
    pub min_cosine: f64,
    pub min_matched: i64,
    pub tol_da: f64,
    pub precursor_tol_ppm: f64,
}

fn status_of(
    sim: Option<Similarity>,
    has_ms2: bool,
    min_cos: f64,
    min_matched: i64,
) -> &'static str {
    if !has_ms2 {
        return "no_ms2";
    }
    let Some(s) = sim else {
        return "no_reference_ion";
    };
    if s.matched as i64 >= min_matched && s.cosine >= min_cos {
        "confirmed"
    } else if s.cosine >= min_cos || s.reverse >= min_cos {
        "review"
    } else {
        "mismatch"
    }
}

fn check_ms2(cand: &mut Candidate, mz: Option<f64>, k: &Ms2Check) {
    if !PROTONATED.contains(&cand.adduct.as_str()) {
        let ions = k
            .library_spectra
            .get(cand.name.trim())
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let seen = mz.is_some_and(|m| {
            ions.iter()
                .map(|s| s.precursor)
                .filter(|i| i.is_finite())
                .any(|i| (m - i).abs() / i * 1e6 <= k.precursor_tol_ppm)
        });
        if !seen {
            cand.status = "unobserved_ion".into();
            return;
        }
    }
    let spec = k.feature_spectra.get(&cand.id);
    let sim = spec.and_then(|s| {
        (k.compare)(
            s,
            k.library_spectra,
            &cand.name,
            k.precursor_tol_ppm,
            k.tol_da,
        )
    });
    cand.status = status_of(sim, spec.is_some(), k.min_cosine, k.min_matched).into();
    if let Some(s) = sim {
        cand.cosine = Some(py_round(s.cosine, 3));
        cand.reverse = Some(py_round(s.reverse, 3));
        cand.matched = Some(s.matched as i64);
    }
}

fn apply_decisions(
    cand: &mut [Candidate],
    features: &[FeatureRef],
    pol: &str,
    decisions: &[Level1Decision],
    warnings: &mut Vec<String>,
) {
    let at = |id: i64| features.iter().find(|f| f.id == id);
    for x in decisions.iter().filter(|d| d.pol == pol) {
        let same = |c: &Candidate| c.name.trim() == x.standard.trim();
        let mut hit: Vec<usize> = (0..cand.len())
            .filter(|&i| cand[i].id == x.feature_id && same(&cand[i]))
            .collect();
        if hit.is_empty() {
            if let (Some(xmz), Some(xrt)) = (to_num(&x.mz), to_num(&x.rt)) {
                hit = (0..cand.len())
                    .filter(|&i| {
                        let f = at(cand[i].id);
                        same(&cand[i])
                            && f.and_then(|f| f.mz)
                                .is_some_and(|m| (m - xmz).abs() / xmz * 1e6 <= DECISION_MZ_PPM)
                            && f.and_then(|f| f.rt)
                                .is_some_and(|t| (t - xrt).abs() <= DECISION_RT_MIN)
                    })
                    .collect();
            }
        }
        if hit.is_empty() {
            warnings.push(format!(
                "Level 1 decision not applied (feature not found): {pol} {} {} -- check level1_decisions.csv",
                x.feature_id, x.standard
            ));
        }
        for i in hit {
            cand[i].status = if x.decision == "confirm" {
                "confirmed_manual"
            } else {
                "rejected_manual"
            }
            .into();
        }
    }
}

/// The best standard of a feature with the diagnostics of the others.
#[derive(Clone, Debug, PartialEq)]
pub struct StandardMatch {
    pub pol: String,
    pub id: i64,
    pub name: Option<String>,
    pub formula: Option<String>,
    pub category: Option<String>,
    pub ppm: f64,
    pub drt: f64,
    pub adduct: String,
    pub status: String,
    pub cosine: Option<f64>,
    pub reverse: Option<f64>,
    pub matched: Option<i64>,
    pub n_candidates: usize,
    pub ambiguous: bool,
    pub candidates: String,
    pub rejected: Option<String>,
}

impl Record for StandardMatch {
    const COLUMNS: &'static [&'static str] = &[
        "_pol",
        "_id",
        "std_name",
        "std_formula",
        "std_category",
        "std_ppm",
        "std_drt",
        "std_adduct",
        "std_status",
        "std_ms2_cosine",
        "std_ms2_reverse",
        "std_ms2_matched",
        "std_n_candidates",
        "std_ambiguous",
        "std_candidates",
        "std_rejected",
    ];
    fn cells(&self) -> Vec<Cell> {
        vec![
            Cell::Str(self.pol.clone()),
            Cell::Int(self.id),
            Cell::str(&self.name),
            Cell::str(&self.formula),
            Cell::str(&self.category),
            Cell::Num(self.ppm),
            Cell::Num(self.drt),
            Cell::Str(self.adduct.clone()),
            Cell::Str(self.status.clone()),
            Cell::num(self.cosine),
            Cell::num(self.reverse),
            self.matched.map_or(Cell::Null, Cell::Int),
            Cell::Int(self.n_candidates as i64),
            Cell::Bool(self.ambiguous),
            Cell::Str(self.candidates.clone()),
            Cell::str(&self.rejected),
        ]
    }
}

fn collect_matches(cand: Vec<Candidate>) -> Vec<StandardMatch> {
    let mut order: Vec<(String, i64)> = Vec::new();
    let mut groups: HashMap<(String, i64), Vec<Candidate>> = HashMap::new();
    for c in cand {
        let key = (c.pol.clone(), c.id);
        if !groups.contains_key(&key) {
            order.push(key.clone());
        }
        groups.entry(key).or_default().push(c);
    }
    order
        .into_iter()
        .map(|key| {
            let g = &groups[&key];
            let names: BTreeSet<&str> = g.iter().map(|c| c.name.as_str()).collect();
            let ok: Vec<&Candidate> = g
                .iter()
                .filter(|c| !REJECTED_STATUSES.contains(&c.status.as_str()))
                .collect();
            let ok_names: BTreeSet<&str> = ok.iter().map(|c| c.name.as_str()).collect();
            let rej: Vec<String> = g
                .iter()
                .filter(|c| REJECTED_STATUSES.contains(&c.status.as_str()))
                .map(|c| format!("{} ({})", c.name, c.status))
                .collect();
            let b = &g[0];
            let named = !ok.is_empty();
            StandardMatch {
                pol: b.pol.clone(),
                id: b.id,
                name: named.then(|| b.name.clone()),
                formula: b.formula.clone().filter(|_| named),
                category: b.category.clone().filter(|_| named),
                ppm: b.ppm,
                drt: b.drt,
                adduct: b.adduct.clone(),
                status: b.status.clone(),
                cosine: b.cosine,
                reverse: b.reverse,
                matched: b.matched,
                n_candidates: names.len(),
                ambiguous: ok_names.len() > 1,
                candidates: names.iter().copied().collect::<Vec<_>>().join(" | "),
                rejected: (!rej.is_empty()).then(|| rej.join(" | ")),
            }
        })
        .collect()
}

/// One row per feature with a mass + RT candidate. Without library spectra every candidate is `no_reference_ion`: nothing reaches Level 1 without MS/MS.
pub fn match_reference_standards(
    features: &[FeatureRef],
    pol: &str,
    lib: &Frame,
    tol: &Tolerances,
    ms2: &Ms2Check,
    decisions: &[Level1Decision],
    warnings: &mut Vec<String>,
) -> Result<Vec<StandardMatch>> {
    let mut cand = mass_rt_candidates(features, pol, lib, tol)?;
    if cand.is_empty() {
        return Ok(Vec::new());
    }
    for c in cand.iter_mut() {
        let mz = features.iter().find(|f| f.id == c.id).and_then(|f| f.mz);
        check_ms2(c, mz, ms2);
    }
    apply_decisions(&mut cand, features, pol, decisions, warnings);
    cand.sort_by(|a, b| {
        let ord = |x: f64, y: f64| x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal);
        rank(&a.status)
            .cmp(&rank(&b.status))
            .then(ord(a.ppm, b.ppm))
            .then(ord(a.drt, b.drt))
    });
    Ok(collect_matches(cand))
}
