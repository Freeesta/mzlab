// Ported from Attributio ecotox/hazard.py (tier candidates).
//! The per-tier hazard candidates of one identity (curated, measured, notified, predicted) and the choice of the best one: first tier with a category wins,
//! within the tier the most severe category. Used by `hazard::record`.

use super::ecosar::{as_pairs, sorted_values, taxon_values, Taxa};
use super::ghs;
use super::hazard::{fmt_gn, Curated, HazardData, NOT_ASSESSABLE};
use super::snapshot::EcotoxValue;
use std::collections::BTreeSet;

pub(super) const ACUTE_ORDER: [&str; 5] = [
    "Acute 1",
    "Acute 2",
    "Acute 3",
    ghs::NOT_CLASSIFIED,
    NOT_ASSESSABLE,
];
pub(super) const CHRONIC_ORDER: [&str; 6] = [
    "Chronic 1",
    "Chronic 2",
    "Chronic 3",
    "Chronic 4",
    ghs::NOT_CLASSIFIED,
    NOT_ASSESSABLE,
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

pub(super) fn measured(d: &HazardData, blocks: &[String]) -> (Taxa, Taxa, String) {
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

pub(super) fn ghs_codes(d: &HazardData, blocks: &[String]) -> Vec<String> {
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

pub(super) fn log_kow(d: &HazardData, blocks: &[String]) -> Option<f64> {
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

fn acute_to_chronic(a: &str) -> &'static str {
    match a {
        "Acute 1" => "Chronic 1",
        "Acute 2" => "Chronic 2",
        "Acute 3" => "Chronic 3",
        "Not classified" => ghs::NOT_CLASSIFIED,
        _ => "",
    }
}

/// `(tier, category, basis)` candidates.
type Cands = Vec<(String, String, String)>;

fn add(lst: &mut Cands, tier: &str, cat: Option<&str>, basis: String) {
    if let Some(c) = cat.filter(|c| !c.is_empty()) {
        lst.push((tier.to_string(), c.to_string(), basis));
    }
}

/// All the candidates of one identity, plus what the sources say besides the category.
#[derive(Default)]
pub(super) struct Candidates {
    acute: Cands,
    chronic: Cands,
    pub chronic_route: String,
    pub ecosar_classes: String,
}

/// The winning category of one kind (acute or chronic) and the lower-tier disagreements.
pub(super) struct Picked {
    pub category: String,
    pub tier: String,
    pub basis: String,
    pub other: BTreeSet<String>,
}

impl Candidates {
    pub fn curated(&mut self, rows: &[&Curated]) {
        for r in rows {
            let src = format!("curated: {}; {}", r.basis, r.note)
                .trim_matches(|c| c == ';' || c == ' ')
                .to_string();
            let src = src.trim_matches(';').to_string();
            let tier = if r.tier == "T4" {
                "T4c"
            } else {
                r.tier.as_str()
            };
            add(&mut self.acute, tier, Some(&r.acute), src.clone());
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
            let note = if r.chronic.is_empty() {
                " (chronic from the acute category, not rapidly degradable)"
            } else {
                ""
            };
            add(&mut self.chronic, tier, Some(ch), format!("{src}{note}"));
        }
    }

    pub fn measured(&mut self, d: &HazardData, blocks: &[String], kow: Option<f64>) {
        let (a, c, used) = measured(d, blocks);
        if a.is_empty() && c.is_empty() {
            return;
        }
        let cl = ghs::classify(&as_pairs(&a), &as_pairs(&c), false, kow, None);
        add(&mut self.acute, "T2", cl.acute.as_deref(), used.clone());
        let route = cl.chronic_route.clone().unwrap_or_else(|| "None".into());
        add(
            &mut self.chronic,
            "T2",
            cl.chronic.as_deref(),
            format!("{used} [route: {route}]"),
        );
        self.chronic_route = cl.chronic_route.unwrap_or_default();
    }

    pub fn notified(&mut self, d: &HazardData, blocks: &[String]) {
        let codes = ghs_codes(d, blocks);
        if codes.is_empty() {
            return;
        }
        let refs: Vec<&str> = codes.iter().map(String::as_str).collect();
        let (ha, hc) = ghs::from_h_codes(&refs);
        let b = format!("PubChem GHS: {}", codes.join(", "));
        add(&mut self.acute, "T3", ha.as_deref(), b.clone());
        add(&mut self.chronic, "T3", hc.as_deref(), b);
    }
}

fn order_of(chronic: bool) -> &'static [&'static str] {
    if chronic {
        &CHRONIC_ORDER
    } else {
        &ACUTE_ORDER
    }
}

impl Candidates {
    pub fn predicted(&mut self, d: &HazardData, blocks: &[String]) {
        let Some(key) = blocks
            .iter()
            .find(|b| d.ecosar.iter().any(|e| &e.key == *b))
        else {
            return;
        };
        let tv = taxon_values(&d.ecosar, key);
        self.ecosar_classes = tv.classes.join(", ");
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
            let basis = format!(
                "ECOSAR 2.2 {}: {}{lb}",
                tv.classes.join(", "),
                sorted_values(&tv.acute)
            );
            add(&mut self.acute, "T4", cl.acute.as_deref(), basis);
            let basis = format!("ECOSAR 2.2 ChV: {}{lb}", sorted_values(&tv.chronic));
            add(&mut self.chronic, "T4", cl.chronic.as_deref(), basis);
        }
        for i in 0..2 {
            let have = if i == 0 { &tv.acute } else { &tv.chronic };
            let Some(nes) = tv.nes[i].as_ref().filter(|_| have.is_empty()) else {
                continue;
            };
            let ws = nes.ws.map_or(String::new(), |w| {
                format!("; water solubility {} mg/L", fmt_gn(w, 3))
            });
            let why = nes.why.replace(
                "no effects at saturation (> 10x water solubility)",
                "> 10x water solubility",
            );
            let txt = format!(
                "ECOSAR 2.2 {}: no effects at saturation ({why}{ws})",
                tv.classes.join(", ")
            );
            if i == 0 {
                add(&mut self.acute, "T4", Some(ghs::NOT_CLASSIFIED), txt);
            } else if nes.logkow.is_some_and(|k| k >= 4.0) {
                let net = "CLP safety net: poorly soluble, log Kow >= 4, not rapidly degradable";
                add(
                    &mut self.chronic,
                    "T4",
                    Some("Chronic 4"),
                    format!("{txt}; {net}"),
                );
            } else {
                add(&mut self.chronic, "T4", Some(ghs::NOT_CLASSIFIED), txt);
            }
        }
    }
}

/// The first of the lowest-ranked items.
fn best_by<'a>(
    items: impl Iterator<Item = &'a str>,
    rank: impl Fn(&str) -> f64,
) -> Option<&'a str> {
    items.fold(None, |b, x| {
        if b.is_none_or(|y| rank(x) < rank(y)) {
            Some(x)
        } else {
            b
        }
    })
}

impl Candidates {
    /// The best acute (`chronic` false) or chronic candidate, if any.
    pub fn pick(&self, chronic: bool) -> Option<Picked> {
        let lst = if chronic { &self.chronic } else { &self.acute };
        let order = order_of(chronic);
        let best_tier = best_by(lst.iter().map(|(t, _, _)| t.as_str()), tier_rank)?;
        let same: Vec<(&str, &str)> = lst
            .iter()
            .filter(|(t, _, _)| t == best_tier)
            .map(|(_, c, b)| (c.as_str(), b.as_str()))
            .collect();
        let pos = |c: &str| order.iter().position(|o| *o == c).unwrap_or(99) as f64;
        let cat = best_by(same.iter().map(|(c, _)| *c), pos)?;
        let basis = same
            .iter()
            .filter(|(c, _)| *c == cat)
            .map(|(_, b)| *b)
            .collect::<Vec<_>>()
            .join(" | ");
        let other = lst
            .iter()
            .filter(|(t, c, _)| t != best_tier && c != cat)
            .map(|(t, c, _)| format!("{} {c}", t.trim_end_matches('c')))
            .collect();
        Some(Picked {
            category: cat.to_string(),
            tier: best_tier.trim_end_matches('c').to_string(),
            basis,
            other,
        })
    }
}
