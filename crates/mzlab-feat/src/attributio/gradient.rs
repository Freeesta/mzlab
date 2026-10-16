// Ported from Attributio gradient.py.
//! Upstream against downstream at two sites, for chemical identities and for single features. Presence is the DETECTED rule of the campaign (only above `blank_fold`
//! times the blank), the areas are never gap-filled, and the fold change is the ratio of the areas of one representative feature: the one seen at both sites with the
//! largest signal, else the largest at each site. With `same_sorbent` (`[gradient] same_sorbent = true`) the ratio compares the same ion extracted with the same
//! sorbent; an identity at both sites with no such pair has status `both` and no fold change (counted as not comparable), and an enriched identity whose ion points
//! the other way with the other sorbent is flagged `sorbent_discordant` (reported, not reclassified).

use super::annotate::Feature;
use super::campaign::Campaign;
use super::detection::{site_area, Agg};
use super::error::{CampaignError, Result};
use super::frame::Cell;
use super::gradient_pick::*;
use super::sources::Sources;
use std::collections::HashMap;

/// `(polarity, site)` → `{feature id: largest DETECTED area over the sorbents}`.
type Areas = HashMap<(String, String), HashMap<i64, f64>>;
/// `(polarity, site, sorbent)` → `{feature id: DETECTED area}`.
pub(super) type AreasBySorbent = HashMap<(String, String, String), HashMap<i64, f64>>;

pub const POLARITIES: [&str; 2] = ["POS", "NEG"];

/// Columns of the table of identities, in order.
pub const GRADIENT_COLUMNS: [&str; 12] = [
    "canonical_name",
    "status",
    "representative_pol",
    "representative_id",
    "rt",
    "area_us",
    "area_ds",
    "log2FC_us_ds",
    "log2FC_ds_us",
    "abs_area",
    "fc_sorbent",
    "sorbent_discordant",
];

/// Columns of the table of features.
pub const CLOUD_COLUMNS: [&str; 7] = [
    "_pol",
    "_id",
    "area_us",
    "area_ds",
    "status",
    "log2FC_us_ds",
    "abs_area",
];

/// The counts of an identity gradient.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientSummary {
    pub fold_threshold: f64,
    pub n_both: usize,
    pub n_us_enriched: usize,
    pub n_ds_enriched: usize,
    pub n_within_fold: usize,
    pub n_us_only: usize,
    pub n_ds_only: usize,
    pub site_us: String,
    pub site_ds: String,
    pub n_not_comparable: usize,
    pub n_sorbent_discordant: usize,
}

/// One identity asked for: its name and, when the caller knows it, whether it is present at each site (`in_<site>` columns).
#[derive(Clone, Debug)]
pub struct GradIdentity {
    pub name: String,
    pub present: Option<(bool, bool)>,
}

/// The identity of one feature (`feature_identities`): the name its row counts under.
#[derive(Clone, Debug)]
pub struct FeatureIdentity {
    pub pol: String,
    pub id: i64,
    pub rt: Option<f64>,
    pub name: String,
}

/// One row of the gradient of an identity.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientRow {
    pub name: String,
    pub status: &'static str,
    pub pol: String,
    pub id: i64,
    pub rt: f64,
    pub area_us: f64,
    pub area_ds: f64,
    pub log2fc_us_ds: Option<f64>,
    pub log2fc_ds_us: Option<f64>,
    pub abs_area: f64,
    pub fc_sorbent: Option<String>,
    pub discordant: bool,
}

/// `same_sorbent_default`.
pub fn same_sorbent_default(c: &Campaign) -> bool {
    c.gradient_same_sorbent
}

/// `compute_gradient_areas`: `(pol, site)` → `{id: largest DETECTED area}`, zero where the feature is not DETECTED or is under the blank.
pub fn compute_gradient_areas(
    c: &Campaign,
    src: &Sources,
    sites: [&str; 2],
    pols: &[&str],
) -> Result<Areas> {
    let mut out = HashMap::new();
    for pol in pols {
        let Some(t) = src.pol(pol).map(|p| &p.table) else {
            continue;
        };
        for s in sites {
            out.insert(
                (pol.to_string(), s.to_string()),
                by_id(t, &site_area(c, t, s, pol, None, Agg::Max)?),
            );
        }
    }
    Ok(out)
}

/// `compute_gradient_areas_by_sorbent`: `(pol, site, sorbent)` → `{id: DETECTED area}`.
pub fn compute_gradient_areas_by_sorbent(
    c: &Campaign,
    src: &Sources,
    sites: [&str; 2],
    pols: &[&str],
) -> Result<AreasBySorbent> {
    let mut out = HashMap::new();
    for pol in pols {
        let Some(t) = src.pol(pol).map(|p| &p.table) else {
            continue;
        };
        for s in sites {
            for sb in &c.sorbents {
                let a = site_area(c, t, s, pol, Some(std::slice::from_ref(sb)), Agg::Max)?;
                out.insert((pol.to_string(), s.to_string(), sb.clone()), by_id(t, &a));
            }
        }
    }
    Ok(out)
}

fn by_id(t: &super::frame::Frame, area: &[f64]) -> HashMap<i64, f64> {
    let Some(k) = t.col("id") else {
        return HashMap::new();
    };
    (0..t.len())
        .filter_map(|r| t.int(r, k).map(|i| (i, area[r])))
        .collect()
}

/// `identity_gradient`: one row per identity of `identities` that has features in `fi`, and the counts. `same_sorbent` = the campaign's setting when `None`.
pub fn identity_gradient(
    c: &Campaign,
    src: &Sources,
    identities: &[GradIdentity],
    fi: &[FeatureIdentity],
    sites: [&str; 2],
    fold_threshold: f64,
    same_sorbent: Option<bool>,
) -> Result<(Vec<GradientRow>, GradientSummary)> {
    let same = same_sorbent.unwrap_or_else(|| same_sorbent_default(c));
    let areas = compute_gradient_areas(c, src, sites, &POLARITIES)?;
    let by_sb = if same {
        compute_gradient_areas_by_sorbent(c, src, sites, &POLARITIES)?
    } else {
        HashMap::new()
    };
    let area_of = |pol: &str, site: &str, id: i64| {
        areas
            .get(&(pol.to_string(), site.to_string()))
            .and_then(|m| m.get(&id))
            .copied()
            .unwrap_or(0.0)
    };
    let mut rows = Vec::new();
    for ident in identities {
        let a: Vec<Cand> = fi
            .iter()
            .filter(|f| f.name == ident.name)
            .map(|f| Cand {
                us: area_of(&f.pol, sites[0], f.id),
                ds: area_of(&f.pol, sites[1], f.id),
                pol: f.pol.clone(),
                id: f.id,
                rt: f.rt.unwrap_or(0.0),
            })
            .collect();
        if a.is_empty() {
            continue;
        }
        let (is_us, is_ds) = ident
            .present
            .unwrap_or((a.iter().any(|t| t.us > 0.0), a.iter().any(|t| t.ds > 0.0)));
        let (status, p) = match (is_us, is_ds) {
            (true, true) if same => (
                "both",
                pick_same_sorbent(&a, &by_sb, &c.sorbents, sites, fold_threshold),
            ),
            (true, true) => ("both", pick_plain(&a)),
            (true, false) => ("US_only", pick_one_site(&a, true)),
            (false, true) => ("DS_only", pick_one_site(&a, false)),
            (false, false) => ("none", Pick::none(&a[0])),
        };
        let abs_area = match status {
            "both" => p.us.max(p.ds),
            "US_only" => p.us,
            "DS_only" => p.ds,
            _ => 0.0,
        };
        rows.push(GradientRow {
            name: ident.name.clone(),
            status,
            pol: p.pol,
            id: p.id,
            rt: p.rt,
            area_us: p.us,
            area_ds: p.ds,
            log2fc_us_ds: p.l_ud,
            log2fc_ds_us: p.l_du,
            abs_area,
            fc_sorbent: p.sorbent,
            discordant: p.discordant,
        });
    }
    let summary = summarize(&rows, fold_threshold, sites)?;
    Ok((rows, summary))
}

fn summarize(rows: &[GradientRow], fold: f64, sites: [&str; 2]) -> Result<GradientSummary> {
    let thr = fold.log2();
    let both: Vec<&GradientRow> = rows.iter().filter(|r| r.status == "both").collect();
    let count = |f: &dyn Fn(f64) -> bool| {
        both.iter()
            .filter(|r| r.log2fc_us_ds.is_some_and(f))
            .count()
    };
    let s = GradientSummary {
        fold_threshold: fold,
        n_both: both.len(),
        n_us_enriched: count(&|v| v >= thr),
        n_ds_enriched: count(&|v| v <= -thr),
        n_within_fold: count(&|v| v.abs() < thr),
        n_not_comparable: both.iter().filter(|r| r.log2fc_us_ds.is_none()).count(),
        n_sorbent_discordant: both.iter().filter(|r| r.discordant).count(),
        n_us_only: rows.iter().filter(|r| r.status == "US_only").count(),
        n_ds_only: rows.iter().filter(|r| r.status == "DS_only").count(),
        site_us: sites[0].to_string(),
        site_ds: sites[1].to_string(),
    };
    if s.n_both != s.n_us_enriched + s.n_ds_enriched + s.n_within_fold + s.n_not_comparable {
        return Err(CampaignError::campaign("gradient split does not add up"));
    }
    Ok(s)
}

/// `feature_cloud_gradient`: the position of every feature of `feats` between the two sites (zero for what is not DETECTED).
pub fn feature_cloud_gradient(
    c: &Campaign,
    src: &Sources,
    feats: &[Feature],
    sites: [&str; 2],
) -> Result<Vec<Vec<Cell>>> {
    let areas = compute_gradient_areas(c, src, sites, &POLARITIES)?;
    let area_of = |pol: &str, site: &str, id: i64| {
        areas
            .get(&(pol.to_string(), site.to_string()))
            .and_then(|m| m.get(&id))
            .copied()
            .unwrap_or(0.0)
    };
    Ok(feats
        .iter()
        .map(|f| {
            let (u, d) = (
                area_of(&f.pol, sites[0], f.id),
                area_of(&f.pol, sites[1], f.id),
            );
            let (status, l, abs) = if u > 0.0 && d > 0.0 {
                ("both", Some((u / d).log2()), u.max(d))
            } else if u > 0.0 {
                ("US_only", None, u)
            } else if d > 0.0 {
                ("DS_only", None, d)
            } else {
                ("none", None, 0.0)
            };
            vec![
                Cell::Str(f.pol.clone()),
                Cell::Int(f.id),
                Cell::Num(u),
                Cell::Num(d),
                Cell::Str(status.to_string()),
                Cell::num(l),
                Cell::Num(abs),
            ]
        })
        .collect())
}

impl GradientRow {
    /// The cells in the order of `GRADIENT_COLUMNS`.
    pub fn cells(&self) -> Vec<Cell> {
        vec![
            Cell::Str(self.name.clone()),
            Cell::Str(self.status.to_string()),
            Cell::Str(self.pol.clone()),
            Cell::Int(self.id),
            Cell::Num(self.rt),
            Cell::Num(self.area_us),
            Cell::Num(self.area_ds),
            Cell::num(self.log2fc_us_ds),
            Cell::num(self.log2fc_ds_us),
            Cell::Num(self.abs_area),
            Cell::str(&self.fc_sorbent),
            Cell::Bool(self.discordant),
        ]
    }
}
