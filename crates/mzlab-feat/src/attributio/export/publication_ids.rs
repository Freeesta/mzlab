// Ported from Attributio diagnostics/publication_table.py, tables.py.
//! The identities of the paper and their features: `identity_table_enriched` and `dedup_by_canonical` (one row per canonical name; synonyms, brands and library
//! codes of the same compound merge, isomers do not), and the Level 1-3 features with their curated identity, the row they belong to and the largest area at
//! each site (`canonical_identities`, `feature_frame`).

use super::ctx::{FRow, Loaded};
use crate::attributio::annotate::{AnnotationSettings, Curation};
use crate::attributio::campaign::{Campaign, POLARITIES};
use crate::attributio::detection::{has_site, site_area, Agg};
use crate::attributio::error::Result;
use crate::attributio::identity::{curated, identity_table, DROP_GROUPS};
use crate::attributio::io::Io;
use crate::attributio::members::{feature_evidence, EvidenceExtras, FeatureEvidence};
use std::collections::{BTreeMap, HashMap};

/// An identity of `identity_table` (library codes and internal standards left out) with its formula and canonical name.
#[derive(Clone, Debug)]
pub struct Gt {
    pub identity: String,
    pub best_level: i32,
    pub best_sub: String,
    pub group: String,
    pub cec_category: Option<String>,
    pub in_site: Vec<bool>,
    pub formula: String,
    pub nome: String,
}

/// A row of the paper: all the identities that share a canonical name.
#[derive(Clone, Debug)]
pub struct Dd {
    pub nome: String,
    pub best_level: i32,
    pub best_sub: String,
    pub group: String,
    pub is_anthro: bool,
    pub cec_category: String,
    pub formula: String,
    pub in_site: Vec<bool>,
}

fn is_text(s: &Option<String>) -> Option<&str> {
    s.as_deref().filter(|v| !v.trim().is_empty())
}

/// The most frequent formula of each curated identity (the smallest text among equals), as `Series.mode().iat[0]`.
fn formula_by_identity(l: &Loaded) -> HashMap<String, String> {
    let mut count: HashMap<String, BTreeMap<String, usize>> = HashMap::new();
    for d in curated(l.features(), &[1, 2, 3], &l.cur) {
        if let Some(f) = &d.feature.csi.formula {
            *count
                .entry(d.identity)
                .or_default()
                .entry(f.clone())
                .or_default() += 1;
        }
    }
    count
        .into_iter()
        .map(|(k, m)| {
            let best = m.values().copied().max().unwrap_or(0);
            let f = m.into_iter().find(|(_, n)| *n == best).map(|(f, _)| f);
            (k, f.unwrap_or_default())
        })
        .collect()
}

/// `identity_table_enriched`: the identities of Levels 1-3 with `formula` and `nome_comune`.
pub fn enriched_identities(l: &Loaded) -> Vec<Gt> {
    let fmap = formula_by_identity(l);
    identity_table(l.features(), &l.sites, &l.cur, &DROP_GROUPS)
        .into_iter()
        .map(|r| {
            let base = fmap.get(&r.identity).cloned().unwrap_or_default();
            let formula = l
                .cur
                .formula_override(&r.identity.trim().to_lowercase())
                .unwrap_or(base);
            let nome = l.cur.canonical_name(&r.identity, Some(&formula));
            Gt {
                best_sub: r.best_level_sub,
                cec_category: r.cec_category,
                in_site: r.in_site.iter().map(|(_, v)| *v).collect(),
                identity: r.identity,
                best_level: r.best_level,
                group: r.group,
                formula,
                nome,
            }
        })
        .collect()
}

/// `dedup_by_canonical`: one row per canonical name, in the order of the names.
pub fn dedup_by_canonical(gt: &[Gt], n_sites: usize) -> Vec<Dd> {
    let mut by: BTreeMap<&str, Vec<&Gt>> = BTreeMap::new();
    for g in gt {
        by.entry(g.nome.as_str()).or_default().push(g);
    }
    by.into_iter()
        .map(|(nome, g)| {
            let contaminant = g.iter().any(|x| x.group == "contaminant");
            let best = g.iter().map(|x| x.best_level).min().unwrap_or(3);
            let subs: Vec<&str> = g
                .iter()
                .filter(|x| x.best_level == best)
                .map(|x| x.best_sub.as_str())
                .collect();
            let some: Vec<&str> = subs.iter().copied().filter(|s| !s.is_empty()).collect();
            let sub = if some.is_empty() {
                ""
            } else if some.iter().all(|s| *s == "b") && subs.iter().all(|s| *s == "b") {
                "b"
            } else {
                "a"
            };
            Dd {
                nome: nome.to_string(),
                best_level: best,
                best_sub: sub.to_string(),
                group: if contaminant {
                    "contaminant".into()
                } else {
                    g[0].group.clone()
                },
                is_anthro: contaminant || g.iter().any(|x| x.group.starts_with("illicit")),
                cec_category: g
                    .iter()
                    .find_map(|x| is_text(&x.cec_category))
                    .unwrap_or("")
                    .to_string(),
                formula: g
                    .iter()
                    .find(|x| !x.formula.trim().is_empty())
                    .map(|x| x.formula.clone())
                    .unwrap_or_default(),
                in_site: (0..n_sites)
                    .map(|k| g.iter().any(|x| x.in_site[k]))
                    .collect(),
            }
        })
        .collect()
}

/// `canonical_identities`: `(gt, dd)`.
pub fn canonical_identities(l: &Loaded) -> (Vec<Gt>, Vec<Dd>) {
    let gt = enriched_identities(l);
    let dd = dedup_by_canonical(&gt, l.sites.len());
    (gt, dd)
}

/// A Level 1-3 feature with its evidence, its row and its largest area at each site.
pub struct PFeat<'a> {
    pub fr: &'a FRow<'a>,
    pub ev: FeatureEvidence,
    pub nome: String,
    /// `maxarea_<site>`: the larger of the sorbents, gap-filled areas count 0 (NaN where the feature is in no table).
    pub max_area: Vec<f64>,
}

impl PFeat<'_> {
    pub fn f(&self) -> &crate::attributio::annotate::Feature {
        self.fr.f
    }
}

/// `_max_area`: `(pol, id) -> area at each site`, the larger of the sorbents.
fn max_areas(c: &Campaign, l: &Loaded) -> Result<HashMap<(String, i64), Vec<f64>>> {
    let mut out = HashMap::new();
    for pol in POLARITIES {
        let Some(t) = l.src.pol(pol).map(|p| &p.table) else {
            continue;
        };
        let Some(kid) = t.col("id") else { continue };
        let mut per_site = Vec::new();
        for s in &l.sites {
            per_site.push(if has_site(c, t, s, pol) {
                site_area(c, t, s, pol, None, Agg::Max)?
            } else {
                vec![0.0; t.len()]
            });
        }
        for r in 0..t.len() {
            if let Some(id) = t.int(r, kid) {
                out.entry((pol.to_string(), id))
                    .or_insert_with(|| per_site.iter().map(|a| a[r]).collect());
            }
        }
    }
    Ok(out)
}

/// `feature_frame`: the Level 1-3 features in the order of the annotations, each with the canonical name of its row.
pub fn feature_frame<'a>(
    c: &Campaign,
    io: &Io,
    l: &Loaded,
    rows: &'a [FRow<'a>],
    gt: &[Gt],
) -> Result<Vec<PFeat<'a>>> {
    let tol = AnnotationSettings::for_campaign(c)?.gnps_mass_tol_ppm;
    let extras = EvidenceExtras::load(c, io, tol)?;
    let ev = feature_evidence(&c.id, l.features(), &l.src, &extras, &l.cur);
    let by_row: HashMap<(&str, i64), &FRow> = rows
        .iter()
        .map(|r| ((r.f.pol.as_str(), r.f.id), r))
        .collect();
    let areas = max_areas(c, l)?;
    let to_row: HashMap<&str, &str> = gt
        .iter()
        .map(|g| (g.identity.as_str(), g.nome.as_str()))
        .collect();
    let mut out = Vec::new();
    for e in ev {
        let Some(fr) = by_row.get(&(e.pol.as_str(), e.id)).copied() else {
            continue;
        };
        let nome = match to_row.get(e.identity.as_str()) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => l
                .cur
                .canonical_name(&e.identity, Some(fr.f.csi.formula.as_deref().unwrap_or(""))),
        };
        let max_area = areas
            .get(&(e.pol.clone(), e.id))
            .cloned()
            .unwrap_or_else(|| vec![f64::NAN; l.sites.len()]);
        out.push(PFeat {
            fr,
            ev: e,
            nome,
            max_area,
        });
    }
    Ok(out)
}
