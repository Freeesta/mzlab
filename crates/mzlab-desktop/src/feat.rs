//! mzFinder in the app: the routes `feat/*` of `rpc`, the native twin of `TP_Mine/files/feat-worker.js`. The steps are the ones the `.wasm` runs (`mzlab_feat_wasm::run`),
//! over the `.mzidx` of the index folder of the app (`idxbuild.rs`), so the `.mzfeat` has the same bytes as the one of the site. What a step leaves (the `.mzfeatf` of each file,
//! the table, the hypotheses, the Dig) stays in `FeatState`, because the page does not hold bytes of files here: it names the registered files and asks for the next step.
//! The routes `finder/*` are in `feat_finder.rs`. Errors are `{error_key, params}` like the rest of `rpc`.

use std::collections::HashMap;

use mzlab_core::budget::MemBudget;
use mzlab_core::idxread::Index;
use mzlab_core::json::Out;
use mzlab_core::{Error, Result};
use mzlab_feat::legacy::dig::Dig;
use mzlab_feat::legacy::lr::LrDig;
use mzlab_feat::store::Slice;
use mzlab_feat_wasm::run::{self, HypoRun};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::engine::State;
use crate::rpc::{map_file, text, Answer, MapAt};

impl State {
    /// The `.mzidx` of the registered mzML `name`.
    pub(crate) fn open_idx(&self, name: &str) -> Result<Index<MapAt>> {
        let (r, size) = map_file(&self.indexer()?.path(name)?)
            .map_err(|_| Error::new("err.engine.index").with("name", name))?;
        Index::open(r, size)
    }

    pub(crate) fn feat_lock(&self) -> std::sync::MutexGuard<'_, FeatState> {
        self.feat.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The `.mzfeatf` held for each of `names`, in that order.
    pub(crate) fn feat_stored(&self, names: &[String]) -> Result<Vec<Vec<u8>>> {
        let g = self.feat_lock();
        names
            .iter()
            .map(|n| {
                g.files
                    .get(n)
                    .cloned()
                    .ok_or_else(|| Error::new("error.featState"))
            })
            .collect()
    }

    fn feat_detect(&self, p: &Value) -> Result<Answer> {
        let name = p["name"].as_str().unwrap_or("");
        let mut ix = self.open_idx(name)?;
        let r = run::recipe_for(p["recipe"].as_str().unwrap_or(""), &mut ix)?;
        // The page asks for several files at once: the pool of the index builds (cores - 1, at most 4) bounds how many run together and leaves a core to the window.
        let bytes = self
            .indexer()?
            .pool
            .install(|| run::detect(&mut ix, name, &r))?;
        let a = written(&bytes);
        self.feat_lock().files.insert(name.to_string(), bytes);
        a
    }

    /// `feat/files`: the names (sorted JSON array) of the files that have an `.mzidx` in the index folder: the series the Pipeline panel can run.
    fn feat_files(&self) -> Result<Answer> {
        let b = self.indexer()?;
        let mut names: Vec<String> = crate::rpc::index_list(&b)
            .into_iter()
            .map(|(n, _)| n)
            .filter(|n| !n.ends_with(".cen"))
            .collect();
        names.sort();
        let mut o = Out::default();
        o.raw("[");
        for (i, n) in names.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.str(n);
        }
        o.raw("]");
        text(o.0)
    }

    /// `feat/preview {name, recipe}`: the estimates with their proofs, the histograms and three traces of one file (`preview_file` of the `.wasm`).
    fn feat_preview(&self, p: &Value) -> Result<Answer> {
        let name = p["name"].as_str().unwrap_or("");
        let recipe = p["recipe"].as_str().unwrap_or("");
        let mut ix = self.open_idx(name)?;
        let r = run::recipe_for(recipe, &mut ix)?;
        text(run::preview(&mut ix, &r, run::floor_given(recipe))?)
    }

    fn feat_align(&self, p: &Value) -> Result<Answer> {
        let ns = names(p);
        let first = ns.first().ok_or_else(|| Error::new("error.featState"))?;
        let r = run::recipe_for(
            p["recipe"].as_str().unwrap_or(""),
            &mut self.open_idx(first)?,
        )?;
        let mut src = self.feat_stored(&ns)?;
        let budget = MemBudget::mb(mzlab_core::budget::NATIVE_BUDGET_MB as f64);
        let mut out = Vec::new();
        run::align_to(&mut src, &budget, &r, |k| self.open_idx(&ns[k]), &mut out)?;
        let a = written(&out);
        let mut g = self.feat_lock();
        g.table = Some(out);
        g.hypo = None;
        g.dig = None;
        g.lr = None;
        a
    }

    fn feat_hypotheses(&self, p: &Value) -> Result<Answer> {
        let ns = names(p);
        let mut ix = ns
            .iter()
            .map(|n| self.open_idx(n))
            .collect::<Result<Vec<_>>>()?;
        let recipe = p["recipe"].as_str().unwrap_or("");
        let r = match ix.first_mut() {
            Some(first) => run::recipe_for(recipe, first),
            None => run::recipe_from_json(recipe),
        }?;
        let rt = Some(num(&p["parent_rt"])).filter(|t| t.is_finite());
        let mut src = self.feat_stored(&ns)?;
        let h = run::hypotheses_stored(
            &mut ix,
            &mut src,
            p["rules"].as_str().unwrap_or("").as_bytes(),
            p["structure"].as_str().unwrap_or(""),
            p["parent"].as_str().unwrap_or(""),
            rt,
            &r,
        )?;
        let mut o = Out::default();
        o.raw("{\"registry\":")
            .raw(&h.registry.to_json())
            .raw(",\"tried\":")
            .int(i64::from(h.result.tried))
            .raw(",\"found\":")
            .int(i64::from(h.result.found))
            .raw("}");
        self.feat_lock().hypo = Some(h);
        text(o.0)
    }

    fn feat_evidence(&self, p: &Value) -> Result<Answer> {
        let r = run::recipe_from_json(p["recipe"].as_str().unwrap_or(""))?;
        let minutes: Vec<f64> = p["minutes"]
            .as_array()
            .map(|a| a.iter().map(num).collect())
            .unwrap_or_default();
        let (pm, pr) = (num(&p["parent_mz"]), num(&p["parent_rt"]));
        let mut parent = (pm.is_finite() && pr.is_finite()).then_some((pm, pr));
        let mut g = self.feat_lock();
        let (Some(table), Some(h)) = (g.table.as_ref(), g.hypo.as_ref()) else {
            return Err(Error::new("error.featState"));
        };
        // `files` + `parent` (the page's JSON): the parent peak of the aligned table, as the site's worker does (`finder_parent`) before the proofs
        if let (Some(files), Some(par)) = (p["files"].as_str(), p["parent"].as_str()) {
            let m = mzlab_feat::legacy::series::parent(par)
                .map_err(crate::feat_finder::user)?
                .mz;
            parent = mzlab_feat::legacy::parent_peak(files, par, table)
                .map_err(crate::feat_finder::user)?
                .map(|x| (m, x.1));
        }
        let budget = MemBudget::mb(mzlab_core::budget::NATIVE_BUDGET_MB as f64);
        let mut out = Vec::new();
        run::evidence_to(
            &mut Slice(table),
            table.len() as u64,
            h,
            &minutes,
            parent,
            &r,
            &budget,
            &mut out,
        )?;
        let a = written(&out);
        g.table = Some(out);
        g.dig = None;
        g.lr = None;
        a
    }

    /// `feat/table`: the table held by the last `feat/align` or `feat/evidence`. `kind` = `view`, `report`, `qc`, `samples`, `export` (`export_kind`), `ms2`
    /// (`export_kind`, the indexes of `names`) as the Pipeline panel asks of `mzlab-feat-table-wasm`; `bytes` = a piece (`off`, `len`) of the `.mzfeat`.
    fn feat_table(&self, p: &Value) -> Result<Answer> {
        use mzlab_feat_table_wasm::core as t;
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let kind = s("kind");
        if kind == "samples" {
            return text(t::samples_json(s("files"))?);
        }
        let g = self.feat_lock();
        let table = g
            .table
            .as_deref()
            .ok_or_else(|| Error::new("error.featState"))?;
        if kind == "bytes" {
            let (off, len) = (
                p["off"].as_u64().unwrap_or(0) as usize,
                p["len"].as_u64().unwrap_or(0) as usize,
            );
            return Ok(Answer::Bytes(
                table
                    .get(off..off.saturating_add(len).min(table.len()))
                    .unwrap_or_default()
                    .to_vec(),
            ));
        }
        let (mut at, size) = (Slice(table), table.len() as u64);
        let held = sha_hex(table);
        // the page names the table it means by its SHA-256: another step of mzFinder may have replaced the one the app holds
        if !s("sha256").is_empty() && s("sha256") != held {
            return Err(Error::new("error.featState"));
        }
        let sha = held;
        let pol = p["polarity"].as_i64().unwrap_or(1) as i32;
        text(match kind {
            "view" => t::view_json(&mut at, size, s("samples"), s("spec"))?,
            "report" => t::report_json(&mut at, size, s("samples"), s("spec"), &sha)?,
            "qc" => t::qc_json(&mut at, size, s("samples"), s("spec"))?,
            "export" => t::table_file(
                &mut at,
                size,
                s("samples"),
                s("spec"),
                &sha,
                pol,
                s("export_kind"),
            )?,
            "ms2" => {
                let ns = names(p);
                t::ms2_file(
                    &mut at,
                    size,
                    s("samples"),
                    s("spec"),
                    &sha,
                    pol,
                    s("export_kind"),
                    |k| self.open_idx(ns.get(k).map_or("", |n| n.as_str())),
                )?
            }
            "row" => t::row_json(
                &mut at,
                size,
                s("samples"),
                p["id"].as_u64().unwrap_or(0) as u32,
            )?,
            "row_traces" => {
                let ns = names(p);
                t::row_traces_json(
                    &mut at,
                    size,
                    s("samples"),
                    p["id"].as_u64().unwrap_or(0) as u32,
                    p["ppm"].as_f64().unwrap_or(5.0),
                    |k| self.open_idx(ns.get(k).map_or("", |n| n.as_str())),
                )?
            }
            _ => return Err(Error::new("rpc.unknown")),
        })
    }

    /// The routes of mzFinder (`feat/*`, `finder/*`).
    pub(crate) fn op_feat(&self, op: &str, p: &Value) -> Result<Answer> {
        match op {
            "feat/detect" => self.feat_detect(p),
            "feat/align" => self.feat_align(p),
            "feat/untargeted/detect" => self.feat_unt_detect(p),
            "feat/untargeted/align" => self.feat_unt_align(p),
            "feat/formula_id" => self.feat_formula_id(p),
            "feat/hypotheses" => self.feat_hypotheses(p),
            "feat/evidence" => self.feat_evidence(p),
            "feat/table" => self.feat_table(p),
            "feat/version" => text(format!("\"{}\"", mzlab_feat::CRATE_VERSION)),
            "feat/preview" => self.feat_preview(p),
            "feat/files" => self.feat_files(),
            "feat/import" => self.op_import(p),
            "feat/attributio/annotate" => self.feat_attributio(p),
            "feat/attributio/review" => self.feat_attributio_review(p),
            "feat/attributio/export" => self.feat_attributio_export(p),
            "feat/grafo" => self.op_grafo(p),
            "feat/clear" => {
                *self.feat_lock() = FeatState::default();
                text("{}")
            }
            _ => self.op_finder(op, p),
        }
    }
}

pub(crate) fn sha_hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect::<String>()
}

/// `{"bytes", "sha256"}`: what a step that wrote bytes answers (the `.wasm` answers the same).
pub(crate) fn written(b: &[u8]) -> Result<Answer> {
    let mut o = Out::default();
    o.raw("{\"bytes\":")
        .int(b.len() as i64)
        .raw(",\"sha256\":")
        .str(&sha_hex(b))
        .raw("}");
    text(o.0)
}

pub(crate) fn names(p: &Value) -> Vec<String> {
    p["names"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// A number of the page; `null` and missing are NaN (a control has no time, no parent has no m/z).
fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

/// The results of the steps, by the name of the file (the `.mzfeatf`) or one only (the rest).
#[derive(Default)]
pub struct FeatState {
    pub(crate) files: HashMap<String, Vec<u8>>,
    pub(crate) table: Option<Vec<u8>>,
    pub(crate) hypo: Option<HypoRun>,
    pub(crate) dig: Option<Dig>,
    /// the Dig of a low-resolution series (`finder/lr_dig`); `dig` and `lr` are never both held, and `lr` holds the names of its columns
    pub(crate) lr: Option<(LrDig, Vec<String>)>,
    /// the MSn tree of the standard of the last `finder/dig` (none without an MSn file)
    pub(crate) msn: Option<crate::feat_msn::Msn>,
}
