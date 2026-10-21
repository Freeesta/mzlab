//! The position of the change of every interpreted row of a `.mzfeat` again under other thresholds (`tp.position`). With the default thresholds the answer
//! is the `site` the interpretation stored, byte for byte. With others the site is made again from what the row holds (the change, the consensus MS2 as
//! rounded in the block, the counts of its scans) and the structure of the parent; a row whose stored site was voted by the MSn tree keeps it, because the
//! tree is not in the file.

use mzlab_core::elements::{parse_formula, Formula};
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::json::{parse, Value};

use super::sito::{analyse_with, candidates, undetermined, Facts, SiteParams};
use super::sito_json::site_json;
use super::store;
use crate::chem::posizioni::{Peak, Pol};

/// The site of one row as JSON, and whether it was made again.
pub struct Positioned {
    pub id: u32,
    pub site: String,
    pub recomputed: bool,
}

fn bad() -> Error {
    Error::new("error.interp.failed").with("detail", "the block interp is not readable")
}

/// The text of the key `site` in the JSON of a row (`null` when the row has none).
fn stored_site(text: &str) -> Option<&str> {
    let a = text.find(",\"site\":")? + ",\"site\":".len();
    let b = text.find(",\"drawing\":")?;
    text.get(a..b)
}

fn numbers(v: Option<&Value>) -> Vec<f64> {
    v.map(|a| a.arr().iter().filter_map(Value::f64).collect())
        .unwrap_or_default()
}

fn isomers(doc: &Value) -> &'static str {
    match doc
        .get("ms2")
        .and_then(|m| m.get("isomers"))
        .and_then(Value::str)
    {
        Some("separated") => "separated",
        Some("not_separated") => "not_separated",
        _ => "single",
    }
}

/// The site of a row made again with `p`; `None` when the row has no change to place.
fn again(doc: &Value, smiles: &str, parent: &Formula, pol: Pol, p: &SiteParams) -> Option<String> {
    let change = doc.get("change").filter(|c| !c.is_null())?;
    let delta = change.get("delta").and_then(Value::str).unwrap_or("");
    let tp = parse_formula(change.get("neutral_formula").and_then(Value::str)?).ok()?;
    let scans = doc.get("ms2").and_then(|m| m.get("scans")).map(Value::arr);
    let scans = scans.unwrap_or_default();
    let facts = Facts {
        scans: scans.len(),
        clean: scans
            .iter()
            .filter(|s| s.get("chimeric").and_then(Value::bool) != Some(true))
            .count(),
        isomers: isomers(doc),
        has_reference: doc.get("vs_parent").is_some_and(|v| !v.is_null()),
    };
    let cons = doc.get("ms2").and_then(|m| m.get("consensus"));
    let (mz, rel) = (
        numbers(cons.and_then(|c| c.get("mz"))),
        numbers(cons.and_then(|c| c.get("rel"))),
    );
    let report = match candidates(smiles, parent, &tp) {
        Ok(c) => {
            let peaks: Vec<Peak> = mz
                .iter()
                .zip(&rel)
                .map(|(&mz, &inten)| Peak { mz, inten })
                .collect();
            analyse_with(&c, &peaks, pol, &facts, None, p)
        }
        Err(reason) => undetermined(delta, vec![reason]),
    };
    Some(site_json(&report))
}

/// The sites of the interpreted rows of the `.mzfeat` of `size` bytes, in the order of the ids. `smiles` and `parent` (its neutral formula) are the
/// parent of the interpretation; `pol` is the polarity of the series.
pub fn reposition<R: ReadAt>(
    r: &mut R,
    size: u64,
    (smiles, parent): (&str, &Formula),
    pol: Pol,
    p: &SiteParams,
) -> Result<Vec<Positioned>> {
    let (_, ids) = store::read_meta(r, size)?.ok_or_else(|| Error::new("error.interp.noTp"))?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let text = store::read_row(r, size, id)?.ok_or_else(bad)?;
        let kept = stored_site(&text).ok_or_else(bad)?.to_string();
        let doc = parse(text.as_bytes()).map_err(|_| bad())?;
        let by_tree = doc
            .get("site")
            .and_then(|s| s.get("source"))
            .and_then(Value::str)
            == Some("tree");
        let made = if *p == SiteParams::default() || by_tree {
            None
        } else {
            again(&doc, smiles, parent, pol, p)
        };
        out.push(Positioned {
            id,
            recomputed: made.is_some(),
            site: made.unwrap_or(kept),
        });
    }
    Ok(out)
}
