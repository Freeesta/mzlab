//! The parent file of a Flow plan (`tp.link`, `tp.position`, `tp.dig_lr`, `structure.render`, `structure.reactivity`): a JSON or one line with a formula
//! or a SMILES, read into the neutral formula, the SMILES, its structure graph, the retention time and the adduct. Shared by the CLI and the `.wasm` modules.

use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

use crate::chem::grafo::analyse_json;
use crate::hypo::Structure;

/// The parent of the file: neutral formula, SMILES when there is one, and the retention time and the adduct when the file says them.
pub struct Parent {
    pub formula: String,
    pub smiles: String,
    pub structure: String,
    pub rt: Option<f64>,
    /// The adduct of the parent ion when the file says it (`[M-H]-`): its sign is the polarity of the products; `[M+H]+` otherwise.
    pub adduct: String,
}

/// The parent file: a JSON `{"formula"|"neutral", "smiles", "rt", "adduct"}`, or one line with a formula (written as `elements::fmt` writes it) or a SMILES.
pub fn parent_of(bytes: &[u8]) -> Result<Parent> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    let json = parse(text.as_bytes()).ok().filter(|v| {
        v.get("rt").is_some()
            || v.get("formula").is_some()
            || v.get("neutral").is_some()
            || v.get("smiles").is_some()
    });
    let (mut formula, smiles, rt) = match &json {
        Some(v) => {
            let s = |k: &str| {
                v.get(k)
                    .and_then(Value::str)
                    .unwrap_or("")
                    .trim()
                    .to_string()
            };
            let f = [s("formula"), s("neutral")]
                .into_iter()
                .find(|x| !x.is_empty());
            (
                f.unwrap_or_default(),
                s("smiles"),
                v.get("rt").and_then(Value::f64).filter(|t| t.is_finite()),
            )
        }
        None => {
            let line = text.lines().next().unwrap_or("").trim().to_string();
            let is_formula =
                elements::parse_formula(&line).is_ok_and(|f| elements::fmt(&f) == line);
            if is_formula {
                (line, String::new(), None)
            } else {
                (String::new(), line, None)
            }
        }
    };
    let structure = if smiles.is_empty() {
        String::new()
    } else {
        analyse_json(&smiles)?
    };
    if formula.is_empty() {
        formula = Structure::parse(&structure)?.formula.unwrap_or_default();
    }
    if formula.is_empty() {
        return Err(Error::new("error.hypo.noParent"));
    }
    let adduct = json
        .as_ref()
        .and_then(|v| v.get("adduct").and_then(Value::str))
        .map(|a| a.trim().to_string())
        .filter(|a| elements::ion_mz(0.0, a).is_some())
        .unwrap_or_else(|| "[M+H]+".into());
    Ok(Parent {
        formula,
        smiles,
        structure,
        rt,
        adduct,
    })
}
