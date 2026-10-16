// Ported from Attributio chem/members.py.
//! Which compounds a project needs in its registry, and on what evidence. Every identity of Levels 1-3 of a campaign is a member, named exactly as
//! `identity_table` names it. The structure of an identity is taken from THE SOURCE THAT GAVE IT ITS NAME, not from the loader's `inchikey_skeleton` (which
//! prefers the CSI:FingerID structure even when the name comes from a spectral library):
//!
//! | name source | structure |
//! |---|---|
//! | reference standard | the standards library row (full InChIKey) |
//! | CSI:FingerID | SIRIUS InChIkey2D and SMILES |
//! | SIRIUS spectral | the matched reference's InChIkey2D and SMILES |
//! | GNPS2 | InChIKey-Planar and SMILES of the exact-mass hit |
//! | mzMine library | InChIKey and SMILES of the hit, ONLY if its formula agrees with the entry's own precursor (a failed lookup of the export silently reuses the previous structure) |
//!
//! When the name source carries no usable structure the identity is resolved later by its name (`lookup_name`). Nothing here changes a count.

use super::annotate::{Curation, Feature};
use super::formula::{structure_fit, STRUCTURE_TOL_PPM};
use super::identity::{curated, inchikey_skeleton};
use super::io::{join, Io};
use super::pattern::Pattern;
use super::readers::sirius::load_sirius;
use super::untid::{untid_column, Untid};
use super::{Campaign, Result, Sources};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// What the typed rows of the readers do not keep: the CSI adduct, the structure and reference of the SIRIUS spectral match, the adduct, library and
/// spectrum of the best GNPS2 hit.
#[derive(Clone, Debug, Default)]
pub struct EvidenceExtras {
    csi_adduct: HashMap<(String, i64), String>,
    /// `(SMILES, adduct, reference links)`.
    #[allow(clippy::type_complexity)]
    spec: HashMap<(String, i64), (Option<String>, Option<String>, Option<String>)>,
    /// `(adduct, library, spectrum id)`.
    #[allow(clippy::type_complexity)]
    gnps: HashMap<(String, i64), (Option<String>, Option<String>, Option<String>)>,
}

fn cell(f: &super::Frame, r: usize, col: Option<usize>) -> Option<String> {
    col.and_then(|c| f.str_na(r, c)).map(str::to_string)
}

impl EvidenceExtras {
    /// Reads the extra columns of the SIRIUS and GNPS2 tables of the campaign (a table that is not there adds nothing).
    pub fn load(c: &Campaign, io: &Io, mass_tol_ppm: f64) -> Result<EvidenceExtras> {
        let mut e = EvidenceExtras::default();
        for t in load_sirius(c, io, "structure_identifications", None, true)? {
            let col = t.frame.col("adduct");
            for (r, id) in t.ids.iter().enumerate() {
                if let (Some(id), Some(a)) = (id, cell(&t.frame, r, col)) {
                    e.csi_adduct.entry((t.pol.clone(), *id)).or_insert(a);
                }
            }
        }
        for t in load_sirius(c, io, "spectral_matches", None, true)? {
            let f = &t.frame;
            let (sim, smi, add, link) = (
                f.col("similarity"),
                f.col("referenceSmiles"),
                f.col("referenceAdduct"),
                f.col("referenceLinks"),
            );
            let mut rows: Vec<usize> = (0..f.len()).filter(|&r| t.ids[r].is_some()).collect();
            let score = |r: usize| sim.and_then(|c| f.num(r, c));
            rows.sort_by(|&a, &b| super::readers::desc_na_last(score(a), score(b)));
            for r in rows {
                let key = (t.pol.clone(), t.ids[r].unwrap_or_default());
                e.spec
                    .entry(key)
                    .or_insert_with(|| (cell(f, r, smi), cell(f, r, add), cell(f, r, link)));
            }
        }
        for pol in super::POLARITIES {
            let path = join(
                &c.gnps_output_dir(pol),
                "nf_output/library/merged_results_with_gnps.tsv",
            );
            let Some(g) = super::readers::read_table(io, &path, '\t')? else {
                continue;
            };
            if g.col("#Scan#").is_none() {
                continue;
            }
            e.read_gnps(&g, pol, mass_tol_ppm);
        }
        Ok(e)
    }

    fn read_gnps(&mut self, g: &super::Frame, pol: &str, tol: f64) {
        let num = |r: usize, n: &str| g.col(n).and_then(|c| g.num(r, c));
        let mut rows: Vec<(usize, bool, Option<f64>)> = (0..g.len())
            .filter(|&r| num(r, "#Scan#").is_some())
            .map(|r| {
                let exact = match (num(r, "MassDiff").map(f64::abs), num(r, "SpecMZ")) {
                    (Some(m), Some(s)) => m / s * 1e6 <= tol,
                    _ => false,
                };
                (r, exact, num(r, "MQScore"))
            })
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(super::readers::desc_na_last(a.2, b.2)));
        let (ad, lib, sid) = (g.col("Adduct"), g.col("LibraryName"), g.col("SpectrumID"));
        for (r, _, _) in rows {
            let id = num(r, "#Scan#").unwrap_or_default() as i64;
            self.gnps
                .entry((pol.to_string(), id))
                .or_insert_with(|| (cell(g, r, ad), cell(g, r, lib), cell(g, r, sid)));
        }
    }
}

/// One Level 1-3 feature: its identity, the source that gave it the name and the structure, adduct and m/z that go with that source.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureEvidence {
    pub campaign: String,
    pub pol: String,
    pub id: i64,
    pub mz: Option<f64>,
    pub rt: Option<f64>,
    pub level: i32,
    pub identity: String,
    pub group: String,
    pub category: String,
    /// `reference standard`, `CSI:FingerID`, `SIRIUS spectral`, `GNPS2`, `mzMine library` or `none`.
    pub name_source: String,
    pub source_name: Option<String>,
    pub name_inchikey: Option<String>,
    pub name_block: Option<String>,
    pub name_smiles: Option<String>,
    pub adduct: Option<String>,
    pub spec_reference: Option<String>,
    pub gnps_library: Option<String>,
    pub gnps_spectrum_id: Option<String>,
    pub loader_block: Option<String>,
    pub loader_source: Option<String>,
    pub sirius_formula: Option<String>,
    pub sirius_xlogp: Option<f64>,
    pub mzlib_structure_ok: Option<bool>,
    pub mzlib_structure_ppm: Option<f64>,
    /// The identity that mzLab gives to the same feature (`Untid::text`): a proof beside the identity of Attributio.
    pub untid: Option<String>,
}

fn filled(s: Option<&str>) -> Option<&str> {
    s.filter(|s| !s.trim().is_empty())
}

/// The name of each source of a feature, in the order of precedence of the curated contaminant.
fn candidates(f: &Feature) -> [(&'static str, Option<&str>); 5] {
    [
        (
            "reference standard",
            f.std.as_ref().and_then(|s| s.name.as_deref()),
        ),
        ("CSI:FingerID", f.name()),
        ("SIRIUS spectral", f.spec.name.as_deref()),
        ("GNPS2", f.gnps.name.as_deref().filter(|_| f.gnps.exact)),
        (
            "mzMine library",
            f.mzlib.name.as_deref().filter(|_| f.mzlib.exact),
        ),
    ]
}

/// Which source's name became the identity (same precedence as the cascade and the curation).
fn name_source(f: &Feature, cur: &dyn Curation) -> &'static str {
    let c = candidates(f);
    if let Some((lab, _)) = &f.cec {
        for (src, name) in c {
            if cur.match_cec(&[name]).is_some_and(|m| &m.0 == lab) {
                return src;
            }
        }
    }
    for src in [
        "reference standard",
        "SIRIUS spectral",
        "GNPS2",
        "mzMine library",
        "CSI:FingerID",
    ] {
        if c.iter().any(|(s, n)| *s == src && filled(*n).is_some()) {
            return src;
        }
    }
    "none"
}

type Structure = (Option<String>, Option<String>, Option<String>);

/// `(InChIKey, SMILES, adduct)` that go with the source of the name.
fn structure(
    f: &Feature,
    src: &str,
    x: &EvidenceExtras,
    sources: &Sources,
    fit_ok: Option<bool>,
) -> Structure {
    let key = (f.pol.clone(), f.id);
    let some = |s: &Option<String>| s.clone();
    match src {
        "reference standard" => {
            let lib = &sources.standards_library;
            let row = f
                .std
                .as_ref()
                .and_then(|s| s.name.as_deref())
                .and_then(|n| {
                    let nc = lib.col("name")?;
                    (0..lib.len()).find(|&r| lib.cell(r, nc) == n)
                });
            let at = |n: &str| {
                row.and_then(|r| lib.col(n).and_then(|c| lib.str_na(r, c)))
                    .map(str::to_string)
            };
            (
                at("inchi_key"),
                at("smiles"),
                f.std.as_ref().map(|s| s.adduct.clone()),
            )
        }
        "CSI:FingerID" => (
            some(&f.csi.inchikey2d),
            some(&f.csi.smiles),
            x.csi_adduct.get(&key).cloned(),
        ),
        "SIRIUS spectral" => {
            let s = x.spec.get(&key);
            (
                some(&f.spec.inchikey2d),
                s.and_then(|s| s.0.clone()),
                s.and_then(|s| s.1.clone()),
            )
        }
        "GNPS2" => (
            some(&f.gnps.inchikey),
            some(&f.gnps.smiles),
            x.gnps.get(&key).and_then(|g| g.0.clone()),
        ),
        "mzMine library" if fit_ok == Some(true) => (
            some(&f.mzlib.inchikey),
            some(&f.mzlib.smiles),
            some(&f.mzlib.adduct),
        ),
        "mzMine library" => (None, None, some(&f.mzlib.adduct)),
        _ => (None, None, None),
    }
}

/// One row per Level 1-3 feature: identity (curated), its name source and the structure, adduct and m/z that go with that source.
pub fn feature_evidence(
    campaign_id: &str,
    feats: &[Feature],
    sources: &Sources,
    x: &EvidenceExtras,
    cur: &dyn Curation,
) -> Vec<FeatureEvidence> {
    let untid: HashMap<(&str, i64), Option<Untid>> = feats
        .iter()
        .map(|f| (f.pol.as_str(), f.id))
        .zip(untid_column(feats, &[]))
        .collect();
    curated(feats, &[1, 2, 3], cur)
        .into_iter()
        .map(|d| {
            let f = d.feature;
            let src = name_source(f, cur);
            let (ppm, ok) = structure_fit(
                f.mzlib.formula.as_deref(),
                f.mzlib.adduct.as_deref(),
                f.mzlib.precursor_mz,
                &f.pol,
                STRUCTURE_TOL_PPM,
            );
            let (ik, smi, add) = structure(f, src, x, sources, ok);
            let ik = ik
                .map(|k| k.trim().to_uppercase())
                .filter(|k| k.chars().count() >= 14);
            let key = (f.pol.clone(), f.id);
            FeatureEvidence {
                campaign: campaign_id.to_string(),
                pol: f.pol.clone(),
                id: f.id,
                mz: f.mz,
                rt: f.rt,
                level: f.level,
                category: f
                    .cec
                    .as_ref()
                    .map_or_else(|| d.identity_category.clone(), |c| c.1.clone()),
                identity: d.identity,
                group: d.group,
                name_source: src.to_string(),
                source_name: candidates(f)
                    .iter()
                    .find(|(s, _)| *s == src)
                    .and_then(|(_, n)| n.map(str::to_string)),
                name_block: ik.as_deref().and_then(inchikey_skeleton),
                name_inchikey: ik,
                name_smiles: smi,
                adduct: add,
                spec_reference: x.spec.get(&key).and_then(|s| s.2.clone()),
                gnps_library: x.gnps.get(&key).and_then(|g| g.1.clone()),
                gnps_spectrum_id: x.gnps.get(&key).and_then(|g| g.2.clone()),
                loader_block: f.inchikey_skeleton.clone(),
                loader_source: f.inchikey_source.clone(),
                sirius_formula: f.csi.formula.clone(),
                sirius_xlogp: f.csi.xlogp,
                mzlib_structure_ok: ok,
                mzlib_structure_ppm: Some(ppm).filter(|p| !p.is_nan()),
                untid: untid
                    .get(&(f.pol.as_str(), f.id))
                    .and_then(|u| u.as_ref())
                    .map(Untid::text),
            }
        })
        .collect()
}

/// A member of the registry: one identity, with the structure of the source that named it.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub campaign: String,
    pub name: String,
    pub group: String,
    pub category: String,
    pub best_level: i32,
    pub n_features: usize,
    pub first_block: Option<String>,
    pub inchikey: Option<String>,
    pub smiles: Option<String>,
    pub ik_basis: String,
    pub name_sources: String,
    /// The name to search for an identity without structure.
    pub lookup_name: Option<String>,
    pub note: String,
}

fn not_a_name() -> &'static Pattern {
    static P: std::sync::OnceLock<Pattern> = std::sync::OnceLock::new();
    P.get_or_init(|| Pattern::new(r"^(unidentified|oprea|zinc|ncgc|cid[\s:]|compound\b)"))
}

/// The name to search for an identity without structure: the curated name without its trailing qualifier (`Sulfamethoxazole (antibiotic)` →
/// `Sulfamethoxazole`); `None` for placeholders and library codes.
pub fn lookup_name(identity: &str) -> Option<String> {
    if not_a_name().search(&identity.trim().to_lowercase()) {
        return None;
    }
    let mut s = identity;
    let t = identity.trim_end();
    if t.ends_with(')') {
        if let Some(o) = t.rfind('(') {
            if !t[o + 1..t.len() - 1].contains(['(', ')']) {
                s = t[..o].trim_end();
            }
        }
    }
    Some(s.trim().to_string()).filter(|n| !n.is_empty())
}

fn joined<'a>(it: impl Iterator<Item = &'a str>) -> String {
    it.collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("+")
}

/// Evidence aggregated to one member row per `(campaign, identity, first block)`; an identity whose naming source has no structure is one row without block.
pub fn identity_members(ev: &[FeatureEvidence]) -> Vec<Member> {
    let mut by: BTreeMap<(&str, &str), Vec<&FeatureEvidence>> = BTreeMap::new();
    for e in ev {
        by.entry((e.campaign.as_str(), e.identity.as_str()))
            .or_default()
            .push(e);
    }
    let mut out = Vec::new();
    for ((camp, ident), g) in by {
        let base = |h: &[&FeatureEvidence]| Member {
            campaign: camp.to_string(),
            name: ident.to_string(),
            group: g[0].group.clone(),
            category: g[0].category.clone(),
            best_level: h.iter().map(|e| e.level).min().unwrap_or(3),
            n_features: h.len(),
            first_block: None,
            inchikey: None,
            smiles: None,
            ik_basis: String::new(),
            name_sources: joined(g.iter().map(|e| e.name_source.as_str())),
            lookup_name: None,
            note: String::new(),
        };
        let mut blocks: BTreeMap<&str, Vec<&FeatureEvidence>> = BTreeMap::new();
        for e in g.iter().filter(|e| e.name_block.is_some()) {
            blocks
                .entry(e.name_block.as_deref().unwrap_or_default())
                .or_default()
                .push(e);
        }
        if blocks.is_empty() {
            let lb: BTreeSet<&str> = g.iter().filter_map(|e| e.loader_block.as_deref()).collect();
            let note = if lb.is_empty() {
                String::new()
            } else {
                format!(
                    "loader block {}",
                    lb.into_iter().collect::<Vec<_>>().join("/")
                )
            };
            out.push(Member {
                lookup_name: lookup_name(ident),
                note,
                ..base(&g)
            });
            continue;
        }
        for (blk, h) in blocks {
            out.push(Member {
                first_block: Some(blk.to_string()),
                inchikey: h
                    .iter()
                    .filter_map(|e| e.name_inchikey.as_deref())
                    .find(|k| k.chars().count() == 27)
                    .map(str::to_string),
                smiles: h.iter().find_map(|e| e.name_smiles.clone()),
                ik_basis: joined(h.iter().map(|e| e.name_source.as_str())),
                ..base(&h)
            });
        }
    }
    out
}
