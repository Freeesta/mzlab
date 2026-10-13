//! wasm-bindgen bridge. It runs only inside a Web Worker; the main thread gets ready-to-draw arrays.
//!
//! Every answer is written into a `Float64Array` the caller provides (the worker recycles them, so no allocation per frame):
//! `[rt(n), y(n)]` for chromatograms and `[n_total, count, (sid, rt, k, mz(k), y(k)) per scan]` for spectra. Errors are JSON
//! `{"error_key": ..., "params": {...}}` strings (the page translates the key), never a text.
#![forbid(unsafe_code)]
mod info; mod diff; mod items; mod metodo; mod stat;
use js_sys::{Function, Uint8Array};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::routes::{Query, Routes};
use mzlab_core::{analysis, mzidx, mzml, peakq, peaks, raw};
use std::collections::HashMap;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use wasm_bindgen::prelude::*;
mod scans;
mod cache;
mod wiff;
mod agilent;
mod centroidi;

/// Version of the engine (core crate), for the page to show and to check the loaded .wasm.
#[wasm_bindgen]
pub fn version() -> String {
    mzlab_core::version().to_string()
}

/// Reads a file in blocks asked to JS: `read(offset, length) -> Uint8Array` (a slice of the Blob read synchronously in the worker).
/// It also seeks (the `.raw` reader jumps from packet to packet): the last block stays in memory, so scans that follow each
/// other cost one JS call per block, never one per read.
struct BlockReader {
    read: Function,
    pos: u64,
    size: u64,
    block: Vec<u8>,
    block_start: u64,
}

impl BlockReader {
    fn new(read: Function, size: u64) -> Self {
        BlockReader {
            read,
            pos: 0,
            size,
            block: Vec::new(),
            block_start: 0,
        }
    }

    fn fill(&mut self) -> std::io::Result<()> {
        let want = (BLOCK as u64).min(self.size - self.pos);
        let block = self
            .read
            .call2(
                &JsValue::NULL,
                &(self.pos as f64).into(),
                &(want as f64).into(),
            )
            .map_err(|_| std::io::Error::other("block read failed"))?;
        let block: Uint8Array = block
            .dyn_into()
            .map_err(|_| std::io::Error::other("block is not a Uint8Array"))?;
        self.block.clear();
        self.block.resize(block.length() as usize, 0);
        block.copy_to(&mut self.block);
        self.block_start = self.pos;
        Ok(())
    }
}

const BLOCK: usize = 1 << 20;

impl Read for BlockReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() || self.pos >= self.size {
            return Ok(0);
        }
        let end = self.block_start + self.block.len() as u64;
        if self.pos < self.block_start || self.pos >= end {
            self.fill()?;
            if self.block.is_empty() {
                return Ok(0);
            }
        }
        let off = (self.pos - self.block_start) as usize;
        let n = buf.len().min(self.block.len() - off);
        buf[..n].copy_from_slice(&self.block[off..off + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for BlockReader {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let p = match to {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::End(d) => self.size.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos = p.ok_or_else(|| std::io::Error::other("seek before the start"))?;
        Ok(self.pos)
    }
}

/// Sends what is written to JS in blocks: `sink(Uint8Array)`; the page keeps the pieces as Blob parts (outside the WASM memory).
pub(crate) struct BlockWriter {
    pub(crate) sink: Function,
    pub(crate) buf: Vec<u8>,
}

impl BlockWriter {
    fn flush_block(&mut self) -> std::io::Result<()> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let a = Uint8Array::from(&self.buf[..]);
        self.sink
            .call1(&JsValue::NULL, &a)
            .map_err(|_| std::io::Error::other("sink failed"))?;
        self.buf.clear();
        Ok(())
    }
}

impl Write for BlockWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(data);
        if self.buf.len() >= 4 * BLOCK {
            self.flush_block()?;
        }
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.flush_block()
    }
}

pub(crate) fn error_json(e: &mzlab_core::Error) -> JsValue {
    let params: Vec<String> = e
        .params
        .iter()
        .map(|(k, v)| format!("\"{}\":{:?}", k, v))
        .collect();
    JsValue::from_str(&format!(
        "{{\"error_key\":\"{}\",\"params\":{{{}}}}}",
        e.key,
        params.join(",")
    ))
}

fn r1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
fn r3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}
fn r4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

/// The files opened in the worker, by slot (the index `k` of the session).
#[wasm_bindgen]
#[derive(Default)]
pub struct Engine {
    runs: HashMap<u32, mzml::Run>,
    /// The `.mzidx` of each file: the routes that need only the index are answered from it (no mzML is read).
    pub(crate) idx: HashMap<u32, Routes<JsReader>>,
    labs: Vec<(mzlab_core::labmethods::Lab, u64)>,
    /// Bytes the cached maps of all the files may hold together (`set_cache_budget`; 0 = the default).
    pub(crate) cache_cap: usize,
}

/// Bytes of the index file asked to JS: `read(offset, length) -> Uint8Array`.
pub(crate) struct JsReader {
    pub(crate) read: Function,
}

impl ReadAt for JsReader {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        let bad = || mzlab_core::Error::new("error.badIndex");
        let v = self
            .read
            .call2(
                &JsValue::NULL,
                &(off as f64).into(),
                &(buf.len() as f64).into(),
            )
            .map_err(|_| bad())?;
        let a: Uint8Array = v.dyn_into().map_err(|_| bad())?;
        if a.length() as usize != buf.len() {
            return Err(bad());
        }
        a.copy_to(buf);
        Ok(())
    }
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        Engine::default()
    }

    /// Opens a file of `size` bytes read through `read(offset, length)`. Returns a JSON summary
    /// `{"spectra": n, "levels": {"1": [scans, profile scans], ...}, "unit": bool, "chromatograms": m}`.
    pub fn open(&mut self, slot: u32, size: f64, read: Function) -> Result<String, JsValue> {
        self.runs.remove(&slot);
        let reader = BufReader::with_capacity(1 << 20, BlockReader::new(read, size as u64));
        let run = mzml::read(reader).map_err(|e| error_json(&e))?;
        let mut levels: std::collections::BTreeMap<u8, (usize, usize)> = Default::default();
        for s in &run.spectra {
            let e = levels.entry(s.level).or_default();
            e.0 += 1;
            e.1 += s.profile as usize;
        }
        let lv: Vec<String> = levels
            .iter()
            .map(|(l, (n, p))| format!("\"{l}\":[{n},{p}]"))
            .collect();
        let out = format!(
            "{{\"spectra\":{},\"levels\":{{{}}},\"unit\":{},\"chromatograms\":{}}}",
            run.spectra.len(),
            lv.join(","),
            analysis::is_unit_resolution(&run.analyzers, &run.spectra),
            run.chromatograms.len()
        );
        self.runs.insert(slot, run);
        Ok(out)
    }

    pub fn close(&mut self, slot: u32) {
        self.runs.remove(&slot);
        self.idx.remove(&slot);
    }

    pub fn close_all(&mut self) {
        self.runs.clear();
        self.idx.clear();
    }

    /// Opens the `.mzidx` of a file (`size` bytes, read through `read(offset, length)`). Returns `{"scans": n, "hr1": bool}`.
    pub fn open_index(&mut self, slot: u32, size: f64, read: Function) -> Result<String, JsValue> {
        self.idx.remove(&slot);
        let idx = Index::open(JsReader { read }, size as u64).map_err(|e| error_json(&e))?;
        let out = format!("{{\"scans\":{},\"hr1\":{}}}", idx.scans.len(), idx.hr1());
        self.idx.insert(slot, Routes::new(idx));
        Ok(out)
    }

    /// Makes the item at `slot` the polarity part of a mixed file (`1` or `-1`; anything else = the whole file).
    pub fn set_polarity(&mut self, slot: u32, pol: i32) { if let Some(r) = self.idx.get_mut(&slot) { r.pol = [1, -1].contains(&pol).then_some(pol as i8); } }

    /// The answer of an `api/...` route that only needs the index, as JSON text; an empty string = not answered here (Python's).
    pub fn route(&mut self, slot: u32, path: &str, query: &str) -> Result<String, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else {
            return Ok(String::new());
        };
        r.handle(path, &Query::parse(query))
            .map(|a| a.unwrap_or_default())
            .map_err(|e| error_json(&e))
    }

    /// `/api/scaninfo` of scan `sid`: the scan comes from the index, the header from the mzML (`size` bytes, read through `read`). Empty = Python's.
    pub fn scaninfo(
        &mut self,
        slot: u32,
        sid: u32,
        size: f64,
        read: Function,
    ) -> Result<String, JsValue> {
        let _ = size;
        match self.idx.get(&slot) {
            Some(r) if r.has_scans() => r
                .scaninfo(sid as usize, &mut JsReader { read })
                .map_err(|e| error_json(&e)),
            _ => Ok(String::new()),
        }
    }

    /// `/api/spectrum` of a window holding ONE scan of a centroid HR file: the exact peaks come from the mzML (`read`). Empty = Python's.
    pub fn spectrum_exact(&mut self, slot: u32, query: &str, read: Function) -> Result<String, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else { return Ok(String::new()) };
        r.spectrum_exact(&Query::parse(query), &mut JsReader { read }).map(|a| a.unwrap_or_default()).map_err(|e| error_json(&e))
    }

    /// The SRM transitions of the file at `slot` (`[{id, q1, q3, ce, name, dwell, rt, y}]`), from the chromatograms of its mzML (`api/mrm`).
    pub fn mrm(&mut self, slot: u32, size: f64, read: Function) -> Result<String, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else {
            return Err(error_json(&mzlab_core::Error::new("error.badIndex")));
        };
        let end = r.idx.scans.last().map_or(0, |s| s.end);
        let mut m = mzlab_core::mzmlhead::Mzml::new(JsReader { read }, size as u64);
        let chroms = m.chromatograms(end).map_err(|e| error_json(&e))?;
        mzlab_core::fileroutes::mrm_transitions(&mut m, &chroms).map_err(|e| error_json(&e))
    }

    /// The entry of the loading table for the file at `slot` (index opened with `open_index`; the mzML is read through `read`): JSON, `App.files()` of Python.
    pub fn lr_entry(
        &mut self,
        slot: u32,
        name: &str,
        size: f64,
        read: Function,
    ) -> Result<String, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else {
            return Err(error_json(&mzlab_core::Error::new("error.badIndex")));
        };
        let mut m = mzlab_core::mzmlhead::Mzml::new(JsReader { read }, size as u64);
        mzlab_core::lr::file_entry(&mut r.idx, &mut m, name, size as u64)
            .map_err(|e| error_json(&e))
    }

    /// The session items of the file at `slot` for a sample (`{label, type, time, conc, cunit, rep}` as JSON, `{}` = guessed from the name):
    /// `[{"info": Item.info(), "hr": bool}]`, one item per part of a mixed file.
    pub fn lr_items(
        &mut self,
        slot: u32,
        name: &str,
        size: f64,
        read: Function,
        sample: &str,
    ) -> Result<String, JsValue> {
        use mzlab_core::json::{self, Value};
        let Some(r) = self.idx.get_mut(&slot) else {
            return Err(error_json(&mzlab_core::Error::new("error.badIndex")));
        };
        let v = json::parse(sample.as_bytes()).map_err(|e| error_json(&e))?;
        let s = |k: &str| v.get(k).and_then(Value::str).map(String::from);
        let sample = mzlab_core::lr::Sample {
            label: s("label"),
            typ: s("type"),
            time: v.get("time").and_then(Value::f64),
            conc: v.get("conc").and_then(Value::f64),
            cunit: s("cunit"),
            rep: s("rep"),
        };
        let mut m = mzlab_core::mzmlhead::Mzml::new(JsReader { read }, size as u64);
        let items = mzlab_core::lr::open_items(&mut r.idx, &mut m, name, &sample)
            .map_err(|e| error_json(&e))?;
        let body: Vec<String> = items
            .iter()
            .map(|i| format!("{{\"info\":{},\"hr\":{}}}", i.info, i.hr))
            .collect();
        Ok(format!("[{}]", body.join(",")))
    }

    /// `{"rt": [...], "y": [...]}` of the XIC of one level from the index.
    pub fn index_xic(
        &mut self,
        slot: u32,
        level: u8,
        mz: f64,
        tol: f64,
    ) -> Result<String, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else {
            return Ok(String::new());
        };
        r.xic(mz, tol, level).map_err(|e| error_json(&e))
    }

    /// Converts a Thermo `.raw` of `size` bytes (read through `read(offset, length)`) to mzML, sent to `sink(Uint8Array)` piece by piece.
    /// The `.raw` is never held in memory: only the block being read. Errors are JSON with a key, as in `open`.
    pub fn raw_to_mzml(
        &mut self,
        name: &str,
        size: f64,
        read: Function,
        sink: Function,
        scans: Function, // scans(done, total): scans converted / scans in the file (first done = 0, last done = total)
        flags: u32, // bit 0: profile data as stored (else the vendor centroids); bit 1: leave the computer, user folders, paths and user-label values out of the mzML
    ) -> Result<(), JsValue> {
        let mut out = BlockWriter { sink, buf: Vec::new() };
        let mut on_scan = |d: u32, t: u32| drop(scans.call2(&JsValue::NULL, &d.into(), &t.into()));
        raw::to_mzml_with(BlockReader::new(read, size as u64), &mut out, name, raw::Convert { profile: flags & 1 != 0, anonymous: flags & 2 != 0 }, &mut on_scan)
            .map_err(|e| error_json(&e))?;
        out.flush().map_err(|_| {
            error_json(&mzlab_core::Error::new("err.file.raw").with("detail", "write"))
        })
    }

    /// The text of the instrument method of a Thermo `.raw` (read through `read(offset, length)`): the Chromeleon script with the gradient; empty when there is none.
    pub fn raw_method_text(&mut self, size: f64, read: Function) -> Result<String, JsValue> {
        raw::instrument_method_text(BlockReader::new(read, size as u64))
            .map(|t| t.unwrap_or_default())
            .map_err(|e| error_json(&e))
    }

    /// Number of scans of one level (the capacity a trace needs).
    pub fn count(&self, slot: u32, level: u8) -> u32 {
        self.runs.get(&slot).map_or(0, |r| {
            r.spectra.iter().filter(|s| s.level == level).count() as u32
        })
    }

    /// TIC of one level into `out` as `[rt(n), y(n)]` (RT rounded to 4 decimals, y to 1, as the page gets them from Python);
    /// `px > 0` keeps at most two points per pixel column. Returns n (0 if the slot or `out` does not fit).
    pub fn tic(&self, slot: u32, level: u8, px: u32, out: &mut [f64]) -> u32 {
        match self.runs.get(&slot) {
            Some(r) => {
                let (rt, y) = analysis::tic(&r.spectra, level);
                write_trace(&rt, &y, px, out)
            }
            None => 0,
        }
    }

    /// XIC over `mz ± tol` of one level, same layout as `tic`.
    pub fn xic(&self, slot: u32, level: u8, mz: f64, tol: f64, px: u32, out: &mut [f64]) -> u32 {
        match self.runs.get(&slot) {
            Some(r) => {
                let (rt, y) = analysis::xic(&r.spectra, level, mz - tol, mz + tol);
                write_trace(&rt, &y, px, out)
            }
            None => 0,
        }
    }

    /// Length of the array `spectra` needs for the scans `i0..=i1` of one level (0 if out of range).
    pub fn spectra_len(&self, slot: u32, level: u8, i0: u32, i1: u32) -> u32 {
        let Some(r) = self.runs.get(&slot) else {
            return 0;
        };
        let mut len = 2usize;
        let mut pos = 0u32;
        for s in r.spectra.iter().filter(|s| s.level == level) {
            if pos >= i0 && pos <= i1 {
                len += 3 + 2 * s.mz.len();
            }
            pos += 1;
        }
        if i0 >= pos {
            0
        } else {
            len as u32
        }
    }

    /// Centroid scans `i0..=i1` of one level in bins of `bin_da` (unit resolution), into `out`:
    /// `[n_total, count, (sid, rt, k, mz(k), y(k)) per scan]`, m/z rounded to 3 decimals and y to 1 as Python does.
    /// Returns the length written (0 if out of range or `out` too short).
    pub fn spectra(
        &self,
        slot: u32,
        level: u8,
        i0: u32,
        i1: u32,
        bin_da: f64,
        out: &mut [f64],
    ) -> u32 {
        let Some(r) = self.runs.get(&slot) else {
            return 0;
        };
        let total = r.spectra.iter().filter(|s| s.level == level).count();
        if i0 as usize >= total || out.len() < 2 {
            return 0;
        }
        let mut w = 2usize;
        let mut count = 0.0;
        for (pos, s) in r.spectra.iter().filter(|s| s.level == level).enumerate() {
            if (pos as u32) < i0 || pos as u32 > i1 {
                continue;
            }
            let (mz, y) = if s.mz.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                analysis::bin_centroids(&s.mz, &s.intensity, bin_da)
            };
            if w + 3 + 2 * mz.len() > out.len() {
                return 0;
            }
            out[w] = s.index as f64;
            out[w + 1] = r4(s.rt);
            out[w + 2] = mz.len() as f64;
            w += 3;
            for (i, v) in mz.iter().enumerate() {
                out[w + i] = r3(*v);
            }
            w += mz.len();
            for (i, v) in y.iter().enumerate() {
                out[w + i] = r1(*v);
            }
            w += y.len();
            count += 1.0;
        }
        out[0] = total as f64;
        out[1] = count;
        w as u32
    }

    /// Test hook: aborts the instance like a lost memory or a panic would (the worker supervisor must recover).
    pub fn crash(&self) {
        #[cfg(target_arch = "wasm32")]
        core::arch::wasm32::unreachable();
        #[cfg(not(target_arch = "wasm32"))]
        panic!("crash hook");
    }
}

/// Writes to JS at offsets: `write(offset, Uint8Array)` (the worker puts them in a file of OPFS).
struct JsOutput {
    write: Function,
}

impl mzidx::Output for JsOutput {
    fn write_at(&mut self, offset: u64, data: &[u8]) -> mzlab_core::Result<()> {
        self.write
            .call2(
                &JsValue::NULL,
                &(offset as f64).into(),
                &Uint8Array::from(data),
            )
            .map(|_| ())
            .map_err(|_| mzlab_core::Error::new("err.file.index").with("detail", "write"))
    }
}

fn f64s(b: &[u8]) -> Vec<f64> {
    b.chunks_exact(8)
        .map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
        .collect()
}

/// The files of the session as the page describes them: `[{slot, label, time, full}]`, in session order.
pub(crate) fn session_items(items: &str) -> Result<Vec<mzlab_core::originroute::ItemMeta>, JsValue> {
    use mzlab_core::json::{self, Value};
    let v = json::parse(items.as_bytes()).map_err(|e| error_json(&e))?;
    Ok(v.arr()
        .iter()
        .map(|i| mzlab_core::originroute::ItemMeta {
            slot: i.get("slot").and_then(Value::f64).unwrap_or(-1.0) as u32,
            label: i.get("label").and_then(Value::str).unwrap_or("").to_string(),
            time: i.get("time").and_then(Value::f64),
            full: matches!(i.get("full"), Some(Value::Bool(true))),
        })
        .collect())
}

/// The routes over the files of the session (evidence of origin, maps).
#[wasm_bindgen]
impl Engine {
    /// `api/origin` over the session `items` for the item `k`; `query` = `mz`, `parent`, `rt0`, `rt1`, `formula`.
    /// Empty = Python's (a file without index).
    pub fn origin(&mut self, items: &str, k: i32, query: &str) -> Result<String, JsValue> {
        use mzlab_core::originroute::{origin_route, Ask};
        let metas = session_items(items)?;
        let q = Query::parse(query);
        let num = |n: &str| q.num(n).map_err(|e| error_json(&e));
        let (Some(mz), Some(parent)) = (num("mz")?, num("parent")?) else {
            return Ok(String::new());
        };
        let ask = Ask { k: k as i64, mz, parent, rt0: num("rt0")?, rt1: num("rt1")?, formula: q.some("formula") };
        match origin_route(&metas, &mut self.idx, &ask).map_err(|e| error_json(&e))? {
            Some(rep) => Ok(mzlab_core::jsonb::write(&rep)),
            None => Ok(String::new()),
        }
    }

    /// `api/subxic` of the high-resolution survey in `slot` (the item `k` of the session): empty = Python's (not high resolution, a request Python refuses).
    pub fn subxic(&mut self, slot: u32, k: i32, query: &str) -> Result<String, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else { return Ok(String::new()) };
        let out = mzlab_core::subxic::subxic(r, k as i64, &Query::parse(query)).map_err(|e| error_json(&e))?;
        Ok(out.unwrap_or_default())
    }
}

/// Builds the `.mzidx` of an mzML of `size` bytes (read through `read(offset, length)`, decoded again for every pass) and writes it with
/// `write(offset, bytes)`. The scan table (levels, polarities as signed bytes, RT and TIC as little-endian `f64`, the JSON rows,
/// the `source` and `run` objects) comes from the Python reader. `budget_mb` bounds the peaks held in memory. Returns the statistics as JSON;
/// errors are JSON with a key (`error.memoryBudget` when memory cannot be reserved).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn build_mzidx(
    size: f64,
    read: Function,
    write: Function,
    levels: &[u8],
    polarities: &[u8],
    rt: &[u8],
    tic: &[u8],
    scans_json: &[u8],
    source_json: String,
    run_json: String,
    hr1: bool,
    budget_mb: u32,
) -> Result<String, JsValue> {
    let table = mzidx::ScanTable {
        level: levels.to_vec(),
        polarity: polarities.iter().map(|&b| b as i8).collect(),
        rt: f64s(rt),
        tic: f64s(tic),
        scans_json: scans_json.to_vec(),
        source_json,
        run_json,
        hr1,
    };
    let mut out = JsOutput { write };
    let st = mzidx::build_mzml(
        &table,
        BlockReader::new(read, size as u64),
        &mut out,
        (budget_mb as usize) << 20,
    )
    .map_err(|e| error_json(&e))?;
    Ok(format!(
        "{{\"scans\":{},\"peaks\":{},\"sections\":{},\"passes\":{},\"bytes\":{}}}",
        st.scans, st.peaks, st.sections, st.passes, st.bytes
    ))
}

/// `/api/formula`: neutral mass and ion m/z of a formula (`mzlab-core::elements`); an error is JSON with a key.
#[wasm_bindgen]
pub fn formula_mz(formula: &str, adduct: &str) -> Result<String, JsValue> {
    mzlab_core::elements::formula_mz(formula, if adduct.is_empty() { None } else { Some(adduct) }).map_err(|e| error_json(&e))
}

/// `api/composition` from its query string (`?mz=...&elements=...`): the JSON of the formulas that fit, or empty when Python must answer.
#[wasm_bindgen]
pub fn composition(query: &str) -> String {
    mzlab_core::comproute::composition_route(&Query::parse(query)).unwrap_or_default()
}

/// What the index builder and the loading table need of an uploaded mzML, without Python (`mzlab-core::upload`): the scan table as the eight inputs of
/// `build_mzidx` (levels, polarities, rt, tic, rows, source, run, hr1) and, at the end, the SHA-256 of the file. `read(offset, length)` gives the bytes.
#[wasm_bindgen]
pub fn prepare_upload(size: f64, read: Function, name: &str) -> Result<js_sys::Array, JsValue> {
    let mut r = JsReader { read };
    let p = mzlab_core::upload::prepare(&mut r, size as u64, name).map_err(|e| error_json(&e))?;
    let t = &p.table;
    let f8 = |v: &[f64]| Uint8Array::from(&v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>()[..]);
    let out = js_sys::Array::new();
    out.push(&Uint8Array::from(&t.level[..]));
    out.push(&Uint8Array::from(&t.polarity.iter().map(|&v| v as u8).collect::<Vec<u8>>()[..]));
    out.push(&f8(&t.rt));
    out.push(&f8(&t.tic));
    out.push(&Uint8Array::from(&t.scans_json[..]));
    out.push(&JsValue::from_str(&t.source_json));
    out.push(&JsValue::from_str(&t.run_json));
    out.push(&JsValue::from_bool(t.hr1));
    out.push(&JsValue::from_str(&p.sha256));
    Ok(out)
}

/// Fields per peak written by `detect_peaks`: apex, lo, hi, area, height, snr, scans, gaussian, asymmetry (NaN = not measurable), noise score.
pub const PEAK_FIELDS: usize = 10;

fn put_peak(rt: &[f64], y: &[f64], noise: f64, p: &peaks::Peak, out: &mut [f64]) {
    let q = peakq::quality(rt, y, p.lo, p.hi, noise);
    out[..PEAK_FIELDS].copy_from_slice(&[
        p.apex_i as f64,
        p.lo as f64,
        p.hi as f64,
        p.area,
        p.height,
        p.snr,
        p.n_scans as f64,
        q.gaussian,
        q.asymmetry.unwrap_or(f64::NAN),
        q.noise_score,
    ]);
}

/// Peaks of a trace (RT in minutes, intensities) into `out` as records of `PEAK_FIELDS` numbers, highest first. `mode` 0 = CWT,
/// 1 = prominence + local minimum. Returns the number of peaks written (those that fit).
#[wasm_bindgen]
pub fn detect_peaks(rt: &[f64], y: &[f64], mode: u8, out: &mut [f64]) -> u32 {
    let n = rt.len().min(y.len());
    let (rt, y) = (&rt[..n], &y[..n]);
    let found = if mode == 1 {
        peakq::detect_prominence(rt, y, peakq::ProminenceParams::default())
    } else {
        peaks::detect_peaks(rt, y, peaks::Params::default())
    };
    let noise = peaks::noise_level(y);
    let room = out.len() / PEAK_FIELDS;
    for (i, p) in found.iter().take(room).enumerate() {
        put_peak(rt, y, noise, p, &mut out[i * PEAK_FIELDS..]);
    }
    found.len().min(room) as u32
}

/// Quality of the peak between the indices `lo` and `hi` of a trace: `[gaussian, asymmetry (NaN = not measurable), noise score]` into `out`;
/// the noise of the whole trace is worked out here. Returns 3, or 0 if the arguments do not fit.
#[wasm_bindgen]
pub fn peak_quality(rt: &[f64], y: &[f64], lo: u32, hi: u32, out: &mut [f64]) -> u32 {
    let n = rt.len().min(y.len());
    if out.len() < 3 || lo as usize >= hi as usize || hi as usize >= n {
        return 0;
    }
    let q = peakq::quality(
        &rt[..n],
        &y[..n],
        lo as usize,
        hi as usize,
        peaks::noise_level(&y[..n]),
    );
    out[..3].copy_from_slice(&[q.gaussian, q.asymmetry.unwrap_or(f64::NAN), q.noise_score]);
    3
}

fn write_trace(rt: &[f64], y: &[f64], px: u32, out: &mut [f64]) -> u32 {
    let keep = analysis::minmax_indices(rt, y, px as usize);
    let n = keep.len();
    if 2 * n > out.len() {
        return 0;
    }
    for (j, &i) in keep.iter().enumerate() {
        out[j] = r4(rt[i]);
        out[n + j] = r1(y[i]);
    }
    n as u32
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_matches_core() {
        assert_eq!(super::version(), mzlab_core::version());
    }

    #[test]
    fn trace_is_rt_then_y_rounded() {
        let mut out = [0.0; 4];
        let n = super::write_trace(&[0.123456, 1.0], &[10.04, 20.06], 0, &mut out);
        assert_eq!(n, 2);
        assert_eq!(out, [0.1235, 1.0, 10.0, 20.1]);
        assert_eq!(
            super::write_trace(&[0.0, 1.0], &[1.0, 2.0], 0, &mut [0.0; 3]),
            0
        );
    }
}
