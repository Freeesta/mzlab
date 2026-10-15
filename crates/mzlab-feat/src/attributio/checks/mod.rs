// Ported from Attributio chem/checks.py.
//! Coherence of the annotations against the chemical registry (warnings only; nothing here changes a count). Every check returns `Finding`s: check, severity,
//! campaign, identity, InChIKey first block, detail, value. The registry, the members and the evidence are typed rows given by the caller (`Context`); the
//! detail texts are those of Attributio. Not ported: the checks `categories` and `toxicity` (PubChemLite categories, aquatic hazard records).

use std::collections::HashMap;

use super::annotate::types::Feature;
use super::annotate::Annotations;
use super::formula::{
    best_common_adduct, ion_mz, neutral_equivalent, parse_adduct, ppm, structure_fit,
    STRUCTURE_TOL_PPM,
};
use super::rt_logp::{build_batch_fit, rt_vs_batch, FitInput, Fitted, Z_FLAG};

pub const CHECK_COVERAGE: &str = "coverage";
pub const CHECK_MASS: &str = "mass";
pub const CHECK_NAME_INCHIKEY: &str = "name_inchikey";
pub const CHECK_XLOGP_RT: &str = "xlogp_rt";
pub const WARNING: &str = "warning";
pub const INFO: &str = "info";
/// Mass tolerance of a campaign without a `standards` table.
pub const DEFAULT_MASS_TOL_PPM: f64 = 5.0;

/// The value column of a finding.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Text(String),
    Num(f64),
    Int(i64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub check: &'static str,
    pub severity: &'static str,
    pub campaign: String,
    pub identity: String,
    pub inchikey_first_block: String,
    pub detail: String,
    pub value: Value,
}

fn finding(
    check: &'static str,
    severity: &'static str,
    campaign: &str,
    identity: &str,
    block: &str,
    detail: String,
    value: Value,
) -> Finding {
    Finding {
        check,
        severity,
        campaign: campaign.to_string(),
        identity: identity.to_string(),
        inchikey_first_block: block.to_string(),
        detail,
        value,
    }
}

/// `round(x, 1)` of Python.
fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// Lowercase ASCII letters and digits only, a trailing qualifier `(...)` removed.
pub fn norm_name(s: &str) -> String {
    let s = strip_qualifier(s);
    s.chars()
        .flat_map(fold_char)
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect()
}

/// Removes `\s*\([^()]*\)\s*$`.
fn strip_qualifier(s: &str) -> &str {
    let t = s.trim_end();
    if !t.ends_with(')') {
        return s;
    }
    let Some(open) = t.rfind('(') else { return s };
    if t[open + 1..t.len() - 1].contains(['(', ')']) {
        return s;
    }
    t[..open].trim_end()
}

/// The ASCII letter a Latin character decomposes to (NFKD then ASCII), lowercased; characters with no ASCII form give nothing.
fn fold_char(c: char) -> Vec<char> {
    if c.is_ascii() {
        return vec![c.to_ascii_lowercase()];
    }
    let base = match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å'
        | 'Ā' | 'Ă' | 'Ą' => 'a',
        'ç' | 'ć' | 'č' | 'Ç' | 'Ć' | 'Č' => 'c',
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' | 'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ė' | 'Ę'
        | 'Ě' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' => 'i',
        'ñ' | 'ń' | 'ň' | 'Ñ' | 'Ń' | 'Ň' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ō' | 'ő' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ō' | 'Ő' => {
            'o'
        }
        'š' | 'ś' | 'ş' | 'Š' | 'Ś' | 'Ş' => 's',
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' | 'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ū' | 'Ů' | 'Ű' => {
            'u'
        }
        'ý' | 'ÿ' | 'Ý' | 'Ÿ' => 'y',
        'ž' | 'ź' | 'ż' | 'Ž' | 'Ź' | 'Ż' => 'z',
        _ => return Vec::new(),
    };
    vec![base]
}

/// A compound of the registry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Compound {
    pub first_block: String,
    pub formula: String,
    pub monoisotopic_mass: Option<f64>,
    pub xlogp: Option<f64>,
    pub xlogp_source: String,
    pub pubchem_cid: String,
    pub pubchem_title: String,
}

/// An identity of a campaign with the structure the registry resolved for it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Member {
    pub campaign: String,
    pub name: String,
    pub first_block: Option<String>,
    pub ik_basis: String,
    pub name_sources: String,
    pub best_level: i64,
    pub n_features: i64,
    pub group: String,
}

/// One feature annotated with an identity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Evidence {
    pub campaign: String,
    pub identity: String,
    pub pol: String,
    pub id: i64,
    pub mz: f64,
    pub rt: Option<f64>,
    pub adduct: Option<String>,
    pub level: i64,
    pub name_block: Option<String>,
    pub loader_block: Option<String>,
    pub name_source: String,
    pub loader_source: String,
    pub source_name: Option<String>,
}

/// The PubChem record of a CID.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pubchem {
    pub cid: String,
    pub title: String,
    pub synonyms: String,
}

/// A library entry of the annotation export with its structure checked against its own precursor.
#[derive(Clone, Debug, PartialEq)]
pub struct MzlibCheck {
    pub pol: String,
    pub name: String,
    pub inchi_key: Option<String>,
    pub mol_formula: String,
    pub structure_ppm: f64,
    pub structure_ok: Option<bool>,
}

/// Everything the checks read.
#[derive(Clone, Debug, Default)]
pub struct Context {
    pub compounds: HashMap<String, Compound>,
    /// Identity members only.
    pub members: Vec<Member>,
    pub evidence: Vec<Evidence>,
    pub pubchem: Vec<Pubchem>,
    pub campaigns: Vec<String>,
    pub mass_tol: HashMap<String, f64>,
    pub fits: HashMap<String, Vec<Fitted>>,
    pub mzlib: Vec<(String, Vec<MzlibCheck>)>,
}

impl Context {
    fn compound(&self, block: Option<&str>) -> Option<&Compound> {
        self.compounds.get(block?)
    }

    /// Registry block of one evidence row: its name structure, else the block of its single identity member.
    pub fn block_of(&self, e: &Evidence) -> Option<String> {
        if let Some(b) = &e.name_block {
            return Some(b.clone());
        }
        let blocks: Vec<&String> = self
            .members
            .iter()
            .filter(|m| m.campaign == e.campaign && m.name == e.identity)
            .filter_map(|m| m.first_block.as_ref())
            .collect();
        (blocks.len() == 1).then(|| blocks[0].clone())
    }
}

/// Identities without a structure (INFO) and structures missing from the registry (WARNING).
pub fn check_coverage(ctx: &Context) -> Vec<Finding> {
    let mut out = Vec::new();
    for m in &ctx.members {
        match &m.first_block {
            None => out.push(finding(
                CHECK_COVERAGE,
                INFO,
                &m.campaign,
                &m.name,
                "",
                "identity without structure: no InChIKey from the name source, name not found on PubChem".into(),
                Value::Text(m.name_sources.clone()),
            )),
            Some(b) if ctx.compound(Some(b)).is_none() => out.push(finding(
                CHECK_COVERAGE,
                WARNING,
                &m.campaign,
                &m.name,
                b,
                "structure not yet in the registry: attributio registro --aggiorna".into(),
                Value::Text(m.ik_basis.clone()),
            )),
            _ => {}
        }
    }
    out
}

/// The m/z of a feature against the ion of its registry compound.
pub fn check_mass(ctx: &Context) -> Vec<Finding> {
    let mut out = Vec::new();
    for e in &ctx.evidence {
        let blk = ctx.block_of(e);
        let Some(c) = ctx.compound(blk.as_deref()) else {
            continue;
        };
        let Some(mass) = c.monoisotopic_mass else {
            continue;
        };
        let m = neutral_equivalent(mass, &c.formula);
        let t = ctx
            .mass_tol
            .get(&e.campaign)
            .copied()
            .unwrap_or(DEFAULT_MASS_TOL_PPM);
        let add = e.adduct.as_deref();
        let theory = add
            .filter(|a| parse_adduct(a).is_some())
            .and_then(|a| ion_mz(m, a));
        let (ba, be) = best_common_adduct(e.mz, m, &e.pol);
        let who = format!("{} {} m/z {:.4}", e.pol, e.id, e.mz);
        let blk = blk.unwrap_or_default();
        if let Some(th) = theory {
            let err = ppm(e.mz, th);
            if err.abs() <= t {
                continue;
            }
            let tail = if be.abs() <= t {
                format!("; explained instead as {ba} ({be:+.1} ppm)")
            } else {
                "; no common adduct within tolerance".into()
            };
            let det = format!(
                "{who}: {} of {} expected {th:.4} ({err:+.1} ppm){tail}",
                add.unwrap_or(""),
                c.formula
            );
            out.push(finding(
                CHECK_MASS,
                WARNING,
                &e.campaign,
                &e.identity,
                &blk,
                det,
                Value::Num(round1(err)),
            ));
        } else if be.abs() > t {
            let det = format!(
                "{who}: adduct not reported; the closest for {} is {ba} at {be:+.1} ppm",
                c.formula
            );
            out.push(finding(
                CHECK_MASS,
                WARNING,
                &e.campaign,
                &e.identity,
                &blk,
                det,
                Value::Num(round1(be)),
            ));
        }
    }
    out
}

mod names;
pub use names::check_names;

/// Level 3 features whose level comes from CSI:FingerID, as the input of the isotonic fit.
pub fn csi_level3(feats: &[Feature]) -> Vec<FitInput> {
    feats
        .iter()
        .filter(|f| f.level == 3 && f.level_basis == "CSI:FingerID")
        .map(|f| FitInput {
            pol: f.pol.clone(),
            id: f.id,
            xlogp: f.csi.xlogp,
            rt: f.rt,
        })
        .collect()
}

/// The fit of a campaign, ready for `check_xlogp_rt`.
pub fn campaign_fit(a: &Annotations) -> Vec<Fitted> {
    build_batch_fit(&csi_level3(&a.features))
}

/// The RT of an identity against the XLogP of its registry compound and the isotonic fit of its campaign (robust z above `Z_FLAG`).
pub fn check_xlogp_rt(ctx: &Context) -> Vec<Finding> {
    let mut out = Vec::new();
    for camp in &ctx.campaigns {
        let Some(fitted) = ctx.fits.get(camp) else {
            continue;
        };
        for e in ctx.evidence.iter().filter(|e| &e.campaign == camp) {
            let blk = ctx.block_of(e);
            let Some(c) = ctx.compound(blk.as_deref()) else {
                continue;
            };
            let Some(xlogp) = c.xlogp else { continue };
            let (rt_pred, z) = rt_vs_batch(fitted, &e.pol, xlogp, e.rt);
            let (Some(z), Some(rt)) = (z, e.rt) else {
                continue;
            };
            if z.abs() > Z_FLAG {
                let detail = format!(
                    "{} {}: RT {rt:.2} min, expected ~{:.2} for XLogP {xlogp:.1} ({}); z = {z:+.1} (campaign isotonic fit, level L{})",
                    e.pol,
                    e.id,
                    rt_pred.unwrap_or(f64::NAN),
                    c.xlogp_source,
                    e.level
                );
                out.push(finding(
                    CHECK_XLOGP_RT,
                    WARNING,
                    camp,
                    &e.identity,
                    &blk.unwrap_or_default(),
                    detail,
                    Value::Num(round1(z)),
                ));
            }
        }
    }
    out
}

/// The library entries of a polarity with their structure checked against their own precursor; built by the caller from the rows of the annotation export.
pub fn mzlib_check_row(
    pol: &str,
    compound_name: &str,
    adduct: Option<&str>,
    precursor: Option<f64>,
    mol_formula: Option<&str>,
    inchi_key: Option<&str>,
) -> MzlibCheck {
    let (ppm, ok) = structure_fit(mol_formula, adduct, precursor, pol, STRUCTURE_TOL_PPM);
    MzlibCheck {
        pol: pol.to_string(),
        name: super::readers::gnps::clean_library_hit_name(compound_name),
        inchi_key: inchi_key.map(str::to_string),
        mol_formula: mol_formula.unwrap_or("nan").to_string(),
        structure_ppm: ppm,
        structure_ok: ok,
    }
}

/// Every library hit of the annotation export of both polarities with its structure checked against its own precursor (`mzlib_structure_check`).
pub fn mzlib_structure_check(c: &super::Campaign, io: &super::Io) -> Vec<MzlibCheck> {
    let mut out = Vec::new();
    for pol in ["POS", "NEG"] {
        let Some(path) = super::readers::mzmine::annotation_file(c, io, pol) else {
            continue;
        };
        let Some(bytes) = io.read(&path) else {
            continue;
        };
        let Ok(frame) = super::Frame::parse(&String::from_utf8_lossy(&bytes), ',') else {
            continue;
        };
        let col = |n: &str| frame.col(n);
        for r in 0..frame.len() {
            let s = |n: &str| col(n).and_then(|i| frame.str_na(r, i));
            out.push(mzlib_check_row(
                pol,
                s("compound_name").unwrap_or("nan"),
                s("adduct"),
                col("precursor_mz").and_then(|i| frame.num(r, i)),
                s("mol_formula"),
                s("inchi_key"),
            ));
        }
    }
    out
}
