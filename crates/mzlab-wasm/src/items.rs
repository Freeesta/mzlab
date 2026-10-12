//! The methods (`.dam`) of the session and the routes of one item that read how it was acquired (`api/method`, `api/method_warnings`, `api/analog`,
//! `api/delay`, `api/chrom?kind=pda`, `api/gradient`): the same answers as `mzlab/app.py`, from the index and the mzML (low resolution only).
use super::{error_json, Engine, JsReader};
use mzlab_core::gradient::{self, Gradient};
use mzlab_core::json::{self, Out};
use mzlab_core::labmethods::{self, Lab};
use mzlab_core::mzmlhead::{Chrom, Mzml};
use wasm_bindgen::prelude::*;
use js_sys::Function;

#[wasm_bindgen]
impl Engine {
    /// Forgets every loaded method (the worker loads them again, oldest first, with `lab_put`).
    pub fn lab_clear(&mut self) {
        self.labs.clear();
    }

    /// Loads (or replaces) the method `name` (the bytes of a `.dam`); a file that cannot be read is kept with the reason.
    pub fn lab_put(&mut self, name: &str, data: &[u8]) {
        self.labs.retain(|(l, _)| l.name != name);
        self.labs.push((Lab::read(name, data.to_vec()), data.len() as u64));
    }

    /// `App.methods()`: `[{name, size, info}]`, oldest first.
    pub fn lab_list(&self) -> String {
        let mut o = Out::default();
        o.raw("[");
        for (i, (l, size)) in self.labs.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            l.write_listed(*size, &mut o);
        }
        o.raw("]");
        o.0
    }

    /// `/api/method_warnings`; `have` = JSON array of the experiment types of the open items (sorted).
    pub fn lab_warnings(&self, have: &str) -> Result<String, JsValue> {
        let v = json::parse(have.as_bytes()).map_err(|e| error_json(&e))?;
        let have: Vec<String> = v.arr().iter().filter_map(|x| x.str().map(String::from)).collect();
        Ok(labmethods::warnings_json(&self.labs(), &have))
    }

    /// `/api/method` of the item `item` (`name` or `name#tag`) of the file at `slot` (`size` bytes of mzML read through `read`).
    pub fn lab_method(&mut self, slot: u32, item: &str, size: f64, read: Function) -> Result<String, JsValue> {
        let data = self.item_data(slot, item, size, read)?;
        labmethods::method_json(&data, &self.labs(), item).map_err(|e| error_json(&e))
    }

    /// `App.gradient` without the typed one: the pump program of the method preselected for the item, `"null"` when it has none.
    pub fn lab_gradient(&mut self, slot: u32, item: &str, size: f64, read: Function) -> Result<String, JsValue> {
        let data = self.item_data(slot, item, size, read)?;
        let labs = self.labs();
        let pick = labmethods::pick(&data, &labs, item).map_err(|e| error_json(&e))?;
        match gradient::from_dam(labmethods::lc_of(&labs, pick)).map_err(|e| error_json(&e))? {
            Some(g) => Ok(text(&g)),
            None => Ok("null".to_string()),
        }
    }

    /// `/api/analog`: trace `i` of the file at `slot`, `n` points at most per extreme.
    pub fn item_analog(&mut self, slot: u32, size: f64, read: Function, i: i32, n: i32) -> Result<String, JsValue> {
        let (mut m, chroms) = self.chroms(slot, size, read)?;
        mzlab_core::itemroutes::analog(&mut m, &chroms, i as i64, n as i64).map_err(|e| error_json(&e))
    }

    /// `/api/chrom?kind=pda`: the UV trace of the file at `slot`.
    pub fn item_pda(&mut self, slot: u32, size: f64, read: Function) -> Result<String, JsValue> {
        let (mut m, chroms) = self.chroms(slot, size, read)?;
        mzlab_core::itemroutes::pda_trace(&mut m, &chroms).map_err(|e| error_json(&e))
    }

    /// `/api/delay` of the file at `slot`.
    pub fn item_delay(&mut self, slot: u32, size: f64, read: Function) -> Result<String, JsValue> {
        let (mut m, chroms) = self.chroms(slot, size, read)?;
        let r = self.idx.get_mut(&slot).ok_or_else(no_index)?;
        r.delay(&mut m, &chroms).map_err(|e| error_json(&e))
    }
}

impl Engine {
    fn labs(&self) -> Vec<Lab> {
        self.labs.iter().map(|(l, _)| l.clone()).collect()
    }

    fn chroms(&mut self, slot: u32, size: f64, read: Function) -> Result<(Mzml<JsReader>, Vec<Chrom>), JsValue> {
        let r = self.idx.get(&slot).ok_or_else(no_index)?;
        let end = r.idx.scans.last().map_or(0, |s| s.end);
        let mut m = Mzml::new(JsReader { read }, size as u64);
        let chroms = m.chromatograms(end).map_err(|e| error_json(&e))?;
        Ok((m, chroms))
    }

    /// `Item.method()` as JSON.
    fn item_data(&mut self, slot: u32, item: &str, size: f64, read: Function) -> Result<String, JsValue> {
        let r = self.idx.get(&slot).ok_or_else(no_index)?;
        let mut m = Mzml::new(JsReader { read }, size as u64);
        mzlab_core::itemroutes::item_method(&r.idx, &mut m, item).map_err(|e| error_json(&e))
    }
}

fn text(g: &Gradient) -> String {
    let mut o = Out::default();
    g.write(&mut o);
    o.0
}

/// `/api/gradient_parse` and the text/model forms of `POST /api/gradient`: `fmt` = `course`, `chromeleon` or `csv` (with `text`), or `model`
/// (`text` is the JSON of the model); the gradient model as JSON.
#[wasm_bindgen]
pub fn gradient_make(fmt: &str, text_in: &str) -> Result<String, JsValue> {
    let g = match fmt {
        "course" => gradient::course(),
        "chromeleon" => mzlab_core::chromeleon::parse(text_in),
        "model" => json::parse(text_in.as_bytes()).and_then(|v| gradient::model_from_json(&v)),
        _ => gradient::parse_csv(text_in),
    };
    g.map(|g| text(&g)).map_err(|e| error_json(&e))
}

/// An engine error as a JS value; the slot has no index open.
fn no_index() -> JsValue {
    error_json(&mzlab_core::Error::new("error.badIndex"))
}
