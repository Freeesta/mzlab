// Ported from Attributio annotate.py.
//! The retention-time rule: one identity is one compound at one retention time. The Level 1-3 features of the campaign are grouped by identity (curated
//! identity, then canonical name), then by RT: sorted by RT, a gap larger than `rt_group_tol_min` starts a new group. The group with the best evidence is the
//! reference; the features of the others are named `<identity> isomer, RT x.x min`. When two or more groups are Level 2 the spectrum does not tell the
//! isomers apart and all of them go to Level 3 (`isomers not discriminated`); when the reference group is Level 1, a Level 2 of another group goes to Level 3
//! (`RT isomer`).

use super::curate::Curation;
use super::decide::{level_sub, mzlib_only};
use super::settings::AnnotationSettings;
use super::types::{Feature, RtInfo};
use crate::attributio::campaign::{Campaign, POLARITIES};
use crate::attributio::detection::{has_site, site_area, Agg};
use crate::attributio::error::Result;
use crate::attributio::sources::Sources;
use crate::sortc::SortC;
use std::collections::HashMap;

/// The outcome of the rule for one feature.
#[derive(Clone, Debug, PartialEq)]
pub struct RtRow {
    pub pol: String,
    pub id: i64,
    pub info: RtInfo,
    pub new_level: Option<i32>,
    pub new_basis: Option<&'static str>,
}

/// Largest DETECTED area of each feature of the scope over the sites and sorbents of the campaign.
fn max_areas(c: &Campaign, src: &Sources) -> Result<HashMap<(String, i64), f64>> {
    let mut out = HashMap::new();
    for pol in POLARITIES {
        let Some(p) = src.pol(pol) else {
            continue;
        };
        let mut best = vec![0.0f64; p.table.len()];
        for site in c.site_keys() {
            if has_site(c, &p.table, site, pol) {
                let a = site_area(c, &p.table, site, pol, None, Agg::Max)?;
                for (b, v) in best.iter_mut().zip(a) {
                    *b = b.max(v);
                }
            }
        }
        if let Some(k) = p.table.col("id") {
            for (r, b) in best.iter().enumerate() {
                if let Some(id) = p.table.int(r, k) {
                    out.insert((pol.to_string(), id), *b);
                }
            }
        }
    }
    Ok(out)
}

/// The formula of each identity: its most frequent CSI:FingerID formula (the first in sorted order on a tie), then the manual overrides.
fn identity_formulas(
    rows: &[(String, Option<String>)],
    cur: &dyn Curation,
) -> HashMap<String, String> {
    let mut by: Vec<(&str, Vec<&str>)> = Vec::new();
    for (ident, f) in rows {
        let slot = match by.iter().position(|(k, _)| k == ident) {
            Some(n) => n,
            None => {
                by.push((ident.as_str(), Vec::new()));
                by.len() - 1
            }
        };
        if let Some(f) = f {
            by[slot].1.push(f.as_str());
        }
    }
    by.into_iter()
        .map(|(k, mut v)| {
            v.sortc();
            let mut best = ("", 0usize);
            let mut n = 0;
            while n < v.len() {
                let m = v[n..].iter().take_while(|x| **x == v[n]).count();
                if m > best.1 {
                    best = (v[n], m);
                }
                n += m;
            }
            let own = best.0.to_string();
            let f = cur
                .formula_override(&k.trim().to_lowercase())
                .unwrap_or(own);
            (k.to_string(), f)
        })
        .collect()
}

struct Item<'a> {
    f: &'a Feature,
    ident: String,
    n_sites: usize,
    area: f64,
}

fn rt_order(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    let key = |v: Option<f64>| (v.is_none_or(f64::is_nan), v.unwrap_or(0.0));
    let (x, y) = (key(a), key(b));
    x.0.cmp(&y.0).then(x.1.total_cmp(&y.1))
}

struct Cluster<'a> {
    rows: Vec<&'a Item<'a>>,
    level: i32,
    src: usize,
    sites: usize,
    area: f64,
    rt: f64,
}

fn evidence_key(i: &Item) -> (i32, i64, i64, f64) {
    (
        i.f.level,
        -(i.f.sources.len() as i64),
        -(i.n_sites as i64),
        -i.area,
    )
}

fn make_cluster<'a>(rows: Vec<&'a Item<'a>>, site_keys: &[&str]) -> Cluster<'a> {
    let mut best = rows[0];
    for r in &rows[1..] {
        let (a, b) = (evidence_key(r), evidence_key(best));
        if a.0
            .cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
            .then(a.3.total_cmp(&b.3))
            .is_lt()
        {
            best = r;
        }
    }
    Cluster {
        level: rows.iter().map(|r| r.f.level).min().unwrap_or(0),
        src: rows.iter().map(|r| r.f.sources.len()).max().unwrap_or(0),
        sites: site_keys
            .iter()
            .filter(|s| rows.iter().any(|r| r.f.in_site(s)))
            .count(),
        area: rows.iter().map(|r| r.area).fold(f64::MIN, f64::max),
        rt: best.f.rt.unwrap_or(f64::NAN),
        rows,
    }
}

fn split_clusters<'a>(mut g: Vec<&'a Item<'a>>, tol: f64, site_keys: &[&str]) -> Vec<Cluster<'a>> {
    g.sortc_by(|a, b| {
        rt_order(a.f.rt, b.f.rt)
            .then_with(|| a.f.pol.cmp(&b.f.pol))
            .then(a.f.id.cmp(&b.f.id))
    });
    let mut groups: Vec<Vec<&Item>> = Vec::new();
    let mut prev: Option<f64> = None;
    for it in g {
        let new = match (prev, it.f.rt) {
            (Some(p), Some(r)) => r - p > tol,
            _ => false,
        };
        if new || groups.is_empty() {
            groups.push(Vec::new());
        }
        groups.last_mut().expect("group").push(it);
        prev = it.f.rt;
    }
    groups
        .into_iter()
        .map(|rows| make_cluster(rows, site_keys))
        .collect()
}

fn rows_of_group(key: &str, clusters: &[Cluster]) -> Vec<RtRow> {
    let mut order: Vec<usize> = (0..clusters.len()).collect();
    order.sortc_by(|&a, &b| {
        let (x, y) = (&clusters[a], &clusters[b]);
        x.level
            .cmp(&y.level)
            .then(y.src.cmp(&x.src))
            .then(y.sites.cmp(&x.sites))
            .then(y.area.total_cmp(&x.area))
    });
    let reference = order[0];
    let n_l2 = clusters.iter().filter(|c| c.level == 2).count();
    let l1_ref = clusters[reference].level == 1;
    let not_discriminated = !l1_ref && n_l2 >= 2;
    let mut rows = Vec::new();
    for (rank, &i) in order.iter().enumerate() {
        let c = &clusters[i];
        let (lo, hi) = c
            .rows
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), r| {
                let t = r.f.rt.unwrap_or(f64::NAN);
                (l.min(t), h.max(t))
            });
        for r in &c.rows {
            let over = (i != reference).then(|| format!("{} isomer, RT {:.1} min", r.ident, c.rt));
            let (new_level, new_basis) = if r.f.level == 2 && not_discriminated {
                (Some(3), Some("isomers not discriminated"))
            } else if r.f.level == 2 && i != reference && l1_ref {
                (Some(3), Some("RT isomer"))
            } else {
                (None, None)
            };
            rows.push(RtRow {
                pol: r.f.pol.clone(),
                id: r.f.id,
                info: RtInfo {
                    identity_override: over,
                    rt_key: Some(key.to_string()),
                    rt_group: Some(rank),
                    rt_groups: Some(clusters.len()),
                    rt_ref: Some(c.rt),
                    rt_min: Some(lo),
                    rt_max: Some(hi),
                    rt_reference_group: Some(i == reference),
                },
                new_level,
                new_basis,
            });
        }
    }
    rows
}

/// `rt_coherence`: one row per Level 1-3 feature of the scope that has an identity.
pub fn rt_coherence(
    c: &Campaign,
    src: &Sources,
    scope: &[Feature],
    s: &AnnotationSettings,
    cur: &dyn Curation,
) -> Result<Vec<RtRow>> {
    let mut idents: Vec<(usize, String)> = Vec::new();
    for (n, f) in scope.iter().enumerate() {
        if !(1..=3).contains(&f.level) {
            continue;
        }
        let cleaned = f.disp_name.as_deref().map(|d| cur.clean_library_name(d));
        let (ident, _) =
            cur.curate_identity(f.cec.as_ref(), cleaned.as_deref(), f.csi.formula.as_deref());
        if !ident.trim().is_empty() && !ident.starts_with("Unidentified") {
            idents.push((n, ident));
        }
    }
    if idents.is_empty() {
        return Ok(Vec::new());
    }
    let formulas: Vec<(String, Option<String>)> = idents
        .iter()
        .map(|(n, i)| (i.clone(), scope[*n].csi.formula.clone()))
        .collect();
    let fmap = identity_formulas(&formulas, cur);
    let areas = max_areas(c, src)?;
    let items: Vec<Item> = idents
        .into_iter()
        .map(|(n, ident)| {
            let f = &scope[n];
            Item {
                f,
                n_sites: f.in_site.iter().filter(|(_, v)| *v).count(),
                area: areas.get(&(f.pol.clone(), f.id)).copied().unwrap_or(0.0),
                ident,
            }
        })
        .collect();
    let mut groups: Vec<(String, Vec<&Item>)> = Vec::new();
    for it in &items {
        let formula = fmap
            .get(&it.ident)
            .filter(|f| !f.is_empty())
            .map(String::as_str);
        let key = cur.canonical_name(&it.ident, formula);
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => v.push(it),
            None => groups.push((key, vec![it])),
        }
    }
    let site_keys = c.site_keys();
    let mut out = Vec::new();
    for (key, g) in groups {
        let clusters = split_clusters(g, s.rt_group_tol_min, &site_keys);
        out.extend(rows_of_group(&key, &clusters));
    }
    Ok(out)
}

/// Merges the outcome of the rule into the features and applies its level changes. The rule wins over a 2b decision: the isomers are not told apart.
pub fn apply_rt_coherence(
    feats: &mut [Feature],
    rows: &[RtRow],
    s: &AnnotationSettings,
    warnings: &mut Vec<String>,
) {
    let by: HashMap<(&str, i64), &RtRow> =
        rows.iter().map(|r| ((r.pol.as_str(), r.id), r)).collect();
    for f in feats.iter_mut() {
        f.rt_info = RtInfo::default();
        if let Some(r) = by.get(&(f.pol.as_str(), f.id)) {
            f.rt_info = r.info.clone();
            if let (Some(level), Some(basis)) = (r.new_level, r.new_basis) {
                f.level = level;
                f.level_basis = basis.into();
                f.level_label = s.label(basis);
                if f.decision.decision_2b {
                    warnings.push(format!(
                        "Level 2b decision on {} {} overridden by the retention-time rule ('{basis}'): the spectrum does not tell the isomers apart",
                        f.pol, f.id
                    ));
                }
                f.decision.decision_2b = false;
                f.decision.auto_2b = false;
            }
        }
        f.level_sub = level_sub(f).into();
        f.level_flag = mzlib_only(f);
    }
}
