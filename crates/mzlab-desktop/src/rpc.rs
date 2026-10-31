//! The native twin of `rust-worker.js` + `mzlab-wasm`: the operations the page asks of its engine (`rpc` over `mzlab://`), answered from memory-mapped files and
//! the same `mzlab-core` code the site runs as WebAssembly. A file is read through `MapAt` (a `ReadAt` over a map), an index is the `.mzidx` of the index folder
//! of the app (`idxbuild.rs`), the files and the notebook of the student are in `store.rs`. An answer is a text (the JSON the route gives) or bytes (`scanbin`);
//! an empty text means «not answered here» exactly as in the worker. Errors are `{error_key, params}` with status 400.

use std::collections::HashMap;
use std::io::{Seek, SeekFrom, Write};
use std::sync::Arc;

use mzlab_core::gradient::Gradient;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
use mzlab_core::labmethods::Lab;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::routes::{Query, Routes};
use mzlab_core::{Error, Result};
use serde_json::Value;

use crate::engine::State;
use crate::mapped::Mapped;
use crate::nativi::{Nativi, Src};

/// The slots of the files of the page's list (not of the session) start here, as in `rust-worker.js`.
pub(crate) const LR_BASE: u32 = 1_000_000;

impl State {
    /// The number of a file of the disk (the same number for the same path); the size is the one the file has now, so a file rewritten at the same path
    /// is still found by name and size.
    pub(crate) fn pick_path(&self, path: &std::path::Path) -> u32 {
        {
            let mut g = self.lock_inner();
            if let Some(id) = g.picked.iter().filter(|(_, p)| p.path == path).map(|(id, _)| *id).max() {
                if let (Ok(m), Some(p)) = (std::fs::metadata(path), g.picked.get_mut(&id)) {
                    p.size = m.len();
                }
                return id;
            }
        }
        self.pick(path.to_path_buf()).map(|(id, _)| id).unwrap_or(0)
    }

    /// A file that reaches the app as bytes (a file dragged into the page, not picked in the system dialog): pieces of `body` written at `off` into the folder
    /// of the files made by the app. Returns the number of the file when the last piece is in.
    pub fn upload_chunk(&self, name: &str, off: u64, total: u64, body: &[u8]) -> Result<Option<u32>> {
        let name = crate::store::clean(name)?;
        if body.is_empty() && total > 0 {
            return Err(Error::new("err.file.unknown").with("detail", "empty upload"));      // a webview that drops the body of a POST must not look like a file that was received
        }
        let dir = self.store.lock().unwrap_or_else(|e| e.into_inner()).own_dir()?;
        let part = dir.join(format!("{name}.part"));
        let mut f = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(&part).map_err(|e| io_error(&e))?;
        f.seek(SeekFrom::Start(off)).map_err(|e| io_error(&e))?;
        f.write_all(body).map_err(|e| io_error(&e))?;
        if off + body.len() as u64 >= total {
            f.set_len(total).map_err(|e| io_error(&e))?;
            drop(f);
            let dest = dir.join(name);
            std::fs::rename(&part, &dest).map_err(|e| io_error(&e))?;
            return Ok(Some(self.pick_path(&dest)));
        }
        Ok(None)
    }

    /// True for an mzML that the app itself wrote in the temporary folder (a converted `.raw`).
    pub(crate) fn is_temp(&self, path: &std::path::Path) -> bool {
        self.lock_inner().temps.iter().any(|t| t == path)
    }

    /// The mzML of a file picked or made by the app, by its number.
    pub(crate) fn mzml_of(&self, fid: u32) -> Result<(MapAt, u64)> {
        map_file(&self.picked(fid)?.path)
    }

    /// The mzML of a registered file, by name.
    pub(crate) fn mzml_named(&self, name: &str) -> Result<(MapAt, u64)> {
        let e = self.store.lock().unwrap_or_else(|e| e.into_inner()).get(name).ok_or_else(|| Error::new("err.session.fileNotFound").with("name", name))?;
        map_file(&e.path)
    }

    /// What the routes read for `slot`: the reader of a native sample, else the mzML of the picked file `fid`.
    pub(crate) fn src_slot<'a>(&self, nat: &'a mut Nativi, slot: u32, fid: u32) -> Result<(Src<'a>, u64)> {
        if nat.is_native(slot) {
            return nat.src(slot)?.ok_or_else(bad_index);
        }
        let (m, n) = self.mzml_of(fid)?;
        Ok((Src::Map(m), n))
    }

    /// The same for a registered file, by name (`slot` is its slot in the page's list).
    pub(crate) fn src_named<'a>(&self, nat: &'a mut Nativi, slot: u32, name: &str) -> Result<(Src<'a>, u64)> {
        if nat.is_native(slot) {
            return nat.src(slot)?.ok_or_else(bad_index);
        }
        let (m, n) = self.mzml_named(name)?;
        Ok((Src::Map(m), n))
    }

    /// Attaches `slot` to the reader of the sample `name` when it is a native one (the store says), else lets it go.
    pub(crate) fn attach_named(&self, eng: &mut Eng, slot: u32, name: &str) {
        match self.store.lock().unwrap_or_else(|e| e.into_inner()).get(name).and_then(|e| e.nat) {
            Some(nat) => eng.nat.attach(slot, &nat),
            None => eng.nat.forget(slot),
        }
    }

    /// Opens the index of `name` in the slot of the page's list. `err.engine.index` when there is none.
    pub(crate) fn open_lr(&self, eng: &mut Eng, name: &str) -> Result<u32> {
        let slot = match eng.lr_slot.get(name) {
            Some(s) => *s,
            None => {
                eng.next_lr += 1;
                eng.lr_slot.insert(name.to_string(), LR_BASE + eng.next_lr);
                LR_BASE + eng.next_lr
            }
        };
        if !eng.idx.contains_key(&slot) {
            eng.idx.insert(slot, self.open_index_file(name).map_err(|_| Error::new("err.engine.index").with("name", name))?);
        }
        self.attach_named(eng, slot, name);
        Ok(slot)
    }

    pub(crate) fn open_index_file(&self, name: &str) -> Result<Routes<MapAt>> {
        let (r, size) = map_file(&self.indexer()?.path(name)?)?;
        Ok(Routes::new(Index::open(r, size)?))
    }

    /// Closes the open index of the registered file `name`, if any.
    pub(crate) fn drop_lr_index(&self, name: &str) {
        let mut eng = self.eng.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(slot) = eng.lr_slot.get(name).copied() {
            eng.idx.remove(&slot);
        }
    }

    /// `lr_index`: the scan table and the index of a registered mzML (both on a thread of the build pool), then the index is opened again on the next request.
    pub(crate) fn lr_index(&self, name: &str) -> Result<Answer> {
        let e = self.store.lock().unwrap_or_else(|e| e.into_inner()).get(name).ok_or_else(|| Error::new("err.session.fileNotFound").with("name", name))?;
        let t0 = std::time::Instant::now();
        // the mapped old index goes before the build replaces its file (a mapped file cannot be replaced on Windows); the next request opens the new one
        self.drop_lr_index(name);
        let mut missing = 0;
        if let Some(nat) = &e.nat {
            self.indexer()?.build_nat(nat, name)?;
        } else {
            missing = self.indexer()?.build(&e.path, name, None)?.missing;
        }
        self.drop_lr_index(name);
        // missing: the spectra the file declares beyond the ones it holds (the row of the loading warns, as on the site)
        text(format!("{{\"ms\":{},\"missing\":{missing}}}", t0.elapsed().as_millis()))
    }

    #[allow(unused_variables)]
    fn op_index(&self, eng: &mut Eng, op: &str, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let u = |k: &str| p[k].as_u64().unwrap_or(0) as u32;
        let f = |k: &str| p[k].as_f64().unwrap_or(0.0);
        let q = || Query::parse(s("query"));
        match op {
            "idxopen" => {
                let mut r = self.open_index_file(s("name")).map_err(|_| Error::new("error.noIndex"))?;
                r.pol = [1, -1].contains(&p["pol"].as_i64().unwrap_or(0)).then(|| p["pol"].as_i64().unwrap_or(0) as i8);      // as `set_polarity` of the wasm: 0 = every section
                let out = format!("{{\"scans\":{},\"hr1\":{}}}", r.idx.scans.len(), r.idx.hr1());
                eng.idx.insert(u("slot"), r);
                self.attach_named(eng, u("slot"), s("name"));
                text(out)
            }
            "route" => match eng.idx.get_mut(&u("slot")) {
                Some(r) => opt(r.handle(s("path"), &q())?),
                None => text(""),
            },
            "scaninfo" => match eng.idx.get(&u("slot")) {
                Some(r) if r.has_scans() => text(r.scaninfo(f("sid") as usize, &mut self.src_slot(&mut eng.nat, u("slot"), u("fid"))?.0)?),
                _ => text(""),
            },
            "mzmlroute" => match eng.idx.get_mut(&u("slot")) {
                Some(r) => opt(r.mzml_route(s("path"), &q(), &mut self.src_slot(&mut eng.nat, u("slot"), u("fid"))?.0)?),
                None => text(""),
            },
            "scanbin" => match eng.idx.get_mut(&u("slot")) {
                Some(r) => Ok(Answer::Bytes(r.scanbin_route(&q(), &mut self.src_slot(&mut eng.nat, u("slot"), u("fid"))?.0)?.unwrap_or_default())),
                None => Ok(Answer::Bytes(Vec::new())),
            },
            "specexact" => match eng.idx.get_mut(&u("slot")) {
                Some(r) => opt(r.spectrum_exact(&q(), &mut self.src_slot(&mut eng.nat, u("slot"), u("fid"))?.0)?),
                None => text(""),
            },
            "specbg" => {
                let slot = u("slot");
                let Some(mut main) = eng.idx.remove(&slot) else { return text("") };
                let out = (|| -> Result<Option<String>> {
                    let Some(req) = main.diff_request(&q())? else { return Ok(None) };
                    let k = req.bg_k() as u32;
                    if k == slot {
                        return mzlab_core::specdiff::spectrum_diff(&mut main, None, &req);
                    }
                    eng.idx.get_mut(&k).map_or(Ok(None), |bg| mzlab_core::specdiff::spectrum_diff(&mut main, Some(bg), &req))
                })();
                eng.idx.insert(slot, main);
                opt(out?)
            }
            "mrm" => {
                let r = eng.idx.get_mut(&u("slot")).ok_or_else(bad_index)?;
                let end = r.idx.scans.last().map_or(0, |s| s.end);
                let (rd, size) = self.src_slot(&mut eng.nat, u("slot"), u("fid"))?;
                let mut m = Mzml::new(rd, size);
                let chroms = m.chromatograms(end)?;
                text(mzlab_core::fileroutes::mrm_transitions(&mut m, &chroms)?)
            }
            "mrmchrom" => {
                let r = eng.idx.get_mut(&u("slot")).ok_or_else(bad_index)?;
                let end = r.idx.scans.last().map_or(0, |s| s.end);
                let (rd, size) = self.src_slot(&mut eng.nat, u("slot"), u("fid"))?;
                let mut m = Mzml::new(rd, size);
                let chroms = m.chromatograms(end)?;
                text(mzlab_core::itemroutes::mrm_chrom(&mut m, &chroms, if s("kind").is_empty() { "tic" } else { s("kind") })?)
            }
            "msngrafo" => self.op_msngrafo(eng, p),
            "masscal" => self.op_masscal(p),
            "canali" => self.op_canali(eng, p),
            "fileinfo" => {
                let r = eng.idx.get_mut(&u("slot")).ok_or_else(bad_index)?;
                let (rd, size) = self.src_slot(&mut eng.nat, u("slot"), u("fid"))?;
                let mut m = Mzml::new(rd, size);
                let head = m.info_head()?;
                text(mzlab_core::fileroutes::fileinfo(r, &mut m, s("name"), &head)?)
            }
            _ => Err(Error::new("rpc.unknown")),
        }
    }

    /// api/canali: the channels of the scans plus the SRM/SIC chromatograms of the mzML (as `Engine::canali` of the wasm).
    fn op_canali(&self, eng: &mut Eng, p: &Value) -> Result<Answer> {
        let slot = p["slot"].as_u64().unwrap_or(0) as u32;
        let r = eng.idx.get_mut(&slot).ok_or_else(bad_index)?;
        let end = r.idx.scans.iter().map(|s| s.end).max().unwrap_or(0);
        let (rd, size) = self.src_slot(&mut eng.nat, slot, p["fid"].as_u64().unwrap_or(0) as u32)?;
        let mut m = Mzml::new(rd, size);
        let chroms = m.chromatograms(end)?;
        let facts = mzlab_core::iteminfo::facts_from(&mut m, end, chroms.len() as i64, true)?;
        r.pon_cromatogrammi(p["name"].as_str().unwrap_or(""), &chroms, &facts);
        text(r.canali().json())
    }

    #[allow(unused_variables)]
    fn op_session(&self, eng: &mut Eng, op: &str, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let u = |k: &str| p[k].as_u64().unwrap_or(0) as u32;
        let f = |k: &str| p[k].as_f64().unwrap_or(0.0);
        let q = || Query::parse(s("query"));
        match op {
            "origin" => {
                use mzlab_core::originroute::{origin_route, Ask};
                let metas = session_items(s("items"))?;
                let qq = q();
                let (Some(mz), Some(parent)) = (qq.num("mz")?, qq.num("parent")?) else { return text("") };
                let ask = Ask { k: p["k"].as_i64().unwrap_or(-1), mz, parent, rt0: qq.num("rt0")?, rt1: qq.num("rt1")?, formula: qq.some("formula") };
                match origin_route(&metas, &mut eng.idx, &ask)? {
                    Some(rep) => text(mzlab_core::jsonb::write(&rep)),
                    None => text(""),
                }
            }
            "subxic" => match eng.idx.get_mut(&u("slot")) {
                Some(r) => opt(mzlab_core::subxic::subxic(r, p["k"].as_i64().unwrap_or(-1), &q())?),
                None => text(""),
            },
            "map" => {
                let metas = session_items(s("items"))?;
                opt(mzlab_core::maproute::map_route(s("path"), &metas, &mut eng.idx, &q())?)
            }
            "reset" => {
                eng.idx.retain(|k, _| *k >= LR_BASE);
                eng.nat.forget_below(LR_BASE);
                text("{}")
            }

            // ---- the files of the page's list, the session and the notebook
            "idxxic" => match eng.idx.get_mut(&u("slot")) {
                Some(r) => text(r.xic(f("mz"), f("tol"), u("level") as u8)?),
                _ => text(""),
            },
            "idxxicbin" => match eng.idx.get_mut(&u("slot")) {
                Some(r) => {
                    let mut out = vec![0.0; 2 * r.xic_len(u("level") as u8)];
                    let n = r.xic_into(f("mz"), f("tol"), u("level") as u8, &mut out)?;
                    out.truncate(2 * n);
                    Ok(Answer::Bytes(out.iter().flat_map(|v| v.to_le_bytes()).collect()))
                }
                _ => Ok(Answer::Bytes(Vec::new())),
            },
            "close" => {
                eng.idx.remove(&u("slot"));
                eng.nat.forget(u("slot"));
                text("{}")
            }
            _ => Err(Error::new("rpc.unknown")),
        }
    }

    #[allow(unused_variables)]
    fn op_diff(&self, eng: &mut Eng, op: &str, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let q = || Query::parse(s("query"));
        match op {
            "diff" => {
                use mzlab_core::diffroute::{plan, Tr};
                let pl = match plan(s("path"), &q()) {
                    None => return text(""),
                    Some(Err(e)) => return text(format!("E{e}")),
                    Some(Ok(pl)) => pl,
                };
                let mut get = |k: u32| -> Result<Option<Tr>> {
                    match eng.idx.get_mut(&k) {
                        None => Ok(None),
                        Some(r) => pl.fetch(k, r),
                    }
                };
                let Some(reference) = get(pl.o.reference)? else { return text("") };
                let mut files = Vec::new();
                for &k in &pl.ks {
                    match get(k)? {
                        Some(t) => files.push(t),
                        None => return text(""),
                    }
                }
                text(pl.finish(&reference, &files))
            }
            "qaqc" => {
                let pl = match mzlab_core::qaqc::plan(&q()) {
                    None => return text(""),
                    Some(Err(e)) => return text(format!("E{e}")),
                    Some(Ok(pl)) => pl,
                };
                let mut rows = Vec::new();
                for &k in &pl.ks {
                    let Some(r) = eng.idx.get_mut(&k) else { return text("") };
                    match pl.measure(k, r)? {
                        Some(row) => rows.push(row),
                        None => return text(""),
                    }
                }
                text(pl.finish(&rows))
            }
            "mrmdiff" => match mzlab_core::diffroute::mrm_diff(s("files"), s("ref"), &q()) {
                None => text(""),
                Some(Ok(j)) => text(j),
                Some(Err(j)) => text(format!("E{j}")),
            },
            _ => Err(Error::new("rpc.unknown")),
        }
    }

    /// One operation of the page's engine. `p` holds the parameters (a file of the page is its number `fid`), `body` the bytes of a POST.
    pub fn rpc(&self, op: &str, p: &Value, body: &[u8]) -> Result<Answer> {
        let _ = body;
        if op == "lr_index" {
            return self.lr_index(p["name"].as_str().unwrap_or(""));                 // the build takes its time: it does not hold the engine
        }
        if op == "lr_clear" {
            self.warm_forget(None);                                                         // the indexes built ahead go with the session
        }
        match op {
            "lr_warm" => return self.lr_warm(p),                                            // the index of a file of the pointed folder, built ahead (`rpc_warm.rs`): it does not hold the engine
            "lr_warm_claim" => return self.lr_warm_claim(p),
            "natsamples" => return self.nat_samples(p),                                     // reading a container takes its time: it does not hold the engine either
            "nat2mzml" => return self.nat_to_mzml(p),
            "natsrm" => return self.nat_srm(p),                                             // the traces of a sample for mzFinder (no mzML)
            "mzml2cen" => return self.mzml_to_cen(p),                                       // a centroid copy of a profile mzML: long, it does not hold the engine
            "lr_index_cen" => return self.lr_index_cen(p["name"].as_str().unwrap_or("")),   // the index of a sample as centroids, on a thread of the build pool
            "conv_add" => return self.conv_add(p),
            "conv_list" => return self.conv_list(p),
            "conv_cancel" => return self.conv_cancel(p),
            "flow/read" => return self.flow_read(p),
            "flow/liste" => return text(mzlab_feat::liste::route(&p.to_string())?),     // checks a list of the user and gives its SHA-256 (`liste::route`)
            "flow/line" => return text(mzlab_core::flow::line_route(&p.to_string(), Some(&mzlab_cli::flow::steps()))?),   // the short line of a plan, or the plan with its --set applied
            "flow/progress" => return self.flow_progress(p),
            "flow/report" => return self.flow_report(p),
            "flow/reuse" => return self.flow_reuse(p),                                   // steps the cache would give back, before Run (`flow_reuse.rs`)
            "flow/run" => return self.flow_run(p),                                      // a whole plan: it does not hold the engine
            _ if op.starts_with("flow/live/") => return self.flow_live(&op[10..], p),   // a plan that takes changes while it runs (`flow_live.rs`)
            _ => {}
        }
        if op.starts_with("llm/") {
            return self.op_llm(op, p);                                                  // the assistant: network on a thread of its own, the key never leaves Rust
        }
        if op.starts_with("feat/") || op.starts_with("finder/") {
            return self.op_feat(op, p);                                              // mzFinder: long jobs of their own, they do not hold the engine
        }
        let mut guard = self.eng.lock().unwrap_or_else(|e| e.into_inner());
        let eng = &mut *guard;
        let tries: [fn(&State, &mut Eng, &str, &Value) -> Result<Answer>; 6] = [State::op_index, State::op_session, State::op_diff, State::op_files, State::op_misc, State::op_lab];
        for t in tries {
            match t(self, eng, op, p) {
                Err(e) if e.key == "rpc.unknown" => {}
                r => return r,
            }
        }
        Err(Error::new("err.file.raw").with("detail", format!("unknown operation {op}")))
    }
}

pub(crate) fn io_error(e: &std::io::Error) -> Error {
    Error::new("err.file.raw").with("detail", e.to_string())
}

pub(crate) fn bad_index() -> Error {
    Error::new("error.badIndex")
}

pub(crate) fn text(t: impl Into<String>) -> Result<Answer> {
    Ok(Answer::Text(t.into()))
}

pub(crate) fn opt(a: Option<String>) -> Result<Answer> {
    text(a.unwrap_or_default())
}

pub(crate) fn map_file(path: &std::path::Path) -> Result<(MapAt, u64)> {
    let m = Arc::new(Mapped::open(path).map_err(|e| io_error(&e))?);
    let n = m.bytes().len() as u64;
    Ok((MapAt(m), n))
}

/// The indexes of the index folder: `(name of the mzML, size)`.
pub(crate) fn index_list(b: &crate::idxbuild::IdxBuilder) -> Vec<(String, u64)> {
    let Some(dir) = b.path("x").ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) else { return Vec::new() };
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    rd.flatten().filter_map(|e| Some((e.file_name().to_string_lossy().strip_suffix(".mzidx")?.to_string(), e.metadata().ok()?.len()))).collect()
}

pub(crate) fn tx(g: &Gradient) -> String {
    let mut o = Out::default();
    g.write(&mut o);
    o.0
}

/// `[{slot, label, time, full}]` of the session as the page describes it.
pub(crate) fn session_items(items: &str) -> Result<Vec<mzlab_core::originroute::ItemMeta>> {
    mzlab_core::originroute::session_items(items)
}

/// A `ReadAt` over a map of the disk.
pub struct MapAt(pub Arc<Mapped>);

impl ReadAt for MapAt {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        let b = self.0.bytes();
        let a = off as usize;
        let src = a.checked_add(buf.len()).and_then(|e| b.get(a..e)).ok_or_else(|| Error::new("error.badIndex"))?;
        buf.copy_from_slice(src);
        Ok(())
    }
}

/// What an operation gives back.
pub enum Answer {
    Text(String),
    Bytes(Vec<u8>),
}

#[derive(Default)]
pub struct Eng {
    pub(crate) idx: HashMap<u32, Routes<MapAt>>,
    pub(crate) lr_slot: HashMap<String, u32>,
    pub(crate) next_lr: u32,
    pub(crate) labs: Vec<(Lab, u64)>,
    pub(crate) labs_sig: Option<String>,
    /// the samples of a maker's file read as they are (`nativi.rs`)
    pub(crate) nat: Nativi,
}
