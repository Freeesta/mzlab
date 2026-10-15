// Ported from Attributio chem/checks.py.
//! Coherence of the annotations against the chemical registry (warnings only; nothing here changes a count). Every check returns `Finding`s: check, severity,
//! campaign, identity, InChIKey first block, detail, value. The registry, the members and the evidence are typed rows given by the caller (`Context`); the
//! detail texts are those of Attributio. Not ported: the checks `categories` and `toxicity` (PubChemLite categories, aquatic hazard records).

use std::collections::{BTreeMap, BTreeSet, HashMap};

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

/// (a) one structure under several names (same campaign: MERGE; several campaigns: curation).
fn names_of_one_structure(ctx: &Context, out: &mut Vec<Finding>) {
    let mut by_block: BTreeMap<&str, Vec<&Member>> = BTreeMap::new();
    for m in ctx.members.iter().filter(|m| m.first_block.is_some()) {
        by_block
            .entry(m.first_block.as_deref().unwrap_or(""))
            .or_default()
            .push(m);
    }
    for (blk, g) in by_block {
        let names: BTreeSet<&str> = g.iter().map(|m| m.name.as_str()).collect();
        if names.len() < 2 {
            continue;
        }
        let mut by_camp: BTreeMap<&str, Vec<&Member>> = BTreeMap::new();
        for m in &g {
            by_camp.entry(m.campaign.as_str()).or_default().push(m);
        }
        for (camp, h) in &by_camp {
            let hn: BTreeSet<&str> = h.iter().map(|m| m.name.as_str()).collect();
            if hn.len() > 1 {
                let value = h
                    .iter()
                    .map(|x| format!("{} [{}, L{}]", x.name, x.ik_basis, x.best_level))
                    .collect::<Vec<_>>()
                    .join("; ");
                out.push(finding(
                    CHECK_NAME_INCHIKEY,
                    WARNING,
                    camp,
                    &hn.iter().copied().collect::<Vec<_>>().join(" | "),
                    blk,
                    "MERGE: same structure under different names in the same campaign -> counted multiple times by identity_table()".into(),
                    Value::Text(value),
                ));
            }
        }
        if by_camp.len() > 1 {
            let per: Vec<(&str, BTreeSet<&str>)> = by_camp
                .iter()
                .map(|(c, h)| (*c, h.iter().map(|m| m.name.as_str()).collect()))
                .collect();
            if per.iter().map(|p| &p.1).collect::<BTreeSet<_>>().len() > 1 {
                let value = per
                    .iter()
                    .map(|(c, v)| {
                        format!("{c}: {}", v.iter().copied().collect::<Vec<_>>().join(", "))
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                out.push(finding(
                    CHECK_NAME_INCHIKEY,
                    INFO,
                    &per.iter().map(|p| p.0).collect::<Vec<_>>().join("+"),
                    &names.iter().copied().collect::<Vec<_>>().join(" | "),
                    blk,
                    "same structure with different names across campaigns (curation to align?)"
                        .into(),
                    Value::Text(value),
                ));
            }
        }
    }
}

/// (b) one name over several structures.
fn structures_of_one_name(ctx: &Context, out: &mut Vec<Finding>) {
    let mut by_name: BTreeMap<(&str, &str), Vec<&Member>> = BTreeMap::new();
    for m in ctx.members.iter().filter(|m| m.first_block.is_some()) {
        by_name
            .entry((m.campaign.as_str(), m.name.as_str()))
            .or_default()
            .push(m);
    }
    for ((camp, name), g) in by_name {
        let mut blocks: Vec<&str> = g.iter().filter_map(|m| m.first_block.as_deref()).collect();
        if blocks.iter().collect::<BTreeSet<_>>().len() < 2 {
            continue;
        }
        let value = g
            .iter()
            .map(|x| {
                format!(
                    "{} [{}, {} features]",
                    x.first_block.as_deref().unwrap_or(""),
                    x.ik_basis,
                    x.n_features
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        blocks.sort_unstable();
        out.push(finding(
            CHECK_NAME_INCHIKEY,
            WARNING,
            camp,
            name,
            &blocks.join("|"),
            "SPLIT: the same name covers different structures (intended homologous series, or name and structure from different sources)".into(),
            Value::Text(value),
        ));
    }
}

/// Four consecutive ASCII lowercase letters (`[a-z]{4}`).
fn has_word(s: &str) -> bool {
    let l = s.to_lowercase();
    let mut run = 0;
    for c in l.chars() {
        run = if c.is_ascii_lowercase() { run + 1 } else { 0 };
        if run >= 4 {
            return true;
        }
    }
    false
}

/// (c) a name that is not among the PubChem synonyms of its structure.
fn names_vs_synonyms(ctx: &Context, out: &mut Vec<Finding>) {
    let mut syn: HashMap<&str, BTreeSet<String>> = HashMap::new();
    for p in &ctx.pubchem {
        let mut s: BTreeSet<String> = p.synonyms.split('|').map(norm_name).collect();
        s.insert(norm_name(&p.title));
        syn.insert(p.cid.as_str(), s);
    }
    let mut ev_names: HashMap<(&str, &str), BTreeSet<&str>> = HashMap::new();
    for e in &ctx.evidence {
        if let Some(n) = e.source_name.as_deref() {
            ev_names
                .entry((e.campaign.as_str(), e.identity.as_str()))
                .or_default()
                .insert(n);
        }
    }
    for m in ctx.members.iter().filter(|m| m.first_block.is_some()) {
        let Some(c) = ctx.compound(m.first_block.as_deref()) else {
            continue;
        };
        if c.pubchem_cid.is_empty() || m.ik_basis.starts_with("PubChem name") {
            continue;
        }
        let Some(s) = syn.get(c.pubchem_cid.as_str()).filter(|s| !s.is_empty()) else {
            continue;
        };
        let mut cands = vec![m.name.as_str()];
        cands.extend(
            ev_names
                .get(&(m.campaign.as_str(), m.name.as_str()))
                .into_iter()
                .flatten()
                .copied(),
        );
        if cands
            .iter()
            .any(|x| !norm_name(x).is_empty() && s.contains(&norm_name(x)))
        {
            continue;
        }
        if !has_word(&m.name) || m.group == "ambiguous" {
            continue;
        }
        let detail = format!(
            "name not among the PubChem synonyms of CID {} ('{}') [structure from: {}]",
            c.pubchem_cid, c.pubchem_title, m.ik_basis
        );
        let sev = if m.group == "contaminant" {
            WARNING
        } else {
            INFO
        };
        let value = cands
            .iter()
            .skip(1)
            .take(3)
            .copied()
            .collect::<Vec<_>>()
            .join("; ");
        out.push(finding(
            CHECK_NAME_INCHIKEY,
            sev,
            &m.campaign,
            &m.name,
            m.first_block.as_deref().unwrap_or(""),
            detail,
            Value::Text(value),
        ));
    }
}

/// (d) the structure from the name source against the loader's skeleton.
fn name_vs_loader(ctx: &Context, out: &mut Vec<Finding>) {
    let mut groups: BTreeMap<(&str, &str), Vec<&Evidence>> = BTreeMap::new();
    for e in &ctx.evidence {
        if let (Some(n), Some(l)) = (&e.name_block, &e.loader_block) {
            if n != l {
                groups
                    .entry((e.campaign.as_str(), e.identity.as_str()))
                    .or_default()
                    .push(e);
            }
        }
    }
    for ((camp, name), g) in groups {
        let first = g[0];
        let loaders: BTreeSet<&str> = g.iter().filter_map(|e| e.loader_block.as_deref()).collect();
        let detail = format!(
            "the structure from the name source ({}) differs from the loader's inchikey_skeleton ({}) in {} features ('problem A'): the registry uses the one from the name source",
            first.name_source,
            first.loader_source,
            g.len()
        );
        let value = loaders.into_iter().collect::<Vec<_>>().join("|");
        out.push(finding(
            CHECK_NAME_INCHIKEY,
            INFO,
            camp,
            name,
            first.name_block.as_deref().unwrap_or(""),
            detail,
            Value::Text(value),
        ));
    }
}

/// (e) mzMine library entries whose structure contradicts their own precursor.
fn mzlib_bugs(ctx: &Context, out: &mut Vec<Finding>) {
    for (camp, rows) in &ctx.mzlib {
        let mut by_key: BTreeMap<&str, Vec<&MzlibCheck>> = BTreeMap::new();
        for r in rows.iter().filter(|r| r.structure_ok == Some(false)) {
            if let Some(k) = r.inchi_key.as_deref().filter(|k| k.chars().count() >= 14) {
                by_key.entry(k).or_default().push(r);
            }
        }
        for (k, g) in by_key {
            let names: BTreeSet<&str> = g.iter().map(|r| r.name.as_str()).collect();
            let identity = names.into_iter().take(6).collect::<Vec<_>>().join(" | ");
            let abs: Vec<f64> = g.iter().map(|r| r.structure_ppm.abs()).collect();
            let med = super::rt_logp::median(&abs);
            let detail = format!(
                "mzMine library bug: {} entries ({}) carry InChIKey {k} ({}) inconsistent with their precursor (median {med:.0} ppm): structure ignored",
                g.len(),
                g[0].pol,
                g[0].mol_formula
            );
            let block: String = k.chars().take(14).collect();
            out.push(finding(
                CHECK_NAME_INCHIKEY,
                INFO,
                camp,
                &identity,
                &block,
                detail,
                Value::Int(g.len() as i64),
            ));
        }
    }
}

/// One structure under several names, one name over several structures, a name outside the PubChem synonyms, the name source against the loader, and the
/// library entries that contradict their own precursor.
pub fn check_names(ctx: &Context) -> Vec<Finding> {
    let mut out = Vec::new();
    names_of_one_structure(ctx, &mut out);
    structures_of_one_name(ctx, &mut out);
    names_vs_synonyms(ctx, &mut out);
    name_vs_loader(ctx, &mut out);
    mzlib_bugs(ctx, &mut out);
    out
}

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
