//! The drawing of a position that is only known as the JSON of its `site` (`sites/1` of `tp.position`, `structure.render`): the key is read back into a
//! `SiteReport` and drawn by `disegno::drawing`, the function that made the `drawing` stored in the block `interp`, so the bytes are the same.
//! The candidate sites are not in the JSON; they are made again from the structure and the change, which is deterministic.

use mzlab_core::json::Value;

use super::disegno::drawing;
use super::sito::{SiteReport, Vote};
use super::sito_json::drawing_json;
use crate::chem::grafo::Mol;
use crate::chem::posizioni::sites;

fn atoms(v: Option<&Value>) -> Vec<usize> {
    v.map(|a| {
        a.arr()
            .iter()
            .filter_map(Value::f64)
            .map(|x| x as usize)
            .collect()
    })
    .unwrap_or_default()
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::str).unwrap_or("").to_string()
}

fn count(v: &Value, key: &str) -> usize {
    v.get(key).and_then(Value::f64).unwrap_or(0.0) as usize
}

/// One class of the key `classes` of a site; the log-likelihood is not written there and the drawing never reads it.
fn vote(v: &Value) -> Vote {
    let status = match v.get("status").and_then(Value::str) {
        Some("excluded") => "excluded",
        Some("supported") => "supported",
        _ => "open",
    };
    Vote {
        id: count(v, "id"),
        atoms: atoms(v.get("atoms")),
        kind: text(v, "kind"),
        status,
        n_support: count(v, "n_support"),
        n_against: count(v, "n_against"),
        ll: 0.0,
        support: v.get("support").and_then(Value::f64).unwrap_or(0.0),
        group: v.get("group").and_then(Value::str).map(str::to_string),
    }
}

/// The report of a `site` JSON for the structure `smiles`; the sites come from the structure and the change.
fn report(smiles: &str, site: &Value) -> SiteReport {
    let transform = text(site, "transform");
    let list = Mol::parse(smiles)
        .ok()
        .and_then(|m| sites(&m, &transform).ok())
        .unwrap_or_default();
    let confidence = match site.get("confidence").and_then(Value::str) {
        Some("site") => "site",
        Some("region") => "region",
        Some("uncertain") => "uncertain",
        _ => "cannot_say",
    };
    let region = site.get("region").filter(|r| !r.is_null());
    let retained = site.get("retained").filter(|r| !r.is_null());
    SiteReport {
        transform,
        confidence,
        reasons: vec![],
        source: if site.get("source").and_then(Value::str) == Some("tree") {
            "tree"
        } else {
            "cuts"
        },
        votes: site
            .get("classes")
            .map(|c| c.arr().iter().map(vote).collect())
            .unwrap_or_default(),
        sites: list,
        region: region.map(|r| (text(r, "name"), atoms(r.get("atoms")))),
        retained: retained.map(|r| (atoms(r.get("atoms")), text(r, "text"))),
        informative: count(site, "informative"),
        margin: site.get("margin").and_then(Value::f64),
        sublevel: "",
        why: "",
    }
}

/// The JSON of the drawing of a row whose `site` is given (`null` when the row has none), for the parent `smiles`.
pub fn drawing_of_site(smiles: &str, site: Option<&Value>) -> String {
    match site.filter(|s| !s.is_null()) {
        Some(s) => drawing_json(&drawing(smiles, &report(smiles, s))),
        None => "null".to_string(),
    }
}
