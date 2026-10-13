//! The routes `finder/*` of `rpc`: what the Pyodide of mzFinder served and `mzlab_feat::legacy` serves now (the same functions the `.wasm` of the feature `finder` calls).
//! `finder/dig` keeps the Dig in `FeatState` for `finder/detail`, `inclusion_csv`, `frag_tree` and `frag_tree_compare`. The text errors of `legacy` come back as `error.finder`
//! with the English text in `detail`.

use mzlab_core::{Error, Result};
use mzlab_feat::legacy;
use serde_json::Value;

use crate::engine::State;
use crate::rpc::{text, Answer};

impl State {
    fn finder_dig(&self, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let lc = legacy::series::lc_files(s("files")).map_err(user)?;
        let mut g = self.feat.lock().unwrap_or_else(|e| e.into_inner());
        let table = g
            .table
            .as_deref()
            .ok_or_else(|| Error::new("error.featState"))?;
        let d = legacy::dig(s("files"), s("parent"), table, s("registry"), |k| {
            self.open_idx(lc.get(k).map_or("", |f| f.name.as_str()))
                .map_err(|e| e.key.to_string())
        })
        .map_err(user)?;
        let out = legacy::out::summary(
            &d,
            if s("settings").is_empty() {
                "{}"
            } else {
                s("settings")
            },
            if s("timing").is_empty() {
                "{}"
            } else {
                s("timing")
            },
        );
        g.dig = Some(d);
        text(out)
    }

    fn finder_plan(&self, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let lc = legacy::series::lc_files(s("files")).map_err(user)?;
        let par = legacy::series::parent(s("parent")).map_err(user)?;
        let structure = par
            .smiles
            .as_deref()
            .and_then(|x| mzlab_feat::chem::grafo::analyse_json(x).ok())
            .unwrap_or_default();
        let mut o = mzlab_core::json::Out::default();
        o.raw("{\"lc\":[");
        for (i, f) in lc.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(&f.name);
        }
        o.raw("],\"minutes\":[");
        for (i, f) in lc.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).opt(f.time);
        }
        o.raw("],\"formula\":")
            .str(&par.formula())
            .raw(",\"structure\":")
            .str(&structure)
            .raw("}");
        text(o.0)
    }

    /// The answers about the Dig of the last `finder/dig`.
    fn finder_of_dig(&self, op: &str, p: &Value) -> Result<Answer> {
        let g = self.feat.lock().unwrap_or_else(|e| e.into_inner());
        let d = g
            .dig
            .as_ref()
            .ok_or_else(|| Error::new("error.featState"))?;
        let id = p["id"].as_u64().unwrap_or(0) as usize;
        text(match op {
            "finder/detail" => legacy::out::detail(d, id).map_err(user)?,
            "finder/inclusion_csv" => legacy::out::inclusion_csv(d, id),
            "finder/frag_tree" => legacy::fragtree::of_dig(d),
            _ => legacy::fragtree::COMPARE_NONE.to_string(),
        })
    }

    pub(crate) fn op_finder(&self, op: &str, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        match op {
            "finder/classify" => {
                let names: Vec<String> = p["names"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|v| v.as_str().unwrap_or("").to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                text(legacy::classify(&names, |n| {
                    self.open_idx(n)
                        .map_err(|_| "no index of the file: open it again".to_string())
                }))
            }
            "finder/plan" => self.finder_plan(p),
            "finder/transformations" => {
                let merged =
                    legacy::transf::merge(s("rules").as_bytes(), s("text")).map_err(user)?;
                text(String::from_utf8_lossy(&merged).into_owned())
            }
            "finder/dig" => self.finder_dig(p),
            "finder/detail"
            | "finder/inclusion_csv"
            | "finder/frag_tree"
            | "finder/frag_tree_compare" => self.finder_of_dig(op, p),
            "finder/formula_info" => text(legacy::formula_info(s("text"), s("adduct"))),
            "finder/identity_graph" => text(legacy::identity_graph(s("payload")).map_err(user)?),
            "finder/polarity_pairs" => text(legacy::polarity_pairs(s("payload")).map_err(user)?),
            "finder/origin_evidence" => {
                text(legacy::edge::origin_evidence(s("payload")).map_err(user)?)
            }
            _ => Err(Error::new("rpc.unknown")),
        }
    }
}

fn user(e: String) -> Error {
    Error::new("error.finder").with("detail", e)
}
