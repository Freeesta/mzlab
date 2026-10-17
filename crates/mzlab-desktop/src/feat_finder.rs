//! The routes `finder/*` of `rpc`: what the Pyodide of mzFinder served and `mzlab_feat::legacy` serves now (the same functions the `.wasm` of the feature `finder` calls).
//! `finder/dig` keeps the Dig in `FeatState` for `finder/detail`, `inclusion_csv`, `frag_tree` and `frag_tree_compare`. The text errors of `legacy` come back as `error.finder`
//! with the English text in `detail`.

use mzlab_core::mzmlhead::Mzml;
use mzlab_core::{Error, Result};
use mzlab_feat::legacy;
use mzlab_feat::store::Slice;
use serde_json::Value;

use crate::engine::State;
use crate::feat::{names, FeatState};
use crate::feat_msn::Msn;
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
        // the network of mass differences reads the rules of the last hypotheses run, as the worker of the site does with `set_rules`
        let net = g
            .hypo
            .as_ref()
            .and_then(|h| h.rules().ok().flatten())
            .map_or("null".to_string(), |r| {
                legacy::network::mass_network(&d, &r)
            });
        let out = legacy::out::summary_with(
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
            &net,
        );
        // with an MSn file of the standard (`msn` = its name) the summary gets the tree, the calibration and the localisation of the rows
        let (out, msn) = match s("msn") {
            "" => (out, None),
            name => {
                let mut m = Msn::new(&mut self.open_idx(name)?, s("parent")).map_err(user)?;
                let out = m
                    .summary(&out, |id| legacy::out::detail(&d, id as i64))
                    .map_err(user)?;
                (out, Some(m))
            }
        };
        g.dig = Some(d);
        g.lr = None;
        g.msn = msn;
        text(out)
    }

    /// `finder/lr_plan`: what the page needs before the pipeline of a low-resolution series (`{rust, lc, minutes, formula}`).
    fn finder_lr_plan(&self, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        text(legacy::lr::plan::plan_json(s("files"), s("parent")).map_err(user)?)
    }

    /// `finder/lr_dig`: the Dig of a low-resolution series over the table of the last `feat/align`; it stays for `finder/detail`, `frag_tree` and `frag_tree_compare`.
    fn finder_lr_dig(&self, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let mut g = self.feat.lock().unwrap_or_else(|e| e.into_inner());
        let table = g
            .table
            .as_deref()
            .ok_or_else(|| Error::new("error.featState"))?;
        let settings = if s("settings").is_empty() {
            "{}"
        } else {
            s("settings")
        };
        let lc: Vec<String> =
            legacy::lr::plan::columns(&legacy::lr::plan::files(s("files")).map_err(user)?)
                .into_iter()
                .map(|f| f.name)
                .collect();
        let mut d = legacy::lr::dig(
            s("files"),
            s("parent"),
            settings,
            table,
            s("registry"),
            &mut |k| {
                self.open_idx(lc.get(k).map_or("", |n| n.as_str()))
                    .map_err(|e| e.key.to_string())
            },
        )
        .map_err(user)?;
        let names: Vec<String> = (d.lc.iter().map(|f| f.name.clone()))
            .chain(
                d.files
                    .iter()
                    .filter(|f| f.kind == "ms2")
                    .map(|f| f.name.clone()),
            )
            .collect();
        d.isf = legacy::lr::isf::evaluate(&d, &mut |k| {
            self.open_idx(names.get(k).map_or("", |n| n.as_str()))
                .map_err(|e| e.key.to_string())
        })
        .map_err(user)?;
        // the matches of the MS2 experiments to the rows (`ms2_links/1`), as the example `finder_lr` and the worker of the site
        legacy::lr::ms2link::ms2_link(&mut d, &[], &mut |k| {
            self.open_idx(names.get(k).map_or("", |n| n.as_str()))
                .map_err(|e| e.key.to_string())
        })
        .map_err(user)?;
        let timing = if s("timing").is_empty() {
            "{}"
        } else {
            s("timing")
        };
        let out = legacy::lr::summary::summary(&d, timing);
        g.lr = Some((d, names));
        g.dig = None;
        g.msn = None;
        text(out)
    }

    /// `finder/lr_mrm`: the integration table of the MRM files of the series (`d.mrm` of `finder/lr_dig`, `"null"` without any), read from the registered mzML.
    fn finder_lr_mrm(&self, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let fs = legacy::lr::plan::files(s("files")).map_err(user)?;
        let mut out = Vec::new();
        for f in fs.iter().filter(|f| f.kind == "mrm") {
            let mut guard = self.eng.lock().unwrap_or_else(|e| e.into_inner());
            let eng = &mut *guard;
            let slot = self.open_lr(eng, &f.name)?; // a native sample reads its chromatograms through its reader
            let (rd, size) = self.src_named(&mut eng.nat, slot, &f.name)?;
            let mut m = Mzml::new(rd, size);
            // the scans of an MRM file are few; where they end tells where the chromatograms start
            let end = self
                .open_idx(&f.name)
                .ok()
                .and_then(|ix| ix.scans.last().map(|x| x.end))
                .unwrap_or(0);
            out.push(legacy::lr::mrm::file_from_mzml(f, &mut m, end)?);
        }
        let tol = p["rt_tol"].as_f64().filter(|t| *t > 0.0).unwrap_or(0.25);
        text(legacy::lr::mrm::analyse(&out, tol).unwrap_or_else(|| "null".into()))
    }

    /// `finder/estimate`: the settings of the low-resolution Dig proposed from the data, as the worker of the site writes them: `{da, rt_tol, dead, fwhm, drift?}` from the preview
    /// of the first file (profile `lr`); with the m/z of the parent and two files or more `rt_tol` is the larger of 2 x FWHM and the drift of the parent. `"null"` without a preview.
    fn finder_estimate(&self, p: &Value) -> Result<Answer> {
        let ns = names(p);
        let Some(first) = ns.first() else {
            return text("null".to_string());
        };
        let mut ix = self.open_idx(first)?;
        let r = mzlab_feat_wasm::run::recipe_for("", &mut ix)?;
        let pv: Value = serde_json::from_str(&mzlab_feat_wasm::run::preview(&mut ix, &r, false)?)
            .map_err(|_| Error::new("error.badRecipe"))?;
        let mut out = serde_json::json!({ "da": pv["da"], "rt_tol": pv["rt_tol"], "dead": pv["dead"], "fwhm": pv["fwhm"] });
        let mz = p["mz"].as_f64().unwrap_or(0.0);
        if ns.len() > 1 && mz > 0.0 {
            let mut all = Vec::new();
            for n in &ns {
                let mut ix = self.open_idx(n)?;
                let r = mzlab_feat_wasm::run::recipe_for("", &mut ix)?;
                all.push(
                    mzlab_feat::store::read_file(&mzlab_feat_wasm::run::detect(&mut ix, n, &r)?)?.0,
                );
            }
            let refs: Vec<&mzlab_feat::table::FileFeatures> = all.iter().collect();
            let d = mzlab_feat::params_drift::parent_drift(&refs, mz);
            let base = pv["rt_tol"].as_f64().unwrap_or(f64::NAN);
            let dj: Value = serde_json::from_str(&mzlab_feat::params_drift::drift_json(&d, base))
                .unwrap_or(Value::Null);
            if !dj["drift"].is_null() {
                out["drift"] = serde_json::json!({ "value": dj["drift"], "n": dj["n"] });
                out["rt_tol"] = dj["rt_tol"].clone();
            }
        }
        text(out.to_string())
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

    /// The answers about the Dig of the last `finder/dig` (with its MSn tree, if it had one).
    fn finder_of_dig(&self, op: &str, p: &Value) -> Result<Answer> {
        let mut g = self.feat.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((d, names)) = g.lr.as_ref() {
            return if op == "finder/msn_tree" {
                text("null".to_string())
            } else {
                self.finder_of_lr(op, p, d, names)
            };
        }
        let FeatState { dig, msn, .. } = &mut *g;
        let d = dig.as_ref().ok_or_else(|| Error::new("error.featState"))?;
        let id = p["id"].as_u64().unwrap_or(0) as usize;
        let sid = p["id"].as_i64().unwrap_or(0);
        text(match (op, msn.as_mut()) {
            ("finder/msn_tree", m) => m.map_or("null".to_string(), |m| m.msn_tree()),
            ("finder/detail", None) => legacy::out::detail(d, sid).map_err(user)?,
            ("finder/detail", Some(m)) => m.detail(&legacy::out::detail(d, sid).map_err(user)?),
            ("finder/inclusion_csv", _) => legacy::out::inclusion_csv(d, id),
            ("finder/frag_tree", None) => legacy::fragtree::of_dig(d),
            ("finder/frag_tree", Some(m)) => m.frag_tree(),
            (_, None) => legacy::fragtree::COMPARE_NONE.to_string(),
            (_, Some(m)) => {
                // "candidate not found", and the localisation the comparison uses
                m.detail(&legacy::out::detail(d, sid).map_err(user)?);
                m.frag_tree_compare(id as u32)
            }
        })
    }

    /// The answers about the low-resolution Dig of the last `finder/lr_dig` (`detail` reads the XICs from the `.mzidx` of its columns).
    fn finder_of_lr(
        &self,
        op: &str,
        p: &Value,
        d: &legacy::lr::LrDig,
        names: &[String],
    ) -> Result<Answer> {
        let id = p["id"].as_i64().unwrap_or(0);
        let mut open = |k: usize| {
            self.open_idx(names.get(k).map_or("", |n| n.as_str()))
                .map_err(|e| e.key.to_string())
        };
        text(match op {
            "finder/detail" => legacy::lr::detail::detail(d, id, &mut open).map_err(user)?,
            "finder/frag_tree" => legacy::lr::tree::frag_tree(d, &mut open).map_err(user)?,
            "finder/frag_tree_compare" => {
                legacy::lr::tree::frag_tree_compare(d, id, &mut open).map_err(user)?
            }
            _ => return Err(Error::new("rpc.unknown")),
        })
    }

    /// `finder/tp_table`: the `tptab/1` of the rows `rows` of the table of the last `finder/dig` of a high-resolution series (`samples` = its `campioni/1`), as the `.wasm` `table` writes it.
    fn finder_tp_table(&self, p: &Value) -> Result<Answer> {
        let g = self.feat.lock().unwrap_or_else(|e| e.into_inner());
        let table = match (g.dig.as_ref(), g.table.as_deref()) {
            (Some(_), Some(t)) => t,
            _ => return text("null".to_string()),
        };
        let s = |k: &str| p[k].as_str().unwrap_or("");
        // with `campioni` (the `campioni/2` of the files) the answer carries `gruppi` for two or more conditions; `infos` = the rows' best hypothesis
        if !s("campioni").is_empty() {
            return text(mzlab_feat_table_wasm::core::tp_table_cond_json(
                &mut Slice(table),
                table.len() as u64,
                s("samples"),
                s("rows"),
                s("campioni"),
                s("infos"),
            )?);
        }
        text(mzlab_feat_table_wasm::core::tp_table_json(
            &mut Slice(table),
            table.len() as u64,
            s("samples"),
            s("rows"),
        )?)
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
            "finder/tp_table" => self.finder_tp_table(p),
            "finder/campioni" => text(mzlab_core::campioni::route(s("cop"), s("input"))?),
            "finder/transf_rows" => text(legacy::transf::describe(s("text"))),
            "finder/transformations" => {
                let merged =
                    legacy::transf::merge(s("rules").as_bytes(), s("text")).map_err(user)?;
                text(String::from_utf8_lossy(&merged).into_owned())
            }
            "finder/dig" => self.finder_dig(p),
            "finder/lr_plan" => self.finder_lr_plan(p),
            "finder/lr_dig" => self.finder_lr_dig(p),
            "finder/lr_mrm" => self.finder_lr_mrm(p),
            "finder/estimate" => self.finder_estimate(p),
            "finder/detail"
            | "finder/inclusion_csv"
            | "finder/msn_tree"
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

pub(crate) fn user(e: String) -> Error {
    Error::new("error.finder").with("detail", e)
}
