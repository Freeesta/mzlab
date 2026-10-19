// Ported from Attributio ecotox/ecosar.py (taxon_values).
//! The ECOSAR taxon values of one structure after the rules of use: excess-toxicity classes first, else the baseline (neutral organics), out-of-domain
//! endpoints dropped. Used by `hazard` for the T4 prediction.

use super::hazard::fmt_gn;
use super::snapshot::EcosarEndpoint;
use std::collections::BTreeSet;

pub(super) const BASELINE: &str = "Neutral Organics";

/// Per-taxon values, one `(taxon, mg/L)` pair each.
pub(super) type Taxa = Vec<(String, f64)>;

pub(super) struct TaxonValues {
    pub acute: Taxa,
    pub chronic: Taxa,
    pub classes: Vec<String>,
    pub baseline_only: bool,
    pub logkow: Option<f64>,
    pub nes: [Option<Nes>; 2],
}

/// «No effects at saturation»: every endpoint of a kind is out of domain and none lacks a value.
pub(super) struct Nes {
    pub why: String,
    pub logkow: Option<f64>,
    pub ws: Option<f64>,
}

fn taxon_of(org: &str) -> Option<&'static str> {
    match org {
        "Fish" => Some("fish"),
        "Daphnid" => Some("crustacean"),
        "Green Algae" => Some("algae"),
        _ => None,
    }
}

fn acute_ep(o: &str, e: &str) -> bool {
    matches!(
        (o, e),
        ("Fish", "LC50") | ("Daphnid", "LC50") | ("Green Algae", "EC50")
    )
}

fn chronic_ep(o: &str, e: &str) -> bool {
    matches!(
        (o, e),
        ("Fish", "ChV") | ("Daphnid", "ChV") | ("Green Algae", "ChV")
    )
}

fn no_effect_at_saturation(rows: &[&EcosarEndpoint]) -> Option<Nes> {
    if rows.is_empty()
        || rows.iter().any(|e| e.in_domain)
        || rows.iter().any(|e| e.excluded_because == "no value")
    {
        return None;
    }
    let why = rows
        .iter()
        .map(|e| e.excluded_because.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("; ");
    Some(Nes {
        why,
        logkow: rows[0].logkow.filter(|v| !v.is_nan()),
        ws: rows[0].water_solubility.filter(|v| !v.is_nan()),
    })
}

/// The in-domain rows to use: the excess-toxicity classes when any has a value, else the baseline.
fn rows_in_use<'a>(
    in_domain: &[&'a EcosarEndpoint],
    excess: &[&String],
) -> Vec<&'a EcosarEndpoint> {
    let wanted = |e: &&EcosarEndpoint| {
        if !excess.is_empty() && in_domain.iter().any(|x| excess.contains(&&x.qsar_class)) {
            excess.contains(&&e.qsar_class)
        } else {
            e.qsar_class == BASELINE
        }
    };
    in_domain.iter().filter(|e| wanted(e)).copied().collect()
}

pub(super) fn taxon_values(eco: &[EcosarEndpoint], key: &str) -> TaxonValues {
    let all_key: Vec<&EcosarEndpoint> = eco.iter().filter(|e| e.key == key).collect();
    let in_domain: Vec<&EcosarEndpoint> = all_key.iter().filter(|e| e.in_domain).copied().collect();
    let classes: Vec<String> = all_key
        .iter()
        .map(|e| e.qsar_class.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let excess: Vec<&String> = classes.iter().filter(|c| c.as_str() != BASELINE).collect();
    let use_rows = rows_in_use(&in_domain, &excess);
    let with_taxon: Vec<&EcosarEndpoint> = all_key
        .iter()
        .filter(|e| taxon_of(&e.organism).is_some())
        .copied()
        .collect();
    let of_kind = |is_kind: fn(&str, &str) -> bool| -> Vec<&EcosarEndpoint> {
        with_taxon
            .iter()
            .filter(|e| is_kind(&e.organism, &e.endpoint))
            .copied()
            .collect()
    };
    let mut out = TaxonValues {
        acute: vec![],
        chronic: vec![],
        classes: classes.clone(),
        baseline_only: excess.is_empty(),
        logkow: in_domain
            .first()
            .and_then(|e| e.logkow)
            .filter(|v| !v.is_nan()),
        nes: [
            no_effect_at_saturation(&of_kind(acute_ep)),
            no_effect_at_saturation(&of_kind(chronic_ep)),
        ],
    };
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

pub(super) fn as_pairs(t: &Taxa) -> Vec<(&str, f64)> {
    t.iter().map(|(k, v)| (k.as_str(), *v)).collect()
}

pub(super) fn sorted_values(t: &Taxa) -> String {
    let mut v: Vec<&(String, f64)> = t.iter().collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v.iter()
        .map(|(k, x)| format!("{k} {} mg/L", fmt_gn(*x, 3)))
        .collect::<Vec<_>>()
        .join(", ")
}
