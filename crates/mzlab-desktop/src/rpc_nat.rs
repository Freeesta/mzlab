//! The operations of `rpc` about the samples of a maker's file read as they are (`nativi.rs`): the samples of a dataset the page describes (`natsamples`) and
//! «Download .mzML» (`nat2mzml`: the mzML of one sample written on request to the temporary folder, registered like a picked file; progress by `rawprogress`).

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use mzlab_core::{lettori_mzml, Error, Result};
use serde_json::{json, Value};

use crate::engine::{CountWrite, Picked, Progress, State};
use crate::nativi;
use crate::rpc::{io_error, text, Answer, Eng};

impl State {
    /// Closes the slot the file `name` is open in (its index and its reader): the file changed or was replaced.
    pub(crate) fn drop_slot(eng: &mut Eng, name: &str) {
        if let Some(slot) = eng.lr_slot.get(name).copied() {
            eng.idx.remove(&slot);
            eng.nat.forget(slot);
        }
    }

    /// `natsamples {members: [{name, fid}]}`: `{"format", "samples": [{id, name, scans}]}` of the dataset the members make.
    pub(crate) fn nat_samples(&self, p: &Value) -> Result<Answer> {
        let mut members: Vec<(String, PathBuf)> = Vec::new();
        for m in p["members"].as_array().map_or(&[][..], |a| a.as_slice()) {
            members.push((
                m["name"].as_str().unwrap_or("").to_string(),
                self.picked(m["fid"].as_u64().unwrap_or(0) as u32)?.path,
            ));
        }
        if members.is_empty() {
            return Err(Error::new("err.file.notAccepted").with("name", ""));
        }
        text(nativi::samples(&members)?)
    }

    /// `nat2mzml {name, pid}`: the mzML of the sample `name` in the temporary folder; `{"id", "size"}` of the new file (read with `file`, saved with `save_mzml`).
    /// `pid` is the number the page polls `rawprogress` with.
    pub(crate) fn nat_to_mzml(&self, p: &Value) -> Result<Answer> {
        let name = p["name"].as_str().unwrap_or("");
        let pid = p["pid"].as_u64().unwrap_or(0) as u32;
        let nat = self
            .store
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            .and_then(|e| e.nat)
            .ok_or_else(|| Error::new("err.session.fileNotFound").with("name", name))?;
        let progress = Arc::new(Progress::default());
        self.lock_inner().progress.insert(pid, progress.clone());
        let mut l = nativi::open(&nat.members, nat.centroid)?;
        let tmp = std::env::temp_dir().join(format!("mzlab-{}-n{}.mzML", std::process::id(), pid));
        let mut out = CountWrite {
            inner: std::io::BufWriter::new(std::fs::File::create(&tmp).map_err(|e| io_error(&e))?),
            progress: progress.clone(),
        };
        let pr = progress.clone();
        let mut tell = move |done: usize, total: usize| {
            pr.scans.store(done as u64, Ordering::Relaxed);
            pr.scans_total.store(total as u64, Ordering::Relaxed);
        };
        let opz = mzlab_core::mzmlwrite::Opzioni {
            numpress: p["numpress"].as_bool().unwrap_or(false),
            ..Default::default()
        };
        lettori_mzml::scrivi_con(&mut *l, nat.c, &mut out, &mut tell, opz)?;
        std::io::Write::flush(&mut out).map_err(|e| io_error(&e))?;
        drop(out);
        let size = std::fs::metadata(&tmp).map_err(|e| io_error(&e))?.len();
        let mut g = self.lock_inner();
        g.next_id += 1;
        let id = g.next_id;
        g.picked.insert(
            id,
            Picked {
                path: tmp.clone(),
                name: name.to_string(),
                size,
            },
        );
        g.temps.push(tmp);
        text(json!({ "id": id, "size": size }).to_string())
    }
}
