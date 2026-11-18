//! The module `wiffwrite` of the page: the read-only `.wiff` inventory and verify of `mzlab-cli wiff list|verify` (`mzlab_core::wiff::inventory`), reading the pairs through the
//! page's `read(member, offset, length) -> Uint8Array` callback (the pairs lie one after the other: member 2k is the `.wiff`, 2k+1 its `.wiff.scan`). It returns the
//! JSON text the command prints, byte for byte. A module of its own so that the readers modules and the engine do not carry it.
use js_sys::Function;
use mzlab_core::idxread::ReadAt;
use mzlab_core::readers::{FileSet, Limits, Member};
use wasm_bindgen::prelude::*;

/// Bytes fetched from JS at once for the small reads.
const AHEAD: usize = 256 << 10;

/// Member `i` of the call, read through `read`; the small reads are served from the last block read.
struct JsMember {
    read: Function,
    i: u32,
    dim: u64,
    start: u64,
    block: Vec<u8>,
}

fn fetch(read: &Function, i: u32, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
    let bad = || mzlab_core::Error::new("error.badIndex");
    let v = read
        .call3(
            &JsValue::NULL,
            &i.into(),
            &(off as f64).into(),
            &(buf.len() as f64).into(),
        )
        .map_err(|_| bad())?;
    let a: js_sys::Uint8Array = v.dyn_into().map_err(|_| bad())?;
    if a.length() as usize != buf.len() {
        return Err(bad());
    }
    a.copy_to(buf);
    Ok(())
}

impl ReadAt for JsMember {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        let inside =
            off >= self.start && off + buf.len() as u64 <= self.start + self.block.len() as u64;
        if !inside {
            let n = self.dim.saturating_sub(off).min(AHEAD as u64) as usize;
            if buf.len() >= AHEAD || n < buf.len() {
                return fetch(&self.read, self.i, off, buf);
            }
            self.block.resize(n, 0);
            if let Err(e) = fetch(&self.read, self.i, off, &mut self.block) {
                self.block.clear();
                return Err(e);
            }
            self.start = off;
        }
        let from = (off - self.start) as usize;
        buf.copy_from_slice(&self.block[from..from + buf.len()]);
        Ok(())
    }
}

/// The pairs `names` / `sizes` (two members each) as file sets.
fn sets(names: &[String], sizes: &[f64], read: &Function) -> Vec<FileSet> {
    names
        .chunks(2)
        .zip(sizes.chunks(2))
        .enumerate()
        .map(|(k, (n, d))| FileSet {
            membri: n
                .iter()
                .zip(d)
                .enumerate()
                .map(|(j, (nome, dim))| Member {
                    nome: nome.clone(),
                    dim: *dim as u64,
                    dati: Box::new(JsMember {
                        read: read.clone(),
                        i: (2 * k + j) as u32,
                        dim: *dim as u64,
                        start: 0,
                        block: Vec::new(),
                    }),
                    sha1: None,
                })
                .collect(),
        })
        .collect()
}

fn err(e: mzlab_core::Error) -> JsValue {
    JsValue::from_str(&format!("{{\"error_key\":\"{}\",\"params\":{{}}}}", e.key))
}

/// The name of this module (as `family()` of the readers modules).
#[wasm_bindgen]
pub fn family() -> String {
    "wiffwrite".into()
}

/// `wiff.list/1` of the pairs whose members are `names` and `sizes` (`.wiff`, `.wiff.scan`, ...): the text `mzlab-cli wiff list` prints. `full` is `--inventory`.
#[wasm_bindgen]
pub fn wiff_list(
    names: Vec<String>,
    sizes: Vec<f64>,
    full: bool,
    read: Function,
) -> Result<String, JsValue> {
    mzlab_core::wiff::inventory::list(sets(&names, &sizes, &read), &Limits::sito(), full)
        .map(|r| r.0 + "\n")
        .map_err(err)
}

/// `wiff.verify/1`: the first pair against the others (the sources): the text `mzlab-cli wiff verify` prints.
#[wasm_bindgen]
pub fn wiff_verify(names: Vec<String>, sizes: Vec<f64>, read: Function) -> Result<String, JsValue> {
    let mut all = sets(&names, &sizes, &read);
    if all.len() < 2 {
        return Err(err(
            mzlab_core::Error::new("err.file.wiff").with("detail", "no source")
        ));
    }
    let out = all.remove(0);
    mzlab_core::wiff::inventory::verify(out, all, &Limits::sito())
        .map(|r| r.0 + "\n")
        .map_err(err)
}
