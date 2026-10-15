//! The files of a maker read as they are (`.wiff` + `.wiff.scan`, the members of a `.raw` or of a `.D` folder): no mzML is written. A dataset is a
//! `FileSet` whose members JS reads by offset (`read(member, offset, length)`); the engine opens it with the readers of the registry (`lettori::apri`),
//! builds the `.mzidx` of one sample straight from the reader, and answers the routes that read the file besides its index from the reader and the
//! `guscio` of the sample (the mzML without its spectra: header, chromatograms, polarities). «Scarica .mzML» writes the whole mzML on request.
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use js_sys::Function;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{self, FileSet, Lettore, Limiti, Membro};
use mzlab_core::sorgente::SorgenteLettore;
use mzlab_core::{lettori_centroidi, lettori_mzml, upload};
use wasm_bindgen::prelude::*;

use crate::esterno::{errore_del_modulo, LettoreEsterno, ModuloJs};
use crate::{error_json, BlockWriter, JsOutput, JsReader};

/// Readers kept open at once (a `.wiff` holds its container in memory).
const OPEN: usize = 4;
/// Bytes of `guscio` kept at once.
pub(crate) const SHELLS: usize = 32 << 20;

thread_local! {
    /// The function of the page that opens a dataset on a readers module (`native_external`).
    static ESTERNO: std::cell::RefCell<Option<Function>> = const { std::cell::RefCell::new(None) };
}

/// Registers `open(names, sizes, read)`: the page's function that, for a dataset the engine has no reader for (a Thermo `.raw`), opens it on the readers module of its family and
/// returns the object of the session (`campioni`, `indice`, `scansione`, `intestazione`, `cromatogrammi`, `cromatogramma`, `metodo`, `testata`, `chiudi`), or `null` when no module reads it.
/// The engine then reads the dataset through `LettoreEsterno` like any other (`Membri::open`, `build_mzidx_native`, `native_srm`, `native_to_mzml`).
#[wasm_bindgen]
pub fn native_external(open: Function) {
    ESTERNO.with(|f| *f.borrow_mut() = Some(open));
}

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
pub(crate) struct Membri {
    names: Vec<String>,
    sizes: Vec<f64>,
    read: Function,
}

impl Membri {
    pub(crate) fn new(
        names: js_sys::Array,
        sizes: &[f64],
        read: Function,
    ) -> Result<Membri, JsValue> {
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
                sha1: None,
            })
            .collect();
        FileSet { membri }
    }
}

pub(crate) struct Dataset {
    pub(crate) membri: Membri,
    pub(crate) centroid: bool,
    pub(crate) l: Option<Box<dyn Lettore>>,
    pub(crate) used: u64,
}

impl Membri {
    /// The reader of `fs` (this dataset's members): the engine's own, and when none recognises it the one of the readers module the page registered with `native_external`.
    fn apri_fs(&self, fs: FileSet) -> mzlab_core::Result<Box<dyn Lettore>> {
        let sha1: Vec<String> = fs
            .membri
            .iter()
            .map(|m| m.sha1.clone().unwrap_or_default())
            .collect();
        match lettori::apri(fs, &Limiti::sito()) {
            Err(e) if e.key == "err.file.notAccepted" => self.esterno(&sha1).unwrap_or(Err(e)),
            r => r,
        }
    }

    /// The session of the module for this dataset; `None` when the page registered no function or no module reads it.
    /// `sha1`: the SHA-1 of each member when known (empty text when not), for the header of the mzML.
    fn esterno(&self, sha1: &[String]) -> Option<mzlab_core::Result<Box<dyn Lettore>>> {
        let f = ESTERNO.with(|f| f.borrow().clone())?;
        let names: js_sys::Array = self.names.iter().map(|n| JsValue::from_str(n)).collect();
        let sizes = js_sys::Float64Array::from(&self.sizes[..]);
        let sha1: js_sys::Array = sha1.iter().map(|n| JsValue::from_str(n)).collect();
        let sessione = match f.call4(&JsValue::NULL, &names, &sizes, &self.read, &sha1) {
            Ok(v) if v.is_null() || v.is_undefined() => return None,
            Ok(v) => v,
            Err(e) => {
                return Some(Err(e.as_string().map_or_else(
                    || mzlab_core::Error::new("err.file.notAccepted"),
                    |t| errore_del_modulo(&t),
                )))
            }
        };
        Some(LettoreEsterno::new(ModuloJs::new(sessione)).map(|l| Box::new(l) as Box<dyn Lettore>))
    }

    /// The reader of the dataset; with `centroid` its profile scans become centroids (as the conversion of a `.wiff` does by default).
    pub(crate) fn open(&self, centroid: bool) -> Result<Box<dyn Lettore>, JsValue> {
        let l = self.apri_fs(self.fileset()).map_err(|e| error_json(&e))?;
        Ok(if centroid {
            lettori_centroidi::centroidato(l)
        } else {
            l
        })
    }
}

/// The datasets of the session and which sample of which dataset each slot is.
#[derive(Default)]
pub(crate) struct Nativi {
    pub(crate) sets: HashMap<String, Dataset>,
    pub(crate) slots: HashMap<u32, (String, usize)>,
    pub(crate) shells: HashMap<(String, usize), Rc<Vec<u8>>>,
    pub(crate) order: VecDeque<(String, usize)>,
    pub(crate) clock: u64,
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
pub(crate) fn reader(n: &mut Nativi, ds: String) -> Result<&mut dyn Lettore, JsValue> {
    n.clock += 1;
    let open = n.sets.values().filter(|d| d.l.is_some()).count();
    if open >= OPEN && n.sets.get(&ds).is_some_and(|d| d.l.is_none()) {
        if let Some(old) = n
            .sets
            .values_mut()
            .filter(|d| d.l.is_some())
            .min_by_key(|d| d.used)
        {
            old.l = None;
        }
    }
    let d = n
        .sets
        .get_mut(&ds)
        .ok_or_else(|| error_json(&mzlab_core::Error::new("error.badIndex")))?;
    d.used = n.clock;
    if d.l.is_none() {
        d.l = Some(d.membri.open(d.centroid)?);
    }
    match d.l.as_deref_mut() {
        Some(l) => Ok(l),
        None => Err(error_json(&mzlab_core::Error::new("error.badIndex"))),
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

/// The fingerprints of a member taken while it is copied: `aggiorna(piece)` for each piece in file order, then `fine()` gives `[sha256, sha1, probe]`
/// (`undefined` when the pieces do not add up to `size`). The page passes them on to `build_mzidx_native` and to the reader of the header of the mzML.
/// The fingerprints of a member taken while it is copied: `aggiorna(piece)` for each piece in file order, then `fine()` gives `[sha256, sha1, probe]`
/// (`undefined` when the pieces do not add up to `size`). The page passes them on to `build_mzidx_native` and to the reader of the header of the mzML.
#[wasm_bindgen]
pub struct Passata(Option<upload::Passata>);

#[wasm_bindgen]
impl Passata {
    #[wasm_bindgen(constructor)]
    pub fn new(size: f64) -> Passata {
        Passata(Some(upload::Passata::new(size as u64)))
    }
    pub fn aggiorna(&mut self, piece: &[u8]) {
        self.0.as_mut().unwrap().aggiorna(piece);
    }
    pub fn fine(self) -> Option<js_sys::Array> {
        let (a, b, c) = self.0?.fine()?;
        Some([a, b, c].iter().map(|x| JsValue::from_str(x)).collect())
    }
}

/// Builds the `.mzidx` of sample `c` of a dataset straight from its reader (version 2: the rows say where each scan lies in the members) and writes
/// it with `write(offset, bytes)`; `name` is the name of the item. `phase(n)` is told when a phase begins (1 the members are read for their SHA-256,
/// 2 the scans are read and indexed, 3 the chromatograms and the index are written); JS follows how far each goes from the bytes it is asked for.
/// Returns the statistics and the SHA-256 of the dataset as JSON. `prints`: `[sha256, sha1, probe]` of each member if the page already has them (`Passata`).
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
    phase: Function,
    prints: Option<js_sys::Array>,
) -> Result<String, JsValue> {
    let tell = |n: u32| drop(phase.call1(&JsValue::NULL, &n.into()));
    let m = Membri::new(names, sizes, read)?;
    let mut fs = m.fileset();
    // the fingerprints taken while the members were copied (`Passata`): [sha256, sha1, probe] of each; without them (or not matching) the members are read here
    let given: Vec<String> = prints.map_or(vec![], |a| {
        a.iter()
            .map(|v| v.as_string().unwrap_or_default())
            .collect()
    });
    let given: Vec<(String, String, String)> = given
        .chunks_exact(3)
        .map(|c| (c[0].clone(), c[1].clone(), c[2].clone()))
        .collect();
    let impronte = match upload::impronte_note(&mut fs, &given) {
        Some(i) => i,
        None => {
            tell(1);
            upload::impronte(&mut fs).map_err(|e| error_json(&e))?
        }
    };
    let l = m.apri_fs(fs).map_err(|e| error_json(&e))?;
    let mut l = if centroid {
        lettori_centroidi::centroidato(l)
    } else {
        l
    };
    let c = c as usize;
    tell(2);
    let mut out = JsOutput { write };
    let (st, sha256) = upload::indicizza_lettore(
        &mut *l,
        c,
        &impronte,
        name,
        &mut out,
        (budget_mb as usize) << 20,
        &mut |n| tell(n),
    )
    .map_err(|e| error_json(&e))?;
    Ok(format!(
        "{{\"scans\":{},\"peaks\":{},\"sections\":{},\"passes\":{},\"bytes\":{},\"sha256\":\"{}\"}}",
        st.scans, st.peaks, st.sections, st.passes, st.bytes, sha256
    ))
}

/// The SRM traces of sample `c` of a dataset as `mzlab_core::srm::impacchetta` writes them, given once to `sink(Uint8Array)` (the MRM files of mzFinder: no mzML is made of them).
#[wasm_bindgen]
pub fn native_srm(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
    c: u32,
    centroid: bool,
    sink: Function,
) -> Result<(), JsValue> {
    let mut l = Membri::new(names, sizes, read)?.open(centroid)?;
    let mut t = SorgenteLettore {
        l: &mut *l,
        c: c as usize,
    };
    let v = mzlab_core::srm::leggi(&mut t).map_err(|e| error_json(&e))?;
    let bytes = js_sys::Uint8Array::from(&mzlab_core::srm::impacchetta(&v)[..]);
    sink.call1(&JsValue::NULL, &bytes).map(|_| ())
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
