// Ported from Attributio ecotox/hazard.py, ecotox/ecosar.py (taxon_values).
//! One aquatic hazard record (acute + chronic) per chemical identity. Tiers, first one with a category wins, acute and chronic separately: T1 regulatory (CLP
//! harmonised, curated), T2 measured (ECOTOX snapshot and curated values), T3 notified (PubChem GHS H-statements), T4 predicted (curated judgement, then ECOSAR 2.2),
//! T5 not assessable. Degradability is unknown for almost every compound, so chronic categories assume «not rapidly degradable» and every record says so.
//! A hazard class is a property of a structure, not a risk: no concentration, no PNEC. Everything is read from the snapshots the caller passes in.

use super::candidates::{ghs_codes, log_kow, Candidates, ACUTE_ORDER, CHRONIC_ORDER};
use super::snapshot::{EcosarEndpoint, EcotoxValue, RegistryRow};
use crate::attributio::names::{canonical_name, split_rt_isomer};
use crate::attributio::pattern::Pattern;
use std::collections::BTreeSet;

pub const NOT_ASSESSABLE: &str = "Not assessable";
pub const DEGRADABILITY: &str = "not rapidly degradable (assumed: no degradability data used)";
/// Tier labels in report order.
pub const TIER_LABEL: [(&str, &str); 5] = [
    ("T1", "regulatory (CLP harmonised)"),
    ("T2", "measured"),
    ("T3", "notified classification"),
    ("T4", "predicted"),
    ("T5", "not assessable"),
];

/// A curated decision (`ecotox_curated.csv`).
#[derive(Clone, Debug, Default)]
pub struct Curated {
    pub ik1: String,
    pub label: String,
    pub acute: String,
    pub chronic: String,
    pub basis: String,
    pub tier: String,
    pub note: String,
}

/// A structure given to an identity name by a decision (`ecotox_structures.csv`).
#[derive(Clone, Debug, Default)]
pub struct StructureOverride {
    pub pattern: String,
    pub compound: String,
    pub inchikey: String,
}

/// An identity member of the registry (`registry_members.csv`, member type identity).
#[derive(Clone, Debug, Default)]
pub struct IdentityMember {
    pub campaign: String,
    pub name: String,
    pub formula: String,
    pub first_block: String,
}

/// Everything a record is built from.
#[derive(Clone, Debug, Default)]
pub struct HazardData {
    pub registry: Vec<RegistryRow>,
    pub members: Vec<IdentityMember>,
    pub curated: Vec<Curated>,
    pub overrides: Vec<StructureOverride>,
    pub values: Vec<EcotoxValue>,
    pub ecosar: Vec<EcosarEndpoint>,
}

/// The hazard record of one identity.
#[derive(Clone, Debug, PartialEq)]
pub struct HazardRecord {
    pub identity: String,
    pub blocks: String,
    pub structure_basis: String,
    pub acute: String,
    pub acute_tier: String,
    pub acute_basis: String,
    pub chronic: String,
    pub chronic_tier: String,
    pub chronic_basis: String,
    pub chronic_route: String,
    pub degradability: String,
    pub ecosar_classes: String,
    pub log_kow: Option<f64>,
    pub caveat: String,
    pub cross_check: String,
}

/// Python's `{:.Ng}` for `n` significant digits.
pub(super) fn fmt_gn(v: f64, n: usize) -> String {
    if v == 0.0 || !v.is_finite() {
        return if v == 0.0 { "0".into() } else { format!("{v}") };
    }
    let sci = format!("{:.*e}", n - 1, v);
    let (mant, e) = sci.split_once('e').unwrap_or((&sci, "0"));
    let e: i32 = e.parse().unwrap_or(0);
    let trim = |s: &str| {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if !(-4..n as i32).contains(&e) {
        format!(
            "{}e{}{:02}",
            trim(mant),
            if e < 0 { '-' } else { '+' },
            e.abs()
        )
    } else {
        trim(&format!("{:.*}", (n as i32 - 1 - e).max(0) as usize, v))
    }
}

/// The name without its « isomer, RT x min» suffix, when it has one.
fn strip_isomer(name: &str) -> Option<&str> {
    split_rt_isomer(name).map(|(head, _)| head)
}

/// Lowercases a pattern except the letters after a backslash (`\D` stays `\D`), the way `re.I` reads a pattern against a lowercase name.
fn fold_pattern(p: &str) -> String {
    let (mut out, mut esc) = (String::new(), false);
    for c in p.chars() {
        if esc {
            out.push(c);
            esc = false;
        } else if c == '\\' {
            out.push(c);
            esc = true;
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// First InChIKey blocks of an identity name and how they were found.
pub fn resolve_blocks(d: &HazardData, name: &str, campaign: Option<&str>) -> (Vec<String>, String) {
    for o in &d.overrides {
        if !o.inchikey.is_empty()
            && Pattern::new(&fold_pattern(&o.pattern)).search(&name.to_lowercase())
        {
            return (
                vec![o.inchikey.chars().take(14).collect()],
                format!("curated structure ({})", o.compound),
            );
        }
    }
    let members: Vec<&IdentityMember> = d
        .members
        .iter()
        .filter(|m| campaign.is_none_or(|c| c.is_empty() || m.campaign == c))
        .collect();
    if members.is_empty() {
        return (vec![], String::new());
    }
    let canon: Vec<String> = members
        .iter()
        .map(|m| canonical_name(&m.name, Some(m.formula.as_str()).filter(|f| !f.is_empty())))
        .collect();
    let stripped = strip_isomer(name);
    let cands = [
        (name, "registry"),
        (
            stripped.unwrap_or(name),
            "registry, structure of the name (RT isomer)",
        ),
    ];
    for (cand, how) in cands {
        let blocks: BTreeSet<&str> = members
            .iter()
            .zip(&canon)
            .filter(|(m, c)| (m.name == cand || c.as_str() == cand) && !m.first_block.is_empty())
            .map(|(m, _)| m.first_block.as_str())
            .collect();
        if !blocks.is_empty() {
            return (
                blocks.into_iter().map(str::to_string).collect(),
                how.to_string(),
            );
        }
        if cand == stripped.unwrap_or(name) {
            break;
        }
    }
    (vec![], String::new())
}

fn curated_for<'a>(d: &'a HazardData, blocks: &[String], name: &str) -> Vec<&'a Curated> {
    let hit: Vec<&Curated> = d
        .curated
        .iter()
        .filter(|c| !c.ik1.is_empty() && c.ik1.split('|').any(|b| blocks.iter().any(|x| x == b)))
        .collect();
    if !hit.is_empty() {
        return hit;
    }
    let bare = strip_isomer(name).unwrap_or(name);
    d.curated
        .iter()
        .filter(|c| c.label == name || c.label == bare)
        .collect()
}

/// Caveats that depend on the identity, the level and what the registry holds.
fn caveats(
    d: &HazardData,
    rec: &HazardRecord,
    blocks: &[String],
    no_curated: bool,
    level: Option<i64>,
) -> String {
    let mut out: Vec<String> = Vec::new();
    if split_rt_isomer(&rec.identity).is_some() {
        out.push("RT isomer: hazard of the named structure, the peak may be another isomer".into());
    }
    if level == Some(3) {
        out.push("Level 3: hazard of the CSI:FingerID candidate structure".into());
    }
    let codes = if blocks.is_empty() {
        vec![]
    } else {
        ghs_codes(d, blocks)
    };
    if !codes.is_empty()
        && rec.acute == "Acute 1"
        && rec.acute_tier == "T4"
        && !codes.iter().any(|c| c == "H400")
    {
        out.push(format!(
            "predicted Acute 1 but the notified classification ({}) has no H400",
            codes.join(", ")
        ));
    }
    if blocks.is_empty() && no_curated {
        out.push("no structure in the registry".into());
    }
    out.join("; ")
}

/// The hazard record of one identity (`level` 3 adds the candidate-structure caveat).
pub fn record(
    d: &HazardData,
    name: &str,
    campaign: Option<&str>,
    level: Option<i64>,
) -> HazardRecord {
    let (blocks, how) = resolve_blocks(d, name, campaign);
    let kow = if blocks.is_empty() {
        None
    } else {
        log_kow(d, &blocks)
    };
    let cu = curated_for(d, &blocks, name);
    let mut cands = Candidates::default();
    cands.curated(&cu);
    if !blocks.is_empty() {
        cands.measured(d, &blocks, kow);
        cands.notified(d, &blocks);
        cands.predicted(d, &blocks);
    }
    let mut rec = HazardRecord {
        identity: name.into(),
        blocks: blocks.join("|"),
        structure_basis: how,
        acute: NOT_ASSESSABLE.into(),
        acute_tier: "T5".into(),
        acute_basis: String::new(),
        chronic: NOT_ASSESSABLE.into(),
        chronic_tier: "T5".into(),
        chronic_basis: String::new(),
        chronic_route: cands.chronic_route.clone(),
        degradability: DEGRADABILITY.into(),
        ecosar_classes: cands.ecosar_classes.clone(),
        log_kow: kow,
        caveat: String::new(),
        cross_check: String::new(),
    };
    for chronic in [false, true] {
        let Some(p) = cands.pick(chronic) else {
            continue;
        };
        let which = if chronic { "chronic" } else { "acute" };
        if chronic {
            (rec.chronic, rec.chronic_tier, rec.chronic_basis) = (p.category, p.tier, p.basis);
        } else {
            (rec.acute, rec.acute_tier, rec.acute_basis) = (p.category, p.tier, p.basis);
        }
        if !p.other.is_empty() {
            let sep = if rec.cross_check.is_empty() { "" } else { "; " };
            let list = p.other.into_iter().collect::<Vec<_>>().join(", ");
            rec.cross_check = format!("{}{sep}{which}: {list}", rec.cross_check);
        }
    }
    rec.caveat = caveats(d, &rec, &blocks, cu.is_empty(), level);
    rec
}

/// Counts for QC and frozen numbers: per category, per tier, and the identities without any assessment (acute AND chronic not assessable). `anthropogenic` (one
/// flag per record) adds «anthropogenic not assessable».
pub fn coverage(table: &[HazardRecord], anthropogenic: Option<&[bool]>) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for k in ACUTE_ORDER {
        out.push((
            format!("acute {k}"),
            table.iter().filter(|r| r.acute == k).count(),
        ));
    }
    for k in CHRONIC_ORDER {
        out.push((
            format!("chronic {k}"),
            table.iter().filter(|r| r.chronic == k).count(),
        ));
    }
    for (t, _) in TIER_LABEL {
        out.push((
            format!("acute tier {t}"),
            table.iter().filter(|r| r.acute_tier == t).count(),
        ));
    }
    for (t, _) in TIER_LABEL {
        out.push((
            format!("chronic tier {t}"),
            table.iter().filter(|r| r.chronic_tier == t).count(),
        ));
    }
    let none = |r: &HazardRecord| r.acute == NOT_ASSESSABLE && r.chronic == NOT_ASSESSABLE;
    out.push((
        "not assessable (acute and chronic)".into(),
        table.iter().filter(|r| none(r)).count(),
    ));
    if let Some(an) = anthropogenic {
        out.push((
            "anthropogenic not assessable".into(),
            table.iter().zip(an).filter(|(r, a)| none(r) && **a).count(),
        ));
    }
    out
}
