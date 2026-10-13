//! The files of a maker read as they are (`.wiff` + `.wiff.scan`, the members of a `.raw` or of a `.D` folder): no mzML is written. A dataset is a
//! `FileSet` whose members JS reads by offset (`read(member, offset, length)`); the engine opens it with the readers of the registry (`lettori::apri`),
//! builds the `.mzidx` of one sample straight from the reader, and answers the routes that read the file besides its index from the reader and the
//! `guscio` of the sample (the mzML without its spectra: header, chromatograms, polarities). «Scarica .mzML» writes the whole mzML on request.
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use js_sys::Function;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{self, FileSet, Lettore, Limiti, Membro};
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::{lettori_centroidi, lettori_mzml, mzidx, upload};
use wasm_bindgen::prelude::*;

use crate::{error_json, BlockWriter, Engine, JsOutput, JsReader};

/// Readers kept open at once (a `.wiff` holds its container in memory).
const OPEN: usize = 4;
/// Bytes of `guscio` kept at once.
const SHELLS: usize = 32 << 20;

/// Member `i` of a dataset, read through `read(i, offset, length) -> Uint8Array`.
struct JsMember {
    read: Function,
    i: u32,
}

impl ReadAt for JsMember {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        let bad = || mzlab_core::Error::new("error.badIndex");
        let v = self
            .read
            .call3(
                &JsValue::NULL,
                &self.i.into(),
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
}

/// The members of a dataset as the page describes them: names (relative to the dataset), sizes, the read function.
struct Membri {
    names: Vec<String>,
    sizes: Vec<f64>,
    read: Function,
}

impl Membri {
    fn new(names: js_sys::Array, sizes: &[f64], read: Function) -> Result<Membri, JsValue> {
        let names: Vec<String> = names
            .iter()
            .map(|v| v.as_string().unwrap_or_default())
            .collect();
        if names.len() != sizes.len() || names.is_empty() {
            return Err(error_json(
                &mzlab_core::Error::new("err.file.notAccepted")
                    .with("name", names.first().cloned().unwrap_or_default()),
            ));
        }
        Ok(Membri {
            names,
            sizes: sizes.to_vec(),
            read,
        })
    }

    fn fileset(&self) -> FileSet {
        let membri = self
            .names
            .iter()
            .zip(&self.sizes)
            .enumerate()
            .map(|(i, (n, &s))| Membro {
                nome: n.clone(),
                dim: s as u64,
                dati: Box::new(JsMember {
                    read: self.read.clone(),
                    i: i as u32,
                }),
            })
            .collect();
        FileSet { membri }
    }

    /// The reader of the dataset; with `centroid` its profile scans become centroids (as the conversion of a `.wiff` does by default).
    fn open(&self, centroid: bool) -> Result<Box<dyn Lettore>, JsValue> {
        let l = lettori::apri(self.fileset(), &Limiti::sito()).map_err(|e| error_json(&e))?;
        Ok(if centroid {
            lettori_centroidi::centroidato(l)
        } else {
            l
        })
    }
}

struct Dataset {
    membri: Membri,
    centroid: bool,
    l: Option<Box<dyn Lettore>>,
    used: u64,
}

/// The datasets of the session and which sample of which dataset each slot is.
#[derive(Default)]
pub(crate) struct Nativi {
    sets: HashMap<String, Dataset>,
    slots: HashMap<u32, (String, usize)>,
    shells: HashMap<(String, usize), Rc<Vec<u8>>>,
    order: VecDeque<(String, usize)>,
    clock: u64,
}

/// The bytes the routes read for a slot: the mzML through JS, or the `guscio` of a sample with its reader (`ReadAt::lettore`).
pub(crate) enum Src<'a> {
    Js(JsReader),
    Nat {
        g: Rc<Vec<u8>>,
        l: &'a mut dyn Lettore,
        c: usize,
    },
}

impl ReadAt for Src<'_> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        match self {
            Src::Js(r) => r.read_at(off, buf),
            Src::Nat { g, .. } => {
                let s = g
                    .get(off as usize..off as usize + buf.len())
                    .ok_or_else(|| mzlab_core::Error::new("error.badIndex"))?;
                buf.copy_from_slice(s);
                Ok(())
            }
        }
    }
    fn lettore(&mut self) -> Option<(&mut dyn Lettore, usize)> {
        match self {
            Src::Js(_) => None,
            Src::Nat { l, c, .. } => Some((&mut **l, *c)),
        }
    }
}

/// The open reader of dataset `ds` (opened again when it was closed; the least used ones are closed beyond `OPEN`).
fn reader<'a>(
    sets: &'a mut HashMap<String, Dataset>,
    clock: &mut u64,
    ds: &str,
) -> Result<&'a mut dyn Lettore, JsValue> {
    *clock += 1;
    let open = sets.values().filter(|d| d.l.is_some()).count();
    if open >= OPEN && sets.get(ds).is_some_and(|d| d.l.is_none()) {
        if let Some(old) = sets
            .values_mut()
            .filter(|d| d.l.is_some())
            .min_by_key(|d| d.used)
        {
            old.l = None;
        }
    }
    let d = sets
        .get_mut(ds)
        .ok_or_else(|| error_json(&mzlab_core::Error::new("error.badIndex")))?;
    d.used = *clock;
    if d.l.is_none() {
        d.l = Some(d.membri.open(d.centroid)?);
    }
    match d.l.as_deref_mut() {
        Some(l) => Ok(l),
        None => Err(error_json(&mzlab_core::Error::new("error.badIndex"))),
    }
}

impl Nativi {
    pub(crate) fn forget(&mut self, slot: u32) {
        if let Some((ds, _)) = self.slots.remove(&slot) {
            if !self.slots.values().any(|(d, _)| *d == ds) {
                self.sets.remove(&ds);
                self.shells.retain(|(d, _), _| *d != ds);
                self.order.retain(|(d, _)| *d != ds);
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        *self = Nativi::default();
    }

    /// What the routes read for `slot`: the `guscio` and the reader of its sample when it is native, else the mzML through `read`; with its size.
    pub(crate) fn src(
        &mut self,
        slot: u32,
        read: Function,
        size: f64,
    ) -> Result<(Src<'_>, u64), JsValue> {
        let Some((ds, c)) = self.slots.get(&slot).cloned() else {
            return Ok((Src::Js(JsReader { read }), size as u64));
        };
        let key = (ds.clone(), c);
        let g = match self.shells.get(&key) {
            Some(g) => g.clone(),
            None => {
                let l = reader(&mut self.sets, &mut self.clock, &ds)?;
                let g = Rc::new(lettori_mzml::guscio(l, c).map_err(|e| error_json(&e))?);
                let mut held: usize = self.shells.values().map(|g| g.len()).sum();
                while held + g.len() > SHELLS {
                    let Some(old) = self.order.pop_front() else {
                        break;
                    };
                    held -= self.shells.remove(&old).map_or(0, |g| g.len());
                }
                self.shells.insert(key.clone(), g.clone());
                self.order.push_back(key);
                g
            }
        };
        let len = g.len() as u64;
        let l = reader(&mut self.sets, &mut self.clock, &ds)?;
        Ok((Src::Nat { g, l, c }, len))
    }

    /// The head of the mzML of `slot` (`mzmlhead::Mzml`) over `src`.
    pub(crate) fn mzml(
        &mut self,
        slot: u32,
        read: Function,
        size: f64,
    ) -> Result<Mzml<Src<'_>>, JsValue> {
        let (s, len) = self.src(slot, read, size)?;
        Ok(Mzml::new(s, len))
    }
}

#[wasm_bindgen]
impl Engine {
    /// Makes `slot` sample `c` of the dataset `ds` (`names`, `sizes`, `read(member, offset, length)`): the routes that read the file besides its index
    /// (`open_index`) read it through its reader. `centroid`: the profile scans become centroids (the same value the index was built with).
    #[allow(clippy::too_many_arguments)]
    pub fn native_attach(
        &mut self,
        slot: u32,
        ds: &str,
        names: js_sys::Array,
        sizes: &[f64],
        read: Function,
        c: u32,
        centroid: bool,
    ) -> Result<(), JsValue> {
        self.nat.forget(slot);
        if !self.nat.sets.contains_key(ds) {
            let membri = Membri::new(names, sizes, read)?;
            self.nat.sets.insert(
                ds.to_string(),
                Dataset {
                    membri,
                    centroid,
                    l: None,
                    used: 0,
                },
            );
        }
        self.nat.slots.insert(slot, (ds.to_string(), c as usize));
        Ok(())
    }

    /// Forgets the dataset of `slot` (with the index: `close`).
    pub fn native_detach(&mut self, slot: u32) {
        self.nat.forget(slot);
    }

    /// `metodo/1` of the sample of `slot` from its reader; `""` when `slot` is not native.
    pub fn native_metodo(&mut self, slot: u32) -> Result<String, JsValue> {
        let Some((ds, c)) = self.nat.slots.get(&slot).cloned() else {
            return Ok(String::new());
        };
        let l = reader(&mut self.nat.sets, &mut self.nat.clock, &ds)?;
        l.metodo(c).map_err(|e| error_json(&e))
    }
}

/// The samples of a dataset (`names`, `sizes`, `read(member, offset, length)`): JSON `{"format": id, "samples": [{"id", "name", "scans"}]}`.
#[wasm_bindgen]
pub fn native_samples(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
) -> Result<String, JsValue> {
    use mzlab_core::json::Out;
    let l = Membri::new(names, sizes, read)?.open(false)?;
    let mut o = Out::default();
    o.raw("{\"format\":")
        .str(&format!("{:?}", l.formato()))
        .raw(",\"samples\":[");
    for (i, s) in l.campioni().iter().enumerate() {
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .int(i64::from(s.id))
            .raw(",\"name\":")
            .str(&s.nome)
            .raw(",\"scans\":")
            .int(s.scansioni as i64)
            .raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}

/// Builds the `.mzidx` of sample `c` of a dataset straight from its reader (version 2: the rows say where each scan lies in the members) and writes
/// it with `write(offset, bytes)`; `name` is the name of the item (JS follows the progress from the bytes it is asked for). Returns the statistics and
/// the SHA-256 of the dataset as JSON.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn build_mzidx_native(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
    c: u32,
    name: &str,
    centroid: bool,
    write: Function,
    budget_mb: u32,
) -> Result<String, JsValue> {
    let m = Membri::new(names, sizes, read)?;
    let mut fs = m.fileset();
    let impronte = upload::impronte(&mut fs).map_err(|e| error_json(&e))?;
    let l = lettori::apri(fs, &Limiti::sito()).map_err(|e| error_json(&e))?;
    let mut l = if centroid {
        lettori_centroidi::centroidato(l)
    } else {
        l
    };
    let c = c as usize;
    let p = upload::prepare_lettore(&mut *l, c, &impronte, name).map_err(|e| error_json(&e))?;
    let mut out = JsOutput { write };
    let st = mzidx::build_lettore(&p.table, &mut *l, c, &mut out, (budget_mb as usize) << 20)
        .map_err(|e| error_json(&e))?;
    Ok(format!(
        "{{\"scans\":{},\"peaks\":{},\"sections\":{},\"passes\":{},\"bytes\":{},\"sha256\":\"{}\"}}",
        st.scans, st.peaks, st.sections, st.passes, st.bytes, p.sha256
    ))
}

/// Sample `c` of a dataset as mzML («Scarica .mzML»), sent to `sink(Uint8Array)` piece by piece; `progress(done, total)` counts the spectra written.
/// `centroid` as in `build_mzidx_native`: the mzML has the peaks the index was built from.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn native_to_mzml(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
    c: u32,
    centroid: bool,
    sink: Function,
    progress: Function,
) -> Result<(), JsValue> {
    let mut l = Membri::new(names, sizes, read)?.open(centroid)?;
    let mut out = BlockWriter {
        sink,
        buf: Vec::new(),
    };
    let mut tell = |done: usize, total: usize| {
        let _ = progress.call2(
            &JsValue::NULL,
            &(done as f64).into(),
            &(total as f64).into(),
        );
    };
    lettori_mzml::scrivi(&mut *l, c as usize, &mut out, &mut tell).map_err(|e| error_json(&e))?;
    std::io::Write::flush(&mut out).map_err(|_| {
        error_json(&mzlab_core::Error::new("err.file.notAccepted").with("detail", "write"))
    })
}
