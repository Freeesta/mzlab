//! The operations of `rpc.rs` about the files of the page's list, the notebook and the methods (`.dam`): the native `lr_*` of `rust-worker.js`.


use mzlab_core::gradient;
use mzlab_core::json::{self, Out};
use mzlab_core::labmethods::{self, Lab};
use mzlab_core::mzmlhead::{Chrom, Mzml};
use mzlab_core::routes::Query;
use mzlab_core::{Error, Result};
use serde_json::{json, Value};

use crate::engine::State;
use crate::nativi::Src;
use crate::rpc::*;
use crate::store::clean;

impl State {
    pub(crate) fn labs_of(eng: &Eng) -> Vec<Lab> {
        eng.labs.iter().map(|(l, _)| l.clone()).collect()
    }

    /// The methods (`.dam`) of the store, oldest first, loaded again whenever the set changes.
    pub(crate) fn lab_sync(&self, eng: &mut Eng) {
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
    pub(crate) fn chroms<'a>(&self, eng: &'a mut Eng, name: &str) -> Result<(Mzml<Src<'a>>, Vec<Chrom>, u32)> {
        let slot = self.open_lr(eng, name)?;
        let end = eng.idx[&slot].idx.scans.last().map_or(0, |s| s.end);
        let (r, size) = self.src_named(&mut eng.nat, slot, name)?;
        let mut m = Mzml::new(r, size);
        let chroms = m.chromatograms(end)?;
        Ok((m, chroms, slot))
    }

    #[allow(unused_variables)]
    pub(crate) fn op_files(&self, eng: &mut Eng, op: &str, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let u = |k: &str| p[k].as_u64().unwrap_or(0) as u32;
        let f = |k: &str| p[k].as_f64().unwrap_or(0.0);
        let q = || Query::parse(s("query"));
        match op {
            "lr_boot" => {
                let list = self.store.lock().unwrap_or_else(|e| e.into_inner()).list();
                let mut rows = Vec::new();
                for (name, e, changed) in list {
                    let mzml = e.nat.is_some() || name.to_ascii_lowercase().ends_with(".mzml");
                    if changed {
                        self.indexer()?.remove(&name);
                        Self::drop_slot(eng, &name);
                    }
                    let indexed = mzml && self.open_lr(eng, &name).is_ok();
                    rows.push(json!({ "name": name, "size": e.size, "indexed": indexed, "nat": e.nat.is_some(), "id": self.pick_path(&e.path) }));
                }
                text(Value::Array(rows).to_string())
            }
            "lr_put" => {
                let name = clean(s("name"))?;
                if p["nat"].is_object() {
                    return self.put_native(eng, name, &p["nat"]);
                }
                let pk = self.picked(u("fid"))?;
                let path = if self.is_temp(&pk.path) {
                    let dest = self.store.lock().unwrap_or_else(|e| e.into_inner()).own_dir()?.join(name);
                    std::fs::copy(&pk.path, &dest).map_err(|e| io_error(&e))?;
                    dest
                } else {
                    pk.path
                };
                Self::drop_slot(eng, name);
                self.indexer()?.remove(name);
                let e = self.store.lock().unwrap_or_else(|e| e.into_inner()).put(name, path)?;
                text(json!({ "size": e.size, "id": self.pick_path(&e.path) }).to_string())
            }
            "lr_entry" => {
                let name = s("name");
                let slot = self.open_lr(eng, name)?;
                let (rd, size) = self.src_named(&mut eng.nat, slot, name)?;
                let mut m = Mzml::new(rd, size);
                let r = eng.idx.get_mut(&slot).ok_or_else(bad_index)?;
                let entry = mzlab_core::lr::file_entry(&mut r.idx, &mut m, name, size)?;
                match self.store.lock().unwrap_or_else(|e| e.into_inner()).get(name) {
                    Some(e) if e.nat.is_some() => text(native_entry(&entry, e.size)),             // the size of a sample of a maker's file is the one of its members
                    _ => text(entry),
                }
            }
            "lr_items" => {
                use mzlab_core::json::Value as J;
                let name = s("name");
                let slot = self.open_lr(eng, name)?;
                let (rd, size) = self.src_named(&mut eng.nat, slot, name)?;
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
            _ => Err(Error::new("rpc.unknown")),
        }
    }

    #[allow(unused_variables)]
    pub(crate) fn op_misc(&self, eng: &mut Eng, op: &str, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let u = |k: &str| p[k].as_u64().unwrap_or(0) as u32;
        let f = |k: &str| p[k].as_f64().unwrap_or(0.0);
        let q = || Query::parse(s("query"));
        match op {
            "lr_space" => {
                let names: Vec<String> = self.store.lock().unwrap_or_else(|e| e.into_inner()).list().into_iter().map(|(n, _, _)| n).collect();
                let files: u64 = self.store.lock().unwrap_or_else(|e| e.into_inner()).list().iter().map(|(_, e, _)| e.size).sum();
                let b = self.indexer()?;
                let (mut idx, mut gone, mut n_gone) = (0u64, 0u64, 0u64);
                let derived = p["free"].as_str() == Some("derived");                           // every index can be made again from the sources: all of them go
                for (name, size) in index_list(&b) {
                    if names.contains(&name) && !derived {
                        idx += size;
                    } else if derived || p["free"].as_bool().unwrap_or(false) {
                        b.remove(&name);
                    } else {
                        gone += size;
                        n_gone += 1;
                    }
                }
                text(json!({ "opfs": true, "files": files, "idx": idx + gone, "idx_removed": gone, "n_removed": n_gone, "usage": files + idx + gone, "quota": 0, "persisted": true }).to_string())
            }
            "lr_remove" => {
                let name = s("name");
                if let Some(slot) = eng.lr_slot.get(name).copied() {
                    eng.idx.remove(&slot);
                    eng.nat.forget(slot);
                }
                self.store.lock().unwrap_or_else(|e| e.into_inner()).remove(name);
                text("{}")
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
            "lr_provenance" => text(mzlab_core::provenance::build(s("ctx"))?),
            "lr_kv_get" => text(json!({ "value": self.store.lock().unwrap_or_else(|e| e.into_inner()).kv_get(s("key")) }).to_string()),
            "lr_kv_set" => {
                self.store.lock().unwrap_or_else(|e| e.into_inner()).kv_set(s("key"), s("value"));
                text("{}")
            }
            "lr_clear" => {
                eng.idx.retain(|k, _| *k < LR_BASE);
                eng.nat.clear();
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
            _ => Err(Error::new("rpc.unknown")),
        }
    }

    #[allow(unused_variables)]
    pub(crate) fn op_lab(&self, eng: &mut Eng, op: &str, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let u = |k: &str| p[k].as_u64().unwrap_or(0) as u32;
        let f = |k: &str| p[k].as_f64().unwrap_or(0.0);
        let q = || Query::parse(s("query"));
        match op {
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
                let (rd, size) = self.src_named(&mut eng.nat, slot, name)?;
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
                let slot = self.open_lr(eng, name)?;
                let end = eng.idx[&slot].idx.scans.last().map_or(0, |s| s.end);
                let (rd, size) = self.src_named(&mut eng.nat, slot, name)?;
                let mut m = Mzml::new(rd, size);
                let chroms = m.chromatograms(end)?;
                let r = eng.idx.get_mut(&slot).ok_or_else(bad_index)?;
                text(r.delay_item(&mut m, &chroms, s("name"))?)
            }

            _ => Err(Error::new("rpc.unknown")),
        }
    }
}

/// The entry of a native sample as the page lists it: the size of the members and `nat`.
fn native_entry(entry: &str, size: u64) -> String {
    match serde_json::from_str::<Value>(entry) {
        Ok(mut v) => {
            v["size"] = json!(size);
            v["nat"] = json!(true);
            v.to_string()
        }
        Err(_) => entry.to_string(),
    }
}

impl State {
    /// `lr_put` of a sample of a maker's file: `nat` = `{key, c, id, centroid, members: [{name, fid}]}`; the members stay where they are on the disk.
    fn put_native(&self, eng: &mut Eng, name: &str, nat: &Value) -> Result<Answer> {
        let mut members = Vec::new();
        for m in nat["members"].as_array().map_or(&[][..], |a| a.as_slice()) {
            let n = m["name"].as_str().unwrap_or("");
            members.push((n.to_string(), self.picked(m["fid"].as_u64().unwrap_or(0) as u32)?.path));
        }
        if let Some(slot) = eng.lr_slot.get(name).copied() {
            eng.idx.remove(&slot);
            eng.nat.forget(slot);
        }
        self.indexer()?.remove(name);
        let n = crate::store::Nat {
            ds: nat["key"].as_str().unwrap_or("").to_string(),
            c: nat["c"].as_u64().unwrap_or(0) as usize,
            id: nat["id"].as_u64().unwrap_or(0) as u32,
            centroid: nat["centroid"].as_bool().unwrap_or(false),
            members,
        };
        let e = self.store.lock().unwrap_or_else(|e| e.into_inner()).put_nat(name, n)?;
        text(json!({ "size": e.size, "id": self.pick_path(&e.path) }).to_string())
    }
}
