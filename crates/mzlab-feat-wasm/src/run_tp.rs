//! What `tp.generate` does, shared by the native executor (`mzlab-cli`, `flow_generate.rs`) and the browser (the export `tp_generate` below): the parent file, the
//! parameters of the hypotheses and the registry `ipotesi/1`. The structure graph of a SMILES comes in as text (the CLI makes it with `chem`, the page with the
//! `grafo` .wasm), so this module needs no chemistry.

use mzlab_core::elements;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};
use mzlab_feat::hypo::{generate, HypoParams, Inputs, Rules, Structure};

/// The parent file as written: a JSON `{"formula"|"neutral", "smiles", "rt"}`, or one line with a formula (written as `elements::fmt` writes it) or a SMILES.
/// `formula` is empty when the file gives only a SMILES.
pub struct ParentFile {
    pub formula: String,
    pub smiles: String,
    pub rt: Option<f64>,
}

pub fn read_parent(bytes: &[u8]) -> ParentFile {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    let json = parse(text.as_bytes()).ok().filter(|v| {
        ["rt", "formula", "neutral", "smiles"]
            .iter()
            .any(|k| v.get(k).is_some())
    });
    if let Some(v) = &json {
        let s = |k: &str| {
            v.get(k)
                .and_then(Value::str)
                .unwrap_or("")
                .trim()
                .to_string()
        };
        let formula = [s("formula"), s("neutral")]
            .into_iter()
            .find(|x| !x.is_empty())
            .unwrap_or_default();
        let rt = v.get("rt").and_then(Value::f64).filter(|t| t.is_finite());
        return ParentFile {
            formula,
            smiles: s("smiles"),
            rt,
        };
    }
    let line = text.lines().next().unwrap_or("").trim().to_string();
    // a formula is written as the engine writes it ("C13H24N4O3S"); "CCO" and "CC(C)O" are SMILES
    let is_formula = elements::parse_formula(&line).is_ok_and(|f| elements::fmt(&f) == line);
    if is_formula {
        ParentFile {
            formula: line,
            smiles: String::new(),
            rt: None,
        }
    } else {
        ParentFile {
            formula: String::new(),
            smiles: line,
            rt: None,
        }
    }
}

/// The neutral formula of the parent over its structure text (empty without a SMILES): the one of the file, else the one of the structure.
pub fn parent_formula(p: &ParentFile, structure: &str) -> Result<String> {
    let own = if structure.is_empty() {
        String::new()
    } else {
        Structure::parse(structure)?.formula.unwrap_or_default()
    };
    let formula = if p.formula.is_empty() {
        own
    } else {
        p.formula.clone()
    };
    if formula.is_empty() {
        return Err(Error::new("error.hypo.noParent"));
    }
    Ok(formula)
}

/// The parameters of `tp.generate` over `HypoParams::default()`; `get` gives the value a plan or a job set for a name.
pub fn hypo_params(get: &dyn Fn(&str) -> Option<Value>) -> HypoParams {
    let d = HypoParams::default();
    let num = |n: &str| get(n).and_then(|v| v.f64());
    let flag = |n: &str, dflt: bool| get(n).and_then(|v| v.bool()).unwrap_or(dflt);
    HypoParams {
        polarity: num("polarity").map_or(d.polarity, |v| if v < 0.0 { -1 } else { 1 }),
        max_steps: num("max_steps").map_or(d.max_steps, |v| v as u8),
        min_mz: num("min_mz").unwrap_or(d.min_mz),
        max_suspects: num("max_suspects").map_or(d.max_suspects, |v| v as usize),
        other_polarity: flag("other_polarity", d.other_polarity),
        nominal: flag("nominal", d.nominal),
    }
}

/// `ipotesi/1` text of a parent under the rules (`regole_tp_2.json` bytes): what the engine could find, never a verdict.
pub fn generate_text(
    rules: &[u8],
    formula: &str,
    structure: &str,
    hp: &HypoParams,
) -> Result<String> {
    let st = Structure::parse(structure)?;
    st.check_parent(formula)?;
    let inp = Inputs {
        parent: formula,
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &st.fragments,
        groups: st.groups.as_deref(),
    };
    let rules = Rules::parse_profile(rules, st.profile.as_deref())?;
    Ok(generate(&rules, &inp, hp)?.to_json())
}

/// A SMILES or a formula in the parent file, as `{"formula","smiles","rt"}`: the page asks the `grafo` .wasm for the structure of the SMILES before `tp_generate`.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn tp_parent(parent: &str) -> String {
    let bytes = parent.as_bytes();
    let p = read_parent(bytes);
    let mut o = mzlab_core::json::Out::default();
    o.raw("{\"formula\":")
        .str(&p.formula)
        .raw(",\"smiles\":")
        .str(&p.smiles);
    o.raw(",\"rt\":");
    match p.rt {
        Some(t) => o.num(t),
        None => o.raw("null"),
    };
    o.raw("}");
    o.0
}

/// `tp.generate` in the browser: the registry of the hypotheses of the parent file `parent` (text) with the structure text of its SMILES (empty for a formula)
/// under the parameters `params` (JSON object of the step, `{}` for the defaults) and the rules.
#[cfg(feature = "proofs")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn tp_generate(
    rules: &[u8],
    parent: &str,
    structure: &str,
    params: &str,
) -> std::result::Result<String, wasm_bindgen::JsValue> {
    let run = || -> Result<String> {
        let p = read_parent(parent.as_bytes());
        let formula = parent_formula(&p, structure)?;
        let set = parse(params.as_bytes())?;
        generate_text(
            rules,
            &formula,
            structure,
            &hypo_params(&|n| set.get(n).cloned()),
        )
    };
    run().map_err(crate::js_err)
}
