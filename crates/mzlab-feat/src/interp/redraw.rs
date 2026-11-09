//! The drawing of a position that is only known as the JSON of its `site` (`sites/1` of `tp.position`, `structure.render` and its `render_text`): the key is read back into a
//! `SiteReport` and drawn by `drawing::drawing`, the function that made the `drawing` stored in the block `interp`, so the bytes are the same.
//! The candidate sites are not in the JSON; they are made again from the structure and the change, which is deterministic.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

use super::drawing::drawing;
use super::site::{SiteReport, Vote};
use super::site_json::drawing_json;
use crate::chem::graph::Mol;
use crate::chem::positions::sites;
use crate::parent::parent_of;

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

/// `drawings/1` of `structure.render` for the `sites/1` file and the parent file, the text of `flow run` and of the `.wasm` `interp`. The SMILES of the
/// parent file wins over the one `sites/1` carries.
pub fn render_text(sites: &[u8], parent: &[u8]) -> Result<String> {
    let doc = parse(sites)?;
    if doc.get("schema").and_then(|v| v.str()) != Some("sites/1") {
        return Err(Error::new("err.flow.render.sites"));
    }
    let given = parent_of(parent)?.smiles;
    let carried = doc.get("parent").and_then(|v| v.str()).unwrap_or("");
    let smiles = if given.is_empty() { carried } else { &given };
    if smiles.is_empty() {
        return Err(Error::new("err.flow.position.smiles"));
    }
    let mut o = Out::default();
    o.raw("{\"schema\":\"drawings/1\",\"parent\":")
        .str(smiles)
        .raw(",\"rows\":[");
    for (i, r) in doc
        .get("rows")
        .map(|a| a.arr())
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .int(r.get("id").and_then(|v| v.f64()).unwrap_or(0.0) as i64)
            .raw(",\"drawing\":")
            .raw(&drawing_of_site(smiles, r.get("site")))
            .raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}
