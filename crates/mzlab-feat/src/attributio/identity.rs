// Ported from Attributio identity.py.
//! From features to chemical identities, the unit every count uses. A feature is a signal (m/z, RT) of one alignment; one compound can give several (adducts,
//! in-source fragments, isomer peaks split by mzMine, POS and NEG). Counts are of IDENTITIES: one molecule is one row. Levels 1-3 collapse to one row per
//! curated identity (`identity_table`); Levels 4 and 0 stay feature counts (`feature_level_counts`) because they have no reliable identity to merge on.
//! The identity of a row is decided by `Curation::curate_identity`: the same rule for every count, list and figure.

use super::annotate::{Curation, Feature};
use super::curation::CEC_CATEGORIES;
use std::collections::{BTreeMap, BTreeSet};

/// The groups that the counts leave out by default: library codes and internal standards or carry-over.
pub const DROP_GROUPS: [&str; 2] = ["ambiguous", "lab_artifact"];

/// First 14 characters of an InChIKey (or InChIKey2D / InChIKey-Planar): the connectivity block, the same whatever the stereochemistry and the protonation.
/// `None` for anything shorter than 14 characters.
pub fn inchikey_skeleton(key: &str) -> Option<String> {
    let k = key.trim().to_uppercase();
    (k.chars().count() >= 14).then(|| k.chars().take(14).collect())
}

/// A Level 1-3 feature with its curated identity and group.
#[derive(Clone, Debug)]
pub struct Curated<'a> {
    pub feature: &'a Feature,
    pub identity: String,
    pub identity_category: String,
    pub group: String,
}

impl Curated<'_> {
    fn cec_label(&self) -> Option<&str> {
        self.feature.cec.as_ref().map(|c| c.0.as_str())
    }
    fn cec_category(&self) -> Option<&str> {
        self.feature.cec.as_ref().map(|c| c.1.as_str())
    }
}

/// The curated identity of each feature of `levels`: the curated contaminant, or the override, or the name cleaned of adduct and collision energy; replaced by
/// `identity_override` where the retention-time rule made the feature another compound (`<name> isomer, RT x.x min`). Features without an identity are left
/// out. The group is `contaminant` for a curated contaminant, else the category; the chemist's `group_override` replaces both.
pub fn curated<'a>(feats: &'a [Feature], levels: &[i32], cur: &dyn Curation) -> Vec<Curated<'a>> {
    let mut out = Vec::new();
    for f in feats.iter().filter(|f| levels.contains(&f.level)) {
        let disp = f.disp_name.as_deref().map(|n| cur.clean_library_name(n));
        let (mut identity, mut cat) =
            cur.curate_identity(f.cec.as_ref(), disp.as_deref(), f.csi.formula.as_deref());
        if let Some(o) = f
            .rt_info
            .identity_override
            .as_deref()
            .filter(|o| !o.trim().is_empty())
        {
            identity = o.to_string();
        }
        if identity.trim().is_empty() {
            continue;
        }
        let mut group = if f.cec.is_some() || CEC_CATEGORIES.contains(&cat.as_str()) {
            "contaminant".to_string()
        } else {
            cat.clone()
        };
        if let Some(g) = f
            .decision
            .group_override
            .as_deref()
            .filter(|g| !g.trim().is_empty())
        {
            group = g.to_string();
            cat = g.to_string();
        }
        out.push(Curated {
            feature: f,
            identity,
            identity_category: cat,
            group,
        });
    }
    out
}

/// One identity of Levels 1-3.
#[derive(Clone, Debug, PartialEq)]
pub struct IdentityRow {
    pub identity: String,
    /// 1 > 2 > 3 among its features.
    pub best_level: i32,
    /// `a` or `b` for a best level 2 (`b` only when every Level 2 feature of the identity is a 2b decision); `b` for a best level 3 whose features are all 2b
    /// counted as Level 3; empty otherwise.
    pub best_level_sub: String,
    pub is_cec: bool,
    pub identity_category: String,
    pub group: String,
    pub cec_label: Option<String>,
    pub cec_category: Option<String>,
    /// `(site, present in at least one of its features)`.
    pub in_site: Vec<(String, bool)>,
}

fn present(f: &Feature, site: &str) -> bool {
    f.in_site(site)
}

/// `{identity: sub-level}` rule of `best_level_sub`.
fn best_sub(rows: &[&Curated]) -> String {
    let lv = rows.iter().map(|r| r.feature.level).min().unwrap_or(3);
    let subs: BTreeSet<&str> = rows
        .iter()
        .filter(|r| r.feature.level == lv)
        .map(|r| r.feature.level_sub.as_str())
        .collect();
    match lv {
        2 => if subs.contains("a") { "a" } else { "b" }.to_string(),
        3 if subs.len() == 1 && subs.contains("b") => "b".to_string(),
        _ => String::new(),
    }
}

fn first_some<'a>(
    rows: &[&'a Curated],
    pick: impl Fn(&'a Curated) -> Option<&'a str>,
) -> Option<String> {
    rows.iter().find_map(|r| pick(r)).map(str::to_string)
}

fn keep_group(group: &str, drop_groups: &[&str]) -> bool {
    !drop_groups.contains(&group)
}

/// Levels 1-3 collapsed to one row per curated identity, sorted by identity. `drop_groups` removes library codes (`ambiguous`) and internal standards or
/// carry-over (`lab_artifact`) from the count; pass `&[]` to keep all.
pub fn identity_table(
    feats: &[Feature],
    sites: &[String],
    cur: &dyn Curation,
    drop_groups: &[&str],
) -> Vec<IdentityRow> {
    let d = curated(feats, &[1, 2, 3], cur);
    let mut by: BTreeMap<&str, Vec<&Curated>> = BTreeMap::new();
    for r in &d {
        by.entry(r.identity.as_str()).or_default().push(r);
    }
    by.into_iter()
        .map(|(identity, rows)| IdentityRow {
            identity: identity.to_string(),
            best_level: rows.iter().map(|r| r.feature.level).min().unwrap_or(3),
            best_level_sub: best_sub(&rows),
            is_cec: rows.iter().any(|r| r.feature.cec.is_some()),
            identity_category: rows[0].identity_category.clone(),
            group: rows[0].group.clone(),
            cec_label: first_some(&rows, |r| r.cec_label()),
            cec_category: first_some(&rows, |r| r.cec_category()),
            in_site: sites
                .iter()
                .map(|s| (s.clone(), rows.iter().any(|r| present(r.feature, s))))
                .collect(),
        })
        .filter(|r| keep_group(&r.group, drop_groups))
        .collect()
}

/// An identity of the structural deduplication: the InChIKey connectivity block when a feature has one, the name otherwise.
#[derive(Clone, Debug, PartialEq)]
pub struct StructureRow {
    pub identity: String,
    pub best_level: i32,
    pub is_cec: bool,
    pub identity_category: String,
    pub group: String,
    pub cec_label: Option<String>,
    pub cec_category: Option<String>,
    pub dedup_key: String,
    /// `inchikey_skeleton` or `name_fallback`.
    pub key_type: String,
    /// The names that the name method keeps apart (more than one: the two methods disagree).
    pub names_merged: Vec<String>,
    pub in_site: Vec<(String, bool)>,
}

/// `identity_table` with the InChIKey connectivity block as the key. The chemical review of this deduplication is not done (it would merge a drug with its
/// deuterated internal standard): the rows are for comparison with the name method, never for published numbers.
pub fn identity_table_by_structure(
    feats: &[Feature],
    sites: &[String],
    cur: &dyn Curation,
    drop_groups: &[&str],
) -> Vec<StructureRow> {
    let d = curated(feats, &[1, 2, 3], cur);
    let key_of = |r: &Curated| match r
        .feature
        .inchikey_skeleton
        .as_deref()
        .filter(|k| !k.trim().is_empty())
    {
        Some(k) => (k.to_string(), "inchikey_skeleton"),
        None => (format!("name:{}", r.identity), "name_fallback"),
    };
    let mut by: BTreeMap<String, Vec<&Curated>> = BTreeMap::new();
    for r in &d {
        by.entry(key_of(r).0).or_default().push(r);
    }
    by.into_iter()
        .map(|(key, rows)| {
            let best = rows.iter().map(|r| r.feature.level).min().unwrap_or(3);
            let rep = rows
                .iter()
                .find(|r| r.feature.level == best)
                .map_or("", |r| r.identity.as_str());
            let names: BTreeSet<&str> = rows.iter().map(|r| r.identity.as_str()).collect();
            StructureRow {
                identity: rep.to_string(),
                best_level: best,
                is_cec: rows.iter().any(|r| r.feature.cec.is_some()),
                identity_category: rows[0].identity_category.clone(),
                group: rows[0].group.clone(),
                cec_label: first_some(&rows, |r| r.cec_label()),
                cec_category: first_some(&rows, |r| r.cec_category()),
                key_type: key_of(rows[0]).1.to_string(),
                dedup_key: key,
                names_merged: names.into_iter().map(str::to_string).collect(),
                in_site: sites
                    .iter()
                    .map(|s| (s.clone(), rows.iter().any(|r| present(r.feature, s))))
                    .collect(),
            }
        })
        .filter(|r| keep_group(&r.group, drop_groups))
        .collect()
}

/// The Level 1-3 numbers with Level 2 split into 2a and 2b, for frozen numbers and the QC: identities (`identity_table`) and features.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LevelCounts {
    pub identities: usize,
    pub identities_l1: usize,
    /// 2a + 2b.
    pub identities_l2: usize,
    pub identities_l2a: usize,
    pub identities_l2b: usize,
    pub identities_l3: usize,
    pub identities_l2b_as_l3: usize,
    pub features_l1: usize,
    pub features_l2: usize,
    pub features_l2a: usize,
    pub features_l2b: usize,
    pub features_l3: usize,
    pub features_l2b_as_l3: usize,
}

impl LevelCounts {
    /// `(key, value)` with the keys of the reference tool (`identities_L1`, `features_L2b`, ...), in alphabetical order.
    pub fn pairs(&self) -> Vec<(&'static str, usize)> {
        vec![
            ("features_L1", self.features_l1),
            ("features_L2", self.features_l2),
            ("features_L2a", self.features_l2a),
            ("features_L2b", self.features_l2b),
            ("features_L2b_as_L3", self.features_l2b_as_l3),
            ("features_L3", self.features_l3),
            ("identities", self.identities),
            ("identities_L1", self.identities_l1),
            ("identities_L2", self.identities_l2),
            ("identities_L2a", self.identities_l2a),
            ("identities_L2b", self.identities_l2b),
            ("identities_L2b_as_L3", self.identities_l2b_as_l3),
            ("identities_L3", self.identities_l3),
        ]
    }
}

pub fn level_counts(
    feats: &[Feature],
    sites: &[String],
    cur: &dyn Curation,
    drop_groups: &[&str],
) -> LevelCounts {
    let it = identity_table(feats, sites, cur, drop_groups);
    let n = |pred: &dyn Fn(&IdentityRow) -> bool| it.iter().filter(|r| pred(r)).count();
    let nf = |lv: i32, sub: Option<bool>| {
        feats
            .iter()
            .filter(|f| f.level == lv && sub.is_none_or(|b| (f.level_sub == "b") == b))
            .count()
    };
    LevelCounts {
        identities: it.len(),
        identities_l1: n(&|r| r.best_level == 1),
        identities_l2: n(&|r| r.best_level == 2),
        identities_l2a: n(&|r| r.best_level == 2 && r.best_level_sub != "b"),
        identities_l2b: n(&|r| r.best_level == 2 && r.best_level_sub == "b"),
        identities_l3: n(&|r| r.best_level == 3),
        identities_l2b_as_l3: n(&|r| r.best_level == 3 && r.best_level_sub == "b"),
        features_l1: nf(1, None),
        features_l2: nf(2, None),
        features_l2a: nf(2, Some(false)),
        features_l2b: nf(2, Some(true)),
        features_l3: nf(3, None),
        features_l2b_as_l3: nf(3, Some(true)),
    }
}

/// `{site: {level: number of features}}` for the levels without a reliable identity (default 4 and 0).
pub fn feature_level_counts(
    feats: &[Feature],
    sites: &[String],
    levels: &[i32],
) -> Vec<(String, Vec<(i32, usize)>)> {
    sites
        .iter()
        .map(|s| {
            let per = levels
                .iter()
                .map(|&lv| {
                    (
                        lv,
                        feats
                            .iter()
                            .filter(|f| f.level == lv && present(f, s))
                            .count(),
                    )
                })
                .collect();
            (s.clone(), per)
        })
        .collect()
}

/// One identity of Levels 1-3 with its evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryRow {
    pub identity: String,
    pub group: String,
    /// The contaminant category (else the identity category) of a contaminant; the group of the others.
    pub category: String,
    pub n_features: usize,
    pub best_level: i32,
    /// Best spectral or CSI score (of each feature: the spectral similarity, else the GNPS2 score, else the CSI:FingerID confidence).
    pub best_q: Option<f64>,
    pub evidence: String,
    pub n_sources: usize,
    pub pol: String,
    pub sites: String,
    pub npc_pathway: String,
}

fn feature_q(f: &Feature) -> Option<f64> {
    f.spec.sim.or(f.gnps.score).or(f.csi.conf)
}

/// One row per identity of `levels` with its evidence, by the same identity and group rule as `identity_table`, sorted by group, category, best level and
/// best score (highest first, none last).
pub fn load_confident_inventory(
    feats: &[Feature],
    sites: &[String],
    cur: &dyn Curation,
    levels: &[i32],
) -> Vec<InventoryRow> {
    let d = curated(feats, levels, cur);
    let mut order: Vec<&str> = Vec::new();
    let mut by: BTreeMap<&str, Vec<&Curated>> = BTreeMap::new();
    for r in &d {
        if !by.contains_key(r.identity.as_str()) {
            order.push(r.identity.as_str());
        }
        by.entry(r.identity.as_str()).or_default().push(r);
    }
    let mut rows: Vec<InventoryRow> = order
        .into_iter()
        .map(|identity| {
            let g = &by[identity];
            let srcs: BTreeSet<&str> = g
                .iter()
                .flat_map(|r| r.feature.sources.iter().copied())
                .collect();
            let pols: BTreeSet<&str> = g.iter().map(|r| r.feature.pol.as_str()).collect();
            let site_sets: BTreeSet<String> = g
                .iter()
                .map(|r| {
                    sites
                        .iter()
                        .filter(|s| present(r.feature, s))
                        .map(|s| s.chars().take(2).collect::<String>())
                        .collect::<Vec<_>>()
                        .join("+")
                })
                .collect();
            let group = g[0].group.clone();
            let category = if group == "contaminant" {
                first_some(g, |r| r.cec_category())
                    .unwrap_or_else(|| g[0].identity_category.clone())
            } else {
                group.clone()
            };
            InventoryRow {
                identity: identity.to_string(),
                group,
                category,
                n_features: g.len(),
                best_level: g.iter().map(|r| r.feature.level).min().unwrap_or(3),
                best_q: g
                    .iter()
                    .filter_map(|r| feature_q(r.feature))
                    .reduce(f64::max),
                evidence: srcs.iter().copied().collect::<Vec<_>>().join("+"),
                n_sources: srcs.len(),
                pol: pols.into_iter().collect::<Vec<_>>().join("/"),
                sites: site_sets.into_iter().collect::<Vec<_>>().join("|"),
                npc_pathway: g
                    .iter()
                    .find_map(|r| r.feature.classes[0].clone())
                    .unwrap_or_default(),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        let q = |r: &InventoryRow| r.best_q.map_or(f64::NEG_INFINITY, |v| v);
        (&a.group, &a.category, a.best_level)
            .cmp(&(&b.group, &b.category, b.best_level))
            .then_with(|| match (a.best_q, b.best_q) {
                (None, None) => std::cmp::Ordering::Equal,
                (None, _) => std::cmp::Ordering::Greater,
                (_, None) => std::cmp::Ordering::Less,
                _ => q(b).total_cmp(&q(a)),
            })
    });
    rows
}
