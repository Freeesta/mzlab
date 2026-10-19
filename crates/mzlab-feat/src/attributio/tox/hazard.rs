// Ported from Attributio ecotox/hazard.py, ecotox/ecosar.py (taxon_values).
//! One aquatic hazard record (acute + chronic) per chemical identity. Tiers, first one with a category wins, acute and chronic separately: T1 regulatory (CLP
//! harmonised, curated), T2 measured (ECOTOX snapshot and curated values), T3 notified (PubChem GHS H-statements), T4 predicted (curated judgement, then ECOSAR 2.2),
//! T5 not assessable. Degradability is unknown for almost every compound, so chronic categories assume «not rapidly degradable» and every record says so.
//! A hazard class is a property of a structure, not a risk: no concentration, no PNEC. Everything is read from the snapshots the caller passes in.

use super::ghs;
use super::snapshot::{EcosarEndpoint, EcotoxValue, RegistryRow};
use crate::attributio::names::{canonical_name, split_rt_isomer};
use crate::attributio::pattern::Pattern;
use std::collections::BTreeSet;

pub const NOT_ASSESSABLE: &str = "Not assessable";
pub const DEGRADABILITY: &str = "not rapidly degradable (assumed: no degradability data used)";
const BASELINE: &str = "Neutral Organics";
const ACUTE_ORDER: [&str; 5] = [
    "Acute 1",
    "Acute 2",
    "Acute 3",
    ghs::NOT_CLASSIFIED,
    NOT_ASSESSABLE,
];
const CHRONIC_ORDER: [&str; 6] = [
    "Chronic 1",
    "Chronic 2",
    "Chronic 3",
    "Chronic 4",
    ghs::NOT_CLASSIFIED,
    NOT_ASSESSABLE,
];
/// Tier labels in report order.
pub const TIER_LABEL: [(&str, &str); 5] = [
    ("T1", "regulatory (CLP harmonised)"),
    ("T2", "measured"),
    ("T3", "notified classification"),
    ("T4", "predicted"),
    ("T5", "not assessable"),
];

fn tier_rank(t: &str) -> f64 {
    match t {
        "T1" => 1.0,
        "T2" => 2.0,
        "T3" => 3.0,
        "T4c" => 4.0,
        "T4" => 4.5,
        _ => 5.0,
    }
}

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
fn fmt_gn(v: f64, n: usize) -> String {
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

fn duration(h: f64) -> String {
    if h >= 24.0 * 7.0 {
        format!("{} d", fmt_round(h / 24.0))
    } else {
        format!("{} h", fmt_round(h))
    }
}

/// `f"{x:.0f}"` (round half to even like Python).
fn fmt_round(x: f64) -> String {
    let r = x.round();
    let r = if (x - x.trunc()).abs() == 0.5 && r % 2.0 != 0.0 {
        r - x.signum()
    } else {
        r
    };
    format!("{r:.0}")
}

type Taxa = Vec<(String, f64)>;

fn measured(d: &HazardData, blocks: &[String]) -> (Taxa, Taxa, String) {
    let g: Vec<&EcotoxValue> = d
        .values
        .iter()
        .filter(|v| blocks.contains(&v.ik1) && v.op == "=")
        .collect();
    let (mut acute, mut chronic, mut used) = (Taxa::new(), Taxa::new(), Vec::new());
    for kind in ["acute", "chronic"] {
        let taxa: BTreeSet<&str> = g
            .iter()
            .filter(|v| v.kind == kind)
            .map(|v| v.taxon.as_str())
            .collect();
        for t in taxa {
            let mut h: Vec<&&EcotoxValue> = g
                .iter()
                .filter(|v| v.kind == kind && v.taxon == t)
                .collect();
            // stable sort, NaN last (pandas): the first of the lowest values
            h.sort_by(|a, b| {
                a.value_mg_l
                    .unwrap_or(f64::INFINITY)
                    .total_cmp(&b.value_mg_l.unwrap_or(f64::INFINITY))
            });
            let r = h[0];
            let v = r.value_mg_l.unwrap_or(f64::NAN);
            let dur = r
                .duration_h
                .trim()
                .parse::<f64>()
                .map_or(r.duration_h.clone(), duration);
            used.push(format!(
                "{kind} {t}: {} {} {dur} {} mg/L (n={}, ECOTOX)",
                r.species,
                r.endpoint,
                fmt_gn(v, 3),
                r.n_results
            ));
            if kind == "acute" {
                &mut acute
            } else {
                &mut chronic
            }
            .push((t.to_string(), v));
        }
    }
    (acute, chronic, used.join("; "))
}

fn ghs_codes(d: &HazardData, blocks: &[String]) -> Vec<String> {
    let set: BTreeSet<String> = d
        .registry
        .iter()
        .filter(|r| blocks.contains(&r.first_block))
        .flat_map(|r| r.ghs_aquatic.split('|'))
        .filter(|c| c.starts_with("H4"))
        .map(str::to_string)
        .collect();
    set.into_iter().collect()
}

fn log_kow(d: &HazardData, blocks: &[String]) -> Option<f64> {
    if let Some(k) = d
        .ecosar
        .iter()
        .filter(|e| blocks.contains(&e.key))
        .find_map(|e| e.logkow.filter(|v| !v.is_nan()))
    {
        return Some(k);
    }
    d.registry
        .iter()
        .filter(|r| blocks.contains(&r.first_block))
        .find_map(|r| r.xlogp.filter(|v| !v.is_nan()))
}

/// The ECOSAR taxon values of one structure after the rules of use (excess-toxicity classes first, else baseline; out-of-domain endpoints dropped).
struct TaxonValues {
    acute: Taxa,
    chronic: Taxa,
    classes: Vec<String>,
    baseline_only: bool,
    logkow: Option<f64>,
    nes: [Option<Nes>; 2],
}

struct Nes {
    why: String,
    logkow: Option<f64>,
    ws: Option<f64>,
}

fn taxon_of(org: &str) -> Option<&'static str> {
    match org {
        "Fish" => Some("fish"),
        "Daphnid" => Some("crustacean"),
        "Green Algae" => Some("algae"),
        _ => None,
    }
}

fn taxon_values(eco: &[EcosarEndpoint], key: &str) -> TaxonValues {
    let all_key: Vec<&EcosarEndpoint> = eco.iter().filter(|e| e.key == key).collect();
    let g: Vec<&&EcosarEndpoint> = all_key.iter().filter(|e| e.in_domain).collect();
    let classes: Vec<String> = all_key
        .iter()
        .map(|e| e.qsar_class.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let excess: Vec<&String> = classes.iter().filter(|c| c.as_str() != BASELINE).collect();
    let use_rows: Vec<&&EcosarEndpoint> =
        if !excess.is_empty() && g.iter().any(|e| excess.contains(&&e.qsar_class)) {
            g.iter()
                .filter(|e| excess.contains(&&e.qsar_class))
                .copied()
                .collect()
        } else {
            g.iter()
                .filter(|e| e.qsar_class == BASELINE)
                .copied()
                .collect()
        };
    let allk: Vec<&&EcosarEndpoint> = all_key
        .iter()
        .filter(|e| taxon_of(&e.organism).is_some())
        .collect();
    let acute_ep = |o: &str, e: &str| {
        matches!(
            (o, e),
            ("Fish", "LC50") | ("Daphnid", "LC50") | ("Green Algae", "EC50")
        )
    };
    let chronic_ep = |o: &str, e: &str| {
        matches!(
            (o, e),
            ("Fish", "ChV") | ("Daphnid", "ChV") | ("Green Algae", "ChV")
        )
    };
    let mut out = TaxonValues {
        acute: vec![],
        chronic: vec![],
        classes: classes.clone(),
        baseline_only: excess.is_empty(),
        logkow: g.first().and_then(|e| e.logkow).filter(|v| !v.is_nan()),
        nes: [None, None],
    };
    for (i, is_kind) in [&acute_ep as &dyn Fn(&str, &str) -> bool, &chronic_ep]
        .into_iter()
        .enumerate()
    {
        let rows: Vec<&&&EcosarEndpoint> = allk
            .iter()
            .filter(|e| is_kind(&e.organism, &e.endpoint))
            .collect();
        if !rows.is_empty()
            && !rows.iter().any(|e| e.in_domain)
            && rows.iter().all(|e| e.excluded_because != "no value")
        {
            let why = rows
                .iter()
                .map(|e| e.excluded_because.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join("; ");
            out.nes[i] = Some(Nes {
                why,
                logkow: rows[0].logkow.filter(|v| !v.is_nan()),
                ws: rows[0].water_solubility.filter(|v| !v.is_nan()),
            });
        }
    }
    let pairs: BTreeSet<(&str, &str)> = use_rows
        .iter()
        .map(|e| (e.organism.as_str(), e.endpoint.as_str()))
        .collect();
    for (org, ep) in pairs {
        let Some(t) = taxon_of(org) else { continue };
        let min = use_rows
            .iter()
            .filter(|e| e.organism == org && e.endpoint == ep)
            .filter_map(|e| e.value_mg_l)
            .fold(f64::INFINITY, f64::min);
        if acute_ep(org, ep) {
            out.acute.push((t.to_string(), min));
        } else if chronic_ep(org, ep) {
            out.chronic.push((t.to_string(), min));
        }
    }
    out
}

fn as_pairs(t: &Taxa) -> Vec<(&str, f64)> {
    t.iter().map(|(k, v)| (k.as_str(), *v)).collect()
}

fn sorted_values(t: &Taxa) -> String {
    let mut v: Vec<&(String, f64)> = t.iter().collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v.iter()
        .map(|(k, x)| format!("{k} {} mg/L", fmt_gn(*x, 3)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn acute_to_chronic(a: &str) -> &'static str {
    match a {
        "Acute 1" => "Chronic 1",
        "Acute 2" => "Chronic 2",
        "Acute 3" => "Chronic 3",
        "Not classified" => ghs::NOT_CLASSIFIED,
        _ => "",
    }
}

type Cands = Vec<(String, String, String)>;

fn add(lst: &mut Cands, tier: &str, cat: Option<&str>, basis: String) {
    if let Some(c) = cat.filter(|c| !c.is_empty()) {
        lst.push((tier.to_string(), c.to_string(), basis));
    }
}

/// The hazard record of one identity (`level` 3 adds the candidate-structure caveat).
pub fn record(
    d: &HazardData,
    name: &str,
    campaign: Option<&str>,
    level: Option<i64>,
) -> HazardRecord {
    let mut rec = HazardRecord {
        identity: name.into(),
        blocks: String::new(),
        structure_basis: String::new(),
        acute: NOT_ASSESSABLE.into(),
        acute_tier: "T5".into(),
        acute_basis: String::new(),
        chronic: NOT_ASSESSABLE.into(),
        chronic_tier: "T5".into(),
        chronic_basis: String::new(),
        chronic_route: String::new(),
        degradability: DEGRADABILITY.into(),
        ecosar_classes: String::new(),
        log_kow: None,
        caveat: String::new(),
        cross_check: String::new(),
    };
    let (blocks, how) = resolve_blocks(d, name, campaign);
    rec.blocks = blocks.join("|");
    rec.structure_basis = how;
    let mut caveats: Vec<String> = Vec::new();
    if split_rt_isomer(name).is_some() {
        caveats.push(
            "RT isomer: hazard of the named structure, the peak may be another isomer".into(),
        );
    }
    if level == Some(3) {
        caveats.push("Level 3: hazard of the CSI:FingerID candidate structure".into());
    }
    let kow = if blocks.is_empty() {
        None
    } else {
        log_kow(d, &blocks)
    };
    rec.log_kow = kow;
    let cu = curated_for(d, &blocks, name);
    let (mut acute_c, mut chronic_c): (Cands, Cands) = (vec![], vec![]);
    for r in &cu {
        let src = format!("curated: {}; {}", r.basis, r.note)
            .trim_matches(|c| c == ';' || c == ' ')
            .to_string();
        let src = src.trim_matches(';').to_string();
        let tier = if r.tier == "T4" {
            "T4c"
        } else {
            r.tier.as_str()
        };
        add(&mut acute_c, tier, Some(&r.acute), src.clone());
        let derived = if (r.tier == "T1" || r.tier == "T2") && !r.acute.is_empty() {
            acute_to_chronic(&r.acute)
        } else {
            ""
        };
        let ch = if r.chronic.is_empty() {
            derived
        } else {
            r.chronic.as_str()
        };
        add(
            &mut chronic_c,
            tier,
            Some(ch),
            format!(
                "{src}{}",
                if r.chronic.is_empty() {
                    " (chronic from the acute category, not rapidly degradable)"
                } else {
                    ""
                }
            ),
        );
    }
    if !blocks.is_empty() {
        let (a, c, used) = measured(d, &blocks);
        if !a.is_empty() || !c.is_empty() {
            let cl = ghs::classify(&as_pairs(&a), &as_pairs(&c), false, kow, None);
            add(&mut acute_c, "T2", cl.acute.as_deref(), used.clone());
            add(
                &mut chronic_c,
                "T2",
                cl.chronic.as_deref(),
                format!(
                    "{used} [route: {}]",
                    cl.chronic_route.clone().unwrap_or_else(|| "None".into())
                ),
            );
            rec.chronic_route = cl.chronic_route.unwrap_or_default();
        }
        let codes = ghs_codes(d, &blocks);
        if !codes.is_empty() {
            let refs: Vec<&str> = codes.iter().map(String::as_str).collect();
            let (ha, hc) = ghs::from_h_codes(&refs);
            let b = format!("PubChem GHS: {}", codes.join(", "));
            add(&mut acute_c, "T3", ha.as_deref(), b.clone());
            add(&mut chronic_c, "T3", hc.as_deref(), b);
        }
        if let Some(key) = blocks
            .iter()
            .find(|b| d.ecosar.iter().any(|e| &e.key == *b))
        {
            let tv = taxon_values(&d.ecosar, key);
            rec.ecosar_classes = tv.classes.join(", ");
            if !tv.acute.is_empty() || !tv.chronic.is_empty() {
                let cl = ghs::classify(
                    &as_pairs(&tv.acute),
                    &as_pairs(&tv.chronic),
                    false,
                    tv.logkow,
                    None,
                );
                let lb = if tv.baseline_only {
                    " (baseline narcosis: lower bound, at least this hazard)"
                } else {
                    ""
                };
                add(
                    &mut acute_c,
                    "T4",
                    cl.acute.as_deref(),
                    format!(
                        "ECOSAR 2.2 {}: {}{lb}",
                        tv.classes.join(", "),
                        sorted_values(&tv.acute)
                    ),
                );
                add(
                    &mut chronic_c,
                    "T4",
                    cl.chronic.as_deref(),
                    format!("ECOSAR 2.2 ChV: {}{lb}", sorted_values(&tv.chronic)),
                );
            }
            for (i, kind) in ["acute", "chronic"].into_iter().enumerate() {
                let have = if i == 0 { &tv.acute } else { &tv.chronic };
                let Some(nes) = tv.nes[i].as_ref().filter(|_| have.is_empty()) else {
                    continue;
                };
                let ws = nes.ws.map_or(String::new(), |w| {
                    format!("; water solubility {} mg/L", fmt_gn(w, 3))
                });
                let txt = format!(
                    "ECOSAR 2.2 {}: no effects at saturation ({}{ws})",
                    tv.classes.join(", "),
                    nes.why.replace(
                        "no effects at saturation (> 10x water solubility)",
                        "> 10x water solubility"
                    )
                );
                if kind == "acute" {
                    add(&mut acute_c, "T4", Some(ghs::NOT_CLASSIFIED), txt);
                } else if nes.logkow.is_some_and(|k| k >= 4.0) {
                    add(&mut chronic_c, "T4", Some("Chronic 4"), format!("{txt}; CLP safety net: poorly soluble, log Kow >= 4, not rapidly degradable"));
                } else {
                    add(&mut chronic_c, "T4", Some(ghs::NOT_CLASSIFIED), txt);
                }
            }
        }
    }
    for (which, lst, order) in [
        ("acute", &acute_c, &ACUTE_ORDER[..]),
        ("chronic", &chronic_c, &CHRONIC_ORDER[..]),
    ] {
        if lst.is_empty() {
            continue;
        }
        let best_tier = lst
            .iter()
            .map(|(t, _, _)| t.as_str())
            .fold(None::<&str>, |b, t| {
                if b.is_none_or(|x| tier_rank(t) < tier_rank(x)) {
                    Some(t)
                } else {
                    b
                }
            })
            .unwrap();
        let same: Vec<(&str, &str)> = lst
            .iter()
            .filter(|(t, _, _)| t == best_tier)
            .map(|(_, c, b)| (c.as_str(), b.as_str()))
            .collect();
        let pos = |c: &str| order.iter().position(|o| *o == c).unwrap_or(99);
        let cat = same
            .iter()
            .map(|(c, _)| *c)
            .fold(None::<&str>, |b, c| {
                if b.is_none_or(|x| pos(c) < pos(x)) {
                    Some(c)
                } else {
                    b
                }
            })
            .unwrap();
        let basis = same
            .iter()
            .filter(|(c, _)| *c == cat)
            .map(|(_, b)| *b)
            .collect::<Vec<_>>()
            .join(" | ");
        let other: BTreeSet<String> = lst
            .iter()
            .filter(|(t, c, _)| t != best_tier && c != cat)
            .map(|(t, c, _)| format!("{} {c}", t.trim_end_matches('c')))
            .collect();
        let tier = best_tier.trim_end_matches('c').to_string();
        if which == "acute" {
            (rec.acute, rec.acute_tier, rec.acute_basis) = (cat.to_string(), tier, basis);
        } else {
            (rec.chronic, rec.chronic_tier, rec.chronic_basis) = (cat.to_string(), tier, basis);
        }
        if !other.is_empty() {
            let sep = if rec.cross_check.is_empty() { "" } else { "; " };
            rec.cross_check = format!(
                "{}{sep}{which}: {}",
                rec.cross_check,
                other.into_iter().collect::<Vec<_>>().join(", ")
            );
        }
    }
    let codes = if blocks.is_empty() {
        vec![]
    } else {
        ghs_codes(d, &blocks)
    };
    if !codes.is_empty()
        && rec.acute == "Acute 1"
        && rec.acute_tier == "T4"
        && !codes.iter().any(|c| c == "H400")
    {
        caveats.push(format!(
            "predicted Acute 1 but the notified classification ({}) has no H400",
            codes.join(", ")
        ));
    }
    if blocks.is_empty() && cu.is_empty() {
        caveats.push("no structure in the registry".into());
    }
    rec.caveat = caveats.join("; ");
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
