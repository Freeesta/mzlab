//! The text of `structure.reactivity@1`: `reatt/1` of the SMILES of the parent file (`mzlab_feat::parent::parent_smiles`) under the solvent, the conformers, the form of the
//! parent and the seed of the step. One function for `flow run` and the `.wasm` `chem` of the browser, so both write the same bytes.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};
use mzlab_feat::parent::parent_smiles;

use crate::react::reactivity_json;

/// The options the engine reads (`Opts::parse`), from the parameters the step resolved.
fn opts_json(params: &[(String, Value)]) -> String {
    let given = |k: &str| params.iter().find(|(n, _)| n == k);
    let mut o = Out::default();
    o.raw("{");
    let mut first = true;
    for k in ["solvent", "protomer"] {
        if let Some(s) = given(k).and_then(|(_, v)| v.str()) {
            o.raw(if first { "" } else { "," }).str(k).raw(":").str(s);
            first = false;
        }
    }
    for k in ["conformers", "seed"] {
        if let Some(n) = given(k).and_then(|(_, v)| v.f64()) {
            o.raw(if first { "" } else { "," })
                .str(k)
                .raw(":")
                .int(n as i64);
            first = false;
        }
    }
    o.raw("}");
    o.0
}

/// `reatt/1` text of the parent file under the parameters of the step; a file with no SMILES is `err.flow.position.smiles`, a bad SMILES the engine's error.
pub fn reactivity_text(params: &[(String, Value)], parent: &[u8]) -> Result<String> {
    let smiles = parent_smiles(parent);
    if smiles.is_empty() {
        return Err(Error::new("err.flow.position.smiles"));
    }
    reactivity_json(&smiles, &opts_json(params))
}
