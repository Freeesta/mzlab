//! `native_samples`: the samples of a dataset, each with why it has nothing to show (`readers::samples_json`, the listing the app gives too);
//! `native_ranges`: the bytes of a big `.wiff` pair one sample needs, so that the page copies only those (the partial identity of that sample goes with them).

use std::cell::RefCell;
use std::rc::Rc;

use js_sys::Function;
use mzlab_core::idxread::ReadAt;
use mzlab_core::readers::Limits;
use mzlab_core::wiff::reader::{open_bare, PARTIAL};
use wasm_bindgen::prelude::*;

use crate::error_json;
use crate::natives::Members;

/// The samples of a dataset (`names`, `sizes`, `read(member, offset, length)`): the JSON of `mzlab_core::readers::samples_json`.
#[wasm_bindgen]
pub fn native_samples(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
) -> Result<String, JsValue> {
    let mut l = Members::new(names, sizes, read)?.open(false)?;
    Ok(mzlab_core::readers::samples_json(&mut *l))
}

/// A member that notes what is read from it.
struct Noting(Box<dyn ReadAt>, Rc<RefCell<Vec<(u64, u64)>>>);

impl ReadAt for Noting {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        self.1.borrow_mut().push((off, buf.len() as u64));
        self.0.read_at(off, buf)
    }
}

/// The reads sorted and joined where they touch or overlap.
fn joined(mut v: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    v.sort_unstable();
    let mut out: Vec<(u64, u64)> = Vec::new();
    for (o, n) in v {
        match out.last_mut() {
            Some(l) if o <= l.0 + l.1 => l.1 = l.1.max(o + n - l.0),
            _ => out.push((o, n)),
        }
    }
    out
}

/// What sample `c` of a `.wiff` pair needs of each member, as JSON `{"token", "ranges": [[[offset, length], ...] per member]}`: the pair is opened for that sample alone, its identity taken
/// and its index, and chromatograms read (the identity reads every stream of the sample, the DAD included), and every read is noted. The page copies those ranges (the rest of the members stays out of its storage) and gives `token` back as the
/// identity of the copy, which is not a hash of the whole file.
#[wasm_bindgen]
pub fn native_ranges(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
    c: u32,
) -> Result<String, JsValue> {
    let m = Members::new(names, sizes, read)?;
    let mut noted = Vec::new();
    let mut fs = m.fileset();
    for mb in fs.membri.iter_mut() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let inner = std::mem::replace(&mut mb.dati, Box::new(Vec::<u8>::new()));
        mb.dati = Box::new(Noting(inner, log.clone()));
        noted.push(log);
    }
    if let Some(w) = fs
        .membri
        .iter_mut()
        .find(|x| x.nome.to_lowercase().ends_with(".wiff"))
    {
        w.sha1 = Some(PARTIAL.to_string());
    }
    let c = c as usize;
    let fail = |e: mzlab_core::Error| error_json(&e);
    let mut r = open_bare(fs, &Limits::sito(), c).map_err(fail)?;
    let token = r.source_identity(c).map_err(fail)?.unwrap_or_default();
    r.index(c).map_err(fail)?;
    r.chromatograms(c).map_err(fail)?;
    let per: Vec<String> = noted
        .iter()
        .map(|l| {
            let v: Vec<String> = joined(l.borrow().clone())
                .iter()
                .map(|(o, n)| format!("[{o},{n}]"))
                .collect();
            format!("[{}]", v.join(","))
        })
        .collect();
    Ok(format!(
        "{{\"token\":\"{token}\",\"ranges\":[{}]}}",
        per.join(",")
    ))
}
