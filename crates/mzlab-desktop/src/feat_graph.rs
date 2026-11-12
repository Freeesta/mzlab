//! The route `feat/grafo` of `rpc`: the structure graph of `mzlab_feat::chem` (the same functions as the `.wasm` `grafo`). `{op: "feat/grafo", kind: "analyse", smiles}` gives the
//! `grafo/1` of a SMILES, `kind: "fragments"` (`molfile`, `a`, `b`) the cuts of a molfile (`frag/1`) and `kind: "partial"` (`molfile`, `delta`) a partial structure (`parz/1`).
//! `kind: "positions"` (`smiles`, `transform`, `peaks` as JSON text, `tol`, `ppm`, `positive`, optional `reatt` as the `reatt/1` text of the parent) gives the `posiz/1`
//! of a product (the classes of positions its MS2 supports; with `reatt`, also `prior`).
//! `kind: "labels"` (`request` as a JSON object or text) gives the `etich/1` of the structures of a drawing (`mzlab-cli mol-label`); `kind: "cut"`
//! (`request` the same way) the `cut/1` of a cut across bonds (`mzlab-cli mol-cut`); `kind: "sub"` (`request` the same way) the `sub/1` of a
//! substructure search (`mzlab-cli mol-search --sub`); `kind: "check"` the `check/1` of a drawing (`mzlab-cli mol-check`).
//! `kind: "reactivity"` (`smiles`, `opts` as a JSON object or text) gives the `reatt/1` of `mzlab_chem::reactivity_json` (the same function as the `.wasm` `chem`).
//! An error keeps its key (`error.grafo.ring`, `error.grafo.delta.neg`, ...) as the `.wasm` throws it.

use mzlab_core::{Error, Result};
use mzlab_feat::chem::{check, cut, fragments, graph, labels, positions, substructure};
use serde_json::Value;

use crate::engine::State;
use crate::rpc::{text, Answer};

impl State {
    pub(crate) fn op_graph(&self, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let n = |k: &str| p[k].as_i64().map_or(-1, |v| v as i32);
        match s("kind") {
            "analyse" => text(graph::analyse_json(s("smiles"))?),
            "fragments" => text(fragments::fragments_json(s("molfile"), n("a"), n("b"))?),
            "partial" => text(fragments::partial_json(s("molfile"), s("delta"))?),
            "positions" => text(positions::positions_prior_json(
                s("smiles"),
                s("transform"),
                s("peaks"),
                p["tol"].as_f64().unwrap_or(0.5),
                p["ppm"].as_bool().unwrap_or(false),
                p["positive"].as_bool().unwrap_or(true),
                s("reatt"),
            )?),
            "labels" | "cut" | "sub" | "check" => {
                let req = if p["request"].is_object() {
                    p["request"].to_string()
                } else {
                    s("request").to_string()
                };
                text(match s("kind") {
                    "cut" => cut::cut_json(&req)?,
                    "sub" => substructure::search_json(&req)?,
                    "check" => check::check_json(&req)?,
                    _ => labels::labels_json(&req)?,
                })
            }
            "reactivity" => {
                let opts = if p["opts"].is_object() {
                    p["opts"].to_string()
                } else {
                    s("opts").to_string()
                };
                text(mzlab_chem::reactivity_json(s("smiles"), &opts)?)
            }
            _ => Err(Error::new("rpc.unknown")),
        }
    }
}
