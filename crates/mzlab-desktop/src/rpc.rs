//! The native twin of `rust-worker.js` + `mzlab-wasm`: the operations the page asks of its engine (`rpc` over `mzlab://`), answered from memory-mapped files and
//! the same `mzlab-core` code the site runs as WebAssembly. A file is read through `MapAt` (a `ReadAt` over a map), an index is the `.mzidx` of the index folder
//! of the app (`idxbuild.rs`), the files and the notebook of the student are in `store.rs`. An answer is a text (the JSON the route gives) or bytes (`scanbin`);
//! an empty text means «not answered here» exactly as in the worker. Errors are `{error_key, params}` with status 400.

use std::collections::HashMap;
use std::io::{Seek, SeekFrom, Write};
use std::sync::Arc;

use mzlab_core::gradient::{self, Gradient};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{self, Out};
use mzlab_core::labmethods::{self, Lab};
use mzlab_core::mzmlhead::{Chrom, Mzml};
use mzlab_core::routes::{Query, Routes};
use mzlab_core::{Error, Result};
use serde_json::{json, Value};

use crate::engine::State;
use crate::mapped::Mapped;
use crate::store::clean;

/// The slots of the files of the page's list (not of the session) start here, as in `rust-worker.js`.
const LR_BASE: u32 = 1_000_000;

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
    idx: HashMap<u32, Routes<MapAt>>,
    lr_slot: HashMap<String, u32>,
    next_lr: u32,
    labs: Vec<(Lab, u64)>,
    labs_sig: Option<String>,
}

fn io_error(e: &std::io::Error) -> Error {
    Error::new("err.file.raw").with("detail", e.to_string())
}

fn bad_index() -> Error {
    Error::new("error.badIndex")
}

fn text(t: impl Into<String>) -> Result<Answer> {
    Ok(Answer::Text(t.into()))
}

fn opt(a: Option<String>) -> Result<Answer> {
    text(a.unwrap_or_default())
}

fn map_file(path: &std::path::Path) -> Result<(MapAt, u64)> {
    let m = Arc::new(Mapped::open(path).map_err(|e| io_error(&e))?);
    let n = m.bytes().len() as u64;
    Ok((MapAt(m), n))
}

/// The indexes of the index folder: `(name of the mzML, size)`.
fn index_list(b: &crate::idxbuild::IdxBuilder) -> Vec<(String, u64)> {
    let Some(dir) = b.path("x").ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) else { return Vec::new() };
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    rd.flatten().filter_map(|e| Some((e.file_name().to_string_lossy().strip_suffix(".mzidx")?.to_string(), e.metadata().ok()?.len()))).collect()
}

fn tx(g: &Gradient) -> String {
    let mut o = Out::default();
    g.write(&mut o);
    o.0
}

/// `[{slot, label, time, full}]` of the session as the page describes it.
fn session_items(items: &str) -> Result<Vec<mzlab_core::originroute::ItemMeta>> {
    use mzlab_core::json::Value as J;
    let v = json::parse(items.as_bytes())?;
    Ok(v.arr()
        .iter()
        .map(|i| mzlab_core::originroute::ItemMeta {
            slot: i.get("slot").and_then(J::f64).unwrap_or(-1.0) as u32,
            label: i.get("label").and_then(J::str).unwrap_or("").to_string(),
            time: i.get("time").and_then(J::f64),
            full: matches!(i.get("full"), Some(J::Bool(true))),
        })
        .collect())
}

impl State {
    /// The number of a file of the disk (the same number for the same path).
    pub(crate) fn pick_path(&self, path: &std::path::Path) -> u32 {
        {
            let g = self.lock_inner();
            if let Some((id, _)) = g.picked.iter().filter(|(_, p)| p.path == path).max_by_key(|(id, _)| **id) {
                return *id;
            }
        }
        self.pick(path.to_path_buf()).map(|(id, _)| id).unwrap_or(0)
    }

    /// A file that reaches the app as bytes (a file dragged into the page, not picked in the system dialog): pieces of `body` written at `off` into the folder
    /// of the files made by the app. Returns the number of the file when the last piece is in.
    pub fn upload_chunk(&self, name: &str, off: u64, total: u64, body: &[u8]) -> Result<Option<u32>> {
        let name = crate::store::clean(name)?;
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
    fn mzml_of(&self, fid: u32) -> Result<(MapAt, u64)> {
        map_file(&self.picked(fid)?.path)
    }

    /// The mzML of a registered file, by name.
    fn mzml_named(&self, name: &str) -> Result<(MapAt, u64)> {
        let e = self.store.lock().unwrap_or_else(|e| e.into_inner()).get(name).ok_or_else(|| Error::new("err.session.fileNotFound").with("name", name))?;
        map_file(&e.path)
    }

    /// Opens the index of `name` in the slot of the page's list. `err.engine.index` when there is none.
    fn open_lr(&self, eng: &mut Eng, name: &str) -> Result<u32> {
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
        Ok(slot)
    }

    fn open_index_file(&self, name: &str) -> Result<Routes<MapAt>> {
        let (r, size) = map_file(&self.indexer()?.path(name)?)?;
        Ok(Routes::new(Index::open(r, size)?))
    }

    fn labs_of(eng: &Eng) -> Vec<Lab> {
        eng.labs.iter().map(|(l, _)| l.clone()).collect()
    }

    /// The methods (`.dam`) of the store, oldest first, loaded again whenever the set changes.
    fn lab_sync(&self, eng: &mut Eng) {
        let mut dams: Vec<_> = self.store.lock().unwrap_or_else(|e| e.into_inner()).list().into_iter().filter(|(n, _, _)| n.to_ascii_lowercase().ends_with(".dam")).collect();
        dams.sort_by(|a, b| (a.1.mtime, a.1.seq, &a.0).cmp(&(b.1.mtime, b.1.seq, &b.0)));
        let sig = dams.iter().map(|(n, e, _)| format!("{n}:{}:{}", e.size, e.mtime)).collect::<Vec<_>>().join("|");
        if eng.labs_sig.as_deref() == Some(sig.as_str()) {
            return;
        }
        eng.labs = dams.iter().filter_map(|(n, e, _)| std::fs::read(&e.path).ok().map(|b| (Lab::read(n, b.clone()), b.len() as u64))).collect();
        eng.labs_sig = Some(sig);
    }

    /// The chromatograms of the mzML of `name` and the reader to go on with.
    fn chroms(&self, eng: &mut Eng, name: &str) -> Result<(Mzml<MapAt>, Vec<Chrom>, u32)> {
        let slot = self.open_lr(eng, name)?;
        let end = eng.idx[&slot].idx.scans.last().map_or(0, |s| s.end);
        let (r, size) = self.mzml_named(name)?;
        let mut m = Mzml::new(r, size);
        let chroms = m.chromatograms(end)?;
        Ok((m, chroms, slot))
    }

    /// `lr_index`: the scan table and the index of a registered mzML (a thread of the build pool), then the index is opened again on the next request.
    fn lr_index(&self, name: &str) -> Result<Answer> {
        let e = self.store.lock().unwrap_or_else(|e| e.into_inner()).get(name).ok_or_else(|| Error::new("err.session.fileNotFound").with("name", name))?;
        let t0 = std::time::Instant::now();
        let (mut rd, size) = map_file(&e.path)?;
        let prepared = mzlab_core::upload::prepare(&mut rd, size, name)?;
        self.indexer()?.build(&e.path, name, &prepared.table)?;
        let mut eng = self.eng.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(slot) = eng.lr_slot.get(name).copied() {
            eng.idx.remove(&slot);
        }
        text(format!("{{\"ms\":{}}}", t0.elapsed().as_millis()))
    }

    /// One operation of the page's engine. `p` holds the parameters (a file of the page is its number `fid`), `body` the bytes of a POST.
    pub fn rpc(&self, op: &str, p: &Value, body: &[u8]) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let u = |k: &str| p[k].as_u64().unwrap_or(0) as u32;
        let f = |k: &str| p[k].as_f64().unwrap_or(0.0);
        let q = || Query::parse(s("query"));
        if op == "lr_index" {
            return self.lr_index(s("name"));                 // the build takes its time: it does not hold the engine
        }
        let mut guard = self.eng.lock().unwrap_or_else(|e| e.into_inner());
        let eng = &mut *guard;
        match op {
            // ---- the engine of the page: indexes by slot of the session, routes that read the index and the mzML
            "idxopen" => {
                let r = self.open_index_file(s("name")).map_err(|_| Error::new("error.noIndex"))?;
                let out = format!("{{\"scans\":{},\"hr1\":{}}}", r.idx.scans.len(), r.idx.hr1());
                eng.idx.insert(u("slot"), r);
                text(out)
            }
            "route" => match eng.idx.get_mut(&u("slot")) {
                Some(r) => opt(r.handle(s("path"), &q())?),
                None => text(""),
            },
            "scaninfo" => match eng.idx.get(&u("slot")) {
                Some(r) if r.has_scans() => text(r.scaninfo(f("sid") as usize, &mut self.mzml_of(u("fid"))?.0)?),
                _ => text(""),
            },
            "mzmlroute" => match eng.idx.get_mut(&u("slot")) {
                Some(r) => opt(r.mzml_route(s("path"), &q(), &mut self.mzml_of(u("fid"))?.0)?),
                None => text(""),
            },
            "scanbin" => match eng.idx.get_mut(&u("slot")) {
                Some(r) => Ok(Answer::Bytes(r.scanbin_route(&q(), &mut self.mzml_of(u("fid"))?.0)?.unwrap_or_default())),
                None => Ok(Answer::Bytes(Vec::new())),
            },
            "specexact" => match eng.idx.get_mut(&u("slot")) {
                Some(r) => opt(r.spectrum_exact(&q(), &mut self.mzml_of(u("fid"))?.0)?),
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
                let (rd, size) = self.mzml_of(u("fid"))?;
                let mut m = Mzml::new(rd, size);
                let chroms = m.chromatograms(end)?;
                text(mzlab_core::fileroutes::mrm_transitions(&mut m, &chroms)?)
            }
            "fileinfo" => {
                let r = eng.idx.get_mut(&u("slot")).ok_or_else(bad_index)?;
                let (rd, size) = self.mzml_of(u("fid"))?;
                let mut m = Mzml::new(rd, size);
                let head = m.info_head()?;
                text(mzlab_core::fileroutes::fileinfo(r, &mut m, s("name"), &head)?)
            }
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
            "idxxic" => match eng.idx.get_mut(&u("slot")) {
                Some(r) if r.has_scans() => text(r.xic(f("mz"), f("tol"), u("level") as u8)?),
                _ => text(""),
            },
            "close" => {
                eng.idx.remove(&u("slot"));
                text("{}")
            }
            "reset" => {
                eng.idx.retain(|k, _| *k >= LR_BASE);
                text("{}")
            }

            // ---- the files of the page's list, the session and the notebook
            "lr_boot" => {
                let list = self.store.lock().unwrap_or_else(|e| e.into_inner()).list();
                let mut rows = Vec::new();
                for (name, e, changed) in list {
                    let mzml = name.to_ascii_lowercase().ends_with(".mzml");
                    if changed {
                        self.indexer()?.remove(&name);
                        if let Some(slot) = eng.lr_slot.get(&name).copied() {
                            eng.idx.remove(&slot);
                        }
                    }
                    let indexed = mzml && self.open_lr(eng, &name).is_ok();
                    rows.push(json!({ "name": name, "size": e.size, "indexed": indexed, "id": self.pick_path(&e.path) }));
                }
                text(Value::Array(rows).to_string())
            }
            "lr_put" => {
                let name = clean(s("name"))?;
                let pk = self.picked(u("fid"))?;
                let path = if self.is_temp(&pk.path) {
                    let dest = self.store.lock().unwrap_or_else(|e| e.into_inner()).own_dir()?.join(name);
                    std::fs::copy(&pk.path, &dest).map_err(|e| io_error(&e))?;
                    dest
                } else {
                    pk.path
                };
                if let Some(slot) = eng.lr_slot.get(name) {
                    eng.idx.remove(slot);
                }
                self.indexer()?.remove(name);
                let e = self.store.lock().unwrap_or_else(|e| e.into_inner()).put(name, path)?;
                text(json!({ "size": e.size, "id": self.pick_path(&e.path) }).to_string())
            }
            "lr_entry" => {
                let name = s("name");
                let slot = self.open_lr(eng, name)?;
                let (rd, size) = self.mzml_named(name)?;
                let mut m = Mzml::new(rd, size);
                let r = eng.idx.get_mut(&slot).ok_or_else(bad_index)?;
                text(mzlab_core::lr::file_entry(&mut r.idx, &mut m, name, size)?)
            }
            "lr_items" => {
                use mzlab_core::json::Value as J;
                let name = s("name");
                let slot = self.open_lr(eng, name)?;
                let (rd, size) = self.mzml_named(name)?;
                let mut m = Mzml::new(rd, size);
                let v = json::parse(p["sample"].to_string().as_bytes())?;
                let st = |k: &str| v.get(k).and_then(J::str).map(String::from);
                let sample = mzlab_core::lr::Sample {
                    label: st("label"),
                    typ: st("type"),
                    time: v.get("time").and_then(J::f64),
                    conc: v.get("conc").and_then(J::f64),
                    cunit: st("cunit"),
                    rep: st("rep"),
                };
                let r = eng.idx.get_mut(&slot).ok_or_else(bad_index)?;
                let items = mzlab_core::lr::open_items(&mut r.idx, &mut m, name, &sample)?;
                let body: Vec<String> = items.iter().map(|i| format!("{{\"info\":{},\"hr\":{}}}", i.info, i.hr)).collect();
                text(format!("[{}]", body.join(",")))
            }
            "lr_composition" => text(mzlab_core::comproute::composition_route(&q()).unwrap_or_default()),
            "lr_stat" => match mzlab_core::statroute::stat_route(s("path"), s("body")) {
                None => text(""),
                Some(Ok(j)) => text(j),
                Some(Err(j)) => text(format!("E{j}")),
            },
            "lr_formula" => {
                let ad = s("adduct");
                text(mzlab_core::elements::formula_mz(s("f"), if ad.is_empty() { None } else { Some(ad) })?)
            }
            "lr_remove" => {
                let name = s("name");
                if let Some(slot) = eng.lr_slot.get(name) {
                    eng.idx.remove(slot);
                }
                self.store.lock().unwrap_or_else(|e| e.into_inner()).remove(name);
                text("{}")
            }
            "lr_space" => {
                let names: Vec<String> = self.store.lock().unwrap_or_else(|e| e.into_inner()).list().into_iter().map(|(n, _, _)| n).collect();
                let files: u64 = self.store.lock().unwrap_or_else(|e| e.into_inner()).list().iter().map(|(_, e, _)| e.size).sum();
                let b = self.indexer()?;
                let (mut idx, mut gone, mut n_gone) = (0u64, 0u64, 0u64);
                for (name, size) in index_list(&b) {
                    if names.contains(&name) {
                        idx += size;
                    } else if p["free"].as_bool().unwrap_or(false) {
                        b.remove(&name);
                    } else {
                        gone += size;
                        n_gone += 1;
                    }
                }
                text(json!({ "opfs": true, "files": files, "idx": idx + gone, "idx_removed": gone, "n_removed": n_gone, "usage": files + idx + gone, "quota": 0, "persisted": true }).to_string())
            }
            "lr_kv_get" => text(json!({ "value": self.store.lock().unwrap_or_else(|e| e.into_inner()).kv_get(s("key")) }).to_string()),
            "lr_kv_set" => {
                self.store.lock().unwrap_or_else(|e| e.into_inner()).kv_set(s("key"), s("value"));
                text("{}")
            }
            "lr_clear" => {
                eng.idx.retain(|k, _| *k < LR_BASE);
                eng.lr_slot.clear();
                eng.labs.clear();
                eng.labs_sig = None;
                self.store.lock().unwrap_or_else(|e| e.into_inner()).clear();
                for (name, _) in index_list(&*self.indexer()?) {
                    self.indexer()?.remove(&name);
                }
                text("{}")
            }

            // ---- the methods (.dam) and how an item was acquired
            "lr_dam_sync" => {
                self.lab_sync(eng);
                let mut o = Out::default();
                o.raw("[");
                for (i, (l, size)) in eng.labs.iter().enumerate() {
                    o.raw(if i > 0 { "," } else { "" });
                    l.write_listed(*size, &mut o);
                }
                o.raw("]");
                text(o.0)
            }
            "lr_grad_make" => {
                let t = s("text");
                let g = match s("fmt") {
                    "course" => gradient::course(),
                    "chromeleon" => mzlab_core::chromeleon::parse(t),
                    "model" => json::parse(t.as_bytes()).and_then(|v| gradient::model_from_json(&v)),
                    _ => gradient::parse_csv(t),
                };
                text(tx(&g?))
            }
            "lr_method_warnings" => {
                self.lab_sync(eng);
                let v = json::parse(p["have"].to_string().as_bytes())?;
                let have: Vec<String> = v.arr().iter().filter_map(|x| x.str().map(String::from)).collect();
                text(labmethods::warnings_json(&Self::labs_of(eng), &have))
            }
            "lr_method" | "lr_gradient" => {
                self.lab_sync(eng);
                let item = s("name");
                let name = item.split('#').next().unwrap_or(item);
                let slot = self.open_lr(eng, name)?;
                let (rd, size) = self.mzml_named(name)?;
                let mut m = Mzml::new(rd, size);
                let data = mzlab_core::itemroutes::item_method(&eng.idx[&slot].idx, &mut m, item)?;
                let labs = Self::labs_of(eng);
                if op == "lr_method" {
                    return text(labmethods::method_json(&data, &labs, item)?);
                }
                let pick = labmethods::pick(&data, &labs, item)?;
                match gradient::from_dam(labmethods::lc_of(&labs, pick))? {
                    Some(g) => text(tx(&g)),
                    None => text("null"),
                }
            }
            "lr_analog" => {
                let name = s("name").split('#').next().unwrap_or("");
                let (mut m, chroms, _) = self.chroms(eng, name)?;
                text(mzlab_core::itemroutes::analog(&mut m, &chroms, p["i"].as_i64().unwrap_or(-1), p["n"].as_i64().unwrap_or(2000))?)
            }
            "lr_pda" => {
                let name = s("name").split('#').next().unwrap_or("");
                let (mut m, chroms, _) = self.chroms(eng, name)?;
                text(mzlab_core::itemroutes::pda_trace(&mut m, &chroms)?)
            }
            "lr_delay" => {
                let name = s("name").split('#').next().unwrap_or("");
                let (mut m, chroms, slot) = self.chroms(eng, name)?;
                let r = eng.idx.get_mut(&slot).ok_or_else(bad_index)?;
                text(r.delay(&mut m, &chroms)?)
            }
            _ => {
                let _ = body;
                Err(Error::new("err.file.raw").with("detail", format!("unknown operation {op}")))
            }
        }
    }
}
