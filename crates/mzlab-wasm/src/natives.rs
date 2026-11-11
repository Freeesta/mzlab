//! The files of a maker read as they are (`.wiff` + `.wiff.scan`, the members of a `.raw` or of a `.D` folder): no mzML is written. A dataset is a
//! `FileSet` whose members JS reads by offset (`read(member, offset, length)`); the engine opens it with the readers of the registry (`readers::apri`),
//! builds the `.mzidx` of one sample straight from the reader, and answers the routes that read the file besides its index from the reader and the
//! `guscio` of the sample (the mzML without its spectra: header, chromatograms, polarities). «Scarica .mzML» writes the whole mzML on request.
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use js_sys::{Function, Uint8Array};
use mzlab_core::idxread::ReadAt;
use mzlab_core::mzidx_spill::Spill;
use mzlab_core::readers::{self, FileSet, Limits, Member, Reader};
use mzlab_core::source::ReaderSource;
use mzlab_core::{readers_centroids, readers_mzml, upload};
use wasm_bindgen::prelude::*;

mod member;

use crate::external::{module_error, ExternalReader, JsModule};
use crate::{error_json, Accorpa, BlockWriter, JsReader};

/// Readers kept open at once (a `.wiff` holds its container in memory).
const OPEN: usize = 4;
/// Bytes of `guscio` kept at once.
pub(crate) const SHELLS: usize = 32 << 20;

thread_local! {
    /// The function of the page that opens a dataset on a readers module (`native_external`).
    static EXTERNAL: std::cell::RefCell<Option<Function>> = const { std::cell::RefCell::new(None) };
}

/// Registers `open(names, sizes, read)`: the page's function that, for a dataset the engine has no reader for (a Thermo `.raw`), opens it on the readers module of its family and
/// returns the object of the session (`campioni`, `indice`, `scansione`, `intestazione`, `cromatogrammi`, `cromatogramma`, `metodo`, `testata`, `chiudi`), or `null` when no module reads it.
/// The engine then reads the dataset through `ExternalReader` like any other (`Members::open`, `build_mzidx_native`, `native_srm`, `native_to_mzml`).
#[wasm_bindgen]
pub fn native_external(open: Function) {
    EXTERNAL.with(|f| *f.borrow_mut() = Some(open));
}

/// The members of a dataset as the page describes them: names (relative to the dataset), sizes, the read function.
pub(crate) struct Members {
    names: Vec<String>,
    sizes: Vec<f64>,
    read: Function,
}

impl Members {
    pub(crate) fn new(
        names: js_sys::Array,
        sizes: &[f64],
        read: Function,
    ) -> Result<Members, JsValue> {
        let names: Vec<String> = names.iter().flat_map(|v| v.as_string()).collect();
        if names.len() != sizes.len() || names.is_empty() {
            return Err(error_json(
                &mzlab_core::Error::new("err.file.notAccepted")
                    .with("name", names.first().cloned().unwrap_or_default()),
            ));
        }
        Ok(Members {
            names,
            sizes: sizes.to_vec(),
            read,
        })
    }

    pub(crate) fn fileset(&self) -> FileSet {
        let membri = self
            .names
            .iter()
            .zip(&self.sizes)
            .enumerate()
            .map(|(i, (n, &s))| Member {
                nome: n.clone(),
                dim: s as u64,
                dati: member::member(&self.read, i as u32, s as u64),
                sha1: None,
            })
            .collect();
        FileSet { membri }
    }
}

pub(crate) struct Dataset {
    pub(crate) members: Members,
    pub(crate) centroid: bool,
    pub(crate) l: Option<Box<dyn Reader>>,
    pub(crate) used: u64,
}

impl Members {
    /// The reader of `fs` (this dataset's members): the engine's own, and when none recognises it the one of the readers module the page registered with `native_external`.
    fn open_fs(&self, fs: FileSet) -> mzlab_core::Result<Box<dyn Reader>> {
        let sha1: Vec<String> = fs
            .membri
            .iter()
            .map(|m| m.sha1.clone().unwrap_or_default())
            .collect();
        match readers::apri(fs, &Limits::sito()) {
            Err(e) if e.key == "err.file.notAccepted" => self.external(&sha1).unwrap_or(Err(e)),
            r => r,
        }
    }

    /// The session of the module for this dataset; `None` when the page registered no function or no module reads it.
    /// `sha1`: the SHA-1 of each member when known (empty text when not), for the header of the mzML.
    fn external(&self, sha1: &[String]) -> Option<mzlab_core::Result<Box<dyn Reader>>> {
        let f = EXTERNAL.with(|f| f.borrow().clone())?;
        let names: js_sys::Array = self.names.iter().map(|n| JsValue::from_str(n)).collect();
        let sizes = js_sys::Float64Array::from(&self.sizes[..]);
        let sha1: js_sys::Array = sha1.iter().map(|n| JsValue::from_str(n)).collect();
        let session = match f.call4(&JsValue::NULL, &names, &sizes, &self.read, &sha1) {
            Ok(v) if v.is_null() || v.is_undefined() => return None,
            Ok(v) => v,
            Err(e) => {
                return Some(Err(e.as_string().map_or_else(
                    || mzlab_core::Error::new("err.module.reply").with("detail", "open"),
                    |t| module_error(&t),
                )))
            }
        };
        Some(ExternalReader::new(JsModule::new(session)).map(|l| Box::new(l) as Box<dyn Reader>))
    }

    /// The reader of the dataset; with `centroid` its profile scans become centroids (as the conversion of a `.wiff` does by default).
    pub(crate) fn open(&self, centroid: bool) -> Result<Box<dyn Reader>, JsValue> {
        let l = self.open_fs(self.fileset()).map_err(|e| error_json(&e))?;
        Ok(if centroid {
            readers_centroids::centroidato(l)
        } else {
            l
        })
    }
}

/// The datasets of the session and which sample of which dataset each slot is.
#[derive(Default)]
pub(crate) struct Natives {
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
        l: &'a mut dyn Reader,
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
    fn reader(&mut self) -> Option<(&mut dyn Reader, usize)> {
        match self {
            Src::Js(_) => None,
            Src::Nat { l, c, .. } => Some((&mut **l, *c)),
        }
    }
}

/// The open reader of dataset `ds` (opened again when it was closed; the least used ones are closed beyond `OPEN`).
pub(crate) fn reader(n: &mut Natives, ds: String) -> Result<&mut dyn Reader, JsValue> {
    n.clock += 1;
    let open = n.sets.values().filter(|d| d.l.is_some()).count();
    if open >= OPEN
        && n.sets.get(&ds).is_some_and(|d| d.l.is_none())
        && let Some(old) = n
            .sets
            .values_mut()
            .filter(|d| d.l.is_some())
            .min_by_key(|d| d.used)
    {
        old.l = None;
    }
    let d = n
        .sets
        .get_mut(&ds)
        .ok_or_else(|| error_json(&mzlab_core::Error::new("error.badIndex")))?;
    d.used = n.clock;
    if d.l.is_none() {
        d.l = Some(d.members.open(d.centroid)?);
    }
    match d.l.as_deref_mut() {
        Some(l) => Ok(l),
        None => Err(error_json(&mzlab_core::Error::new("error.badIndex"))),
    }
}

/// The fingerprints of a member taken while it is copied: `aggiorna(piece)` for each piece in file order, then `fine()` gives `[sha256, sha1, probe]`
/// (`undefined` when the pieces do not add up to `size`). The page passes them on to `build_mzidx_native` and to the reader of the header of the mzML.
#[wasm_bindgen]
pub struct Passata(Option<upload::Passata>);

#[wasm_bindgen]
impl Passata {
    #[wasm_bindgen(constructor)]
    pub fn new(size: f64, senza: Option<bool>, solo: Option<bool>) -> Passata {
        Passata(Some(upload::Passata::scelta(size as u64, senza, solo)))
    }
    pub fn aggiorna(&mut self, piece: &[u8]) {
        self.0.as_mut().unwrap().aggiorna(piece);
    }
    pub fn fine(self) -> Option<js_sys::Array> {
        let (a, b, c) = self.0?.fine()?;
        Some([a, b, c].iter().map(|x| JsValue::from_str(x)).collect())
    }
}

/// `f(offset, bytes)` with `bytes` a view of this module's memory, not a copy: valid during the call only (a later allocation can move or reuse it),
/// so `f` writes it at once and keeps nothing (`writeFull` of the worker, a sync OPFS write; `fs.writeSync` in Node). `what` names a refused write.
/// Made with the safe `wasm_bindgen::memory()` (the crate forbids `unsafe`); `data` outlives the call, and the callbacks never call the engine, so nothing moves meanwhile.
fn lend(f: &Function, offset: u64, data: &[u8], what: &str) -> mzlab_core::Result<()> {
    let bad = || mzlab_core::Error::new("err.file.index").with("detail", what);
    let mem: js_sys::WebAssembly::Memory = wasm_bindgen::memory().unchecked_into();
    let (at, n) = (data.as_ptr() as usize, data.len());
    let (at, n) = u32::try_from(at)
        .ok()
        .zip(u32::try_from(n).ok())
        .ok_or_else(bad)?;
    let v = Uint8Array::new_with_byte_offset_and_length(&mem.buffer(), at, n);
    f.call2(&JsValue::NULL, &(offset as f64).into(), &v)
        .map(|_| ())
        .map_err(|_| bad())
}

/// Writes to JS at offsets: `write(offset, Uint8Array)` (the worker puts them in a file of OPFS), the bytes lent (`lend`).
pub(crate) struct JsOutput {
    pub(crate) write: Function,
}

impl mzlab_core::mzidx::Output for JsOutput {
    fn write_at(&mut self, offset: u64, data: &[u8]) -> mzlab_core::Result<()> {
        lend(&self.write, offset, data, "write")
    }
}

/// The scratch file of the index builder (`mzidx_spill`): `put(offset, Uint8Array)` (lent, `lend`) and `get(offset, length) -> Uint8Array` (a second file of OPFS in the worker).
pub(crate) struct JsSpill {
    pub(crate) put: Function,
    pub(crate) get: Function,
}

impl Spill for JsSpill {
    fn put(&mut self, offset: u64, data: &[u8]) -> mzlab_core::Result<()> {
        lend(&self.put, offset, data, "spill write")
    }
    fn get(&mut self, offset: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        let bad = || mzlab_core::Error::new("err.file.index").with("detail", "spill read");
        let v = self
            .get
            .call2(
                &JsValue::NULL,
                &(offset as f64).into(),
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

/// Builds the `.mzidx` of sample `c` of a dataset straight from its reader (version 2: the rows say where each scan lies in the members) and writes
/// it with `write(offset, bytes)`; `name` is the name of the item. `phase(n)` is told when a phase begins (1 the members are read for their SHA-256,
/// 4 the reader opens the dataset, 2 the scans are read and indexed, 3 the chromatograms and the index are written); `step(done, total)`, when given, is told how many scans of sample `c` have been decoded (`counted.rs`), else JS follows the bytes read.
/// `write_cen(offset, bytes)`, when the page wants it: if the first MS1 scan is profile (and `centroid` is not asked) the index with the scans as centroids (the `.cen.mzidx` of mzFinder) is built from the SAME decoding of the scans and written there (`"cen"` in the answer holds its statistics); otherwise nothing is written to it.
/// Returns the statistics, the SHA-256 of the dataset, `missing` (the scans a truncated data file lacks or a `.raw` lists without their event, `Reader::losses`) and its `cause` as JSON. `prints`: `[sha256, sha1, probe]` of each member if the page already has them (`Passata`), or a function called once the first pass is done that gives them (the copy's hash still running) or null; a missing or empty value: they are taken here. `spill_put(offset, bytes)` and `spill_get(offset, length)`: a scratch file for the peaks that do not fit in the budget (the scans are then decoded once). `cap_mb`: the budget of the instance (`MemBudget::from_request`; absent or 0 the default).
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
    prints: JsValue,
    spill_put: Option<Function>,
    spill_get: Option<Function>,
    write_cen: Option<Function>,
    step: Option<Function>,
    cap_mb: Option<f64>,
) -> Result<String, JsValue> {
    let tell = |n: u32| drop(phase.call1(&JsValue::NULL, &n.into()));
    let m = Members::new(names, sizes, read)?;
    let mut fs = m.fileset();
    let fingerprints = crate::fingerprints::Fingerprints::new(&mut fs, &prints, &tell)?;
    let mut members = || fingerprints.given(|| m.fileset());
    let c = c as usize;
    let mut l = crate::counted::open(fs, |fs| m.open_fs(fs), &tell, step, c)?;
    let mut out = Accorpa::flussi(JsOutput { write });
    // peaks that do not fit in the budget go to the scratch file (a second file of OPFS) instead of being decoded again by every round; OPFS writes in 1 MiB blocks (`accorpa`)
    let mut spill = spill_put
        .zip(spill_get)
        .map(|(put, get)| Accorpa::new(JsSpill { put, get }));
    let spill = spill.as_mut().map(|s| s as &mut dyn Spill);
    // a profile sample that the page also wants as centroids: both indexes from one decoding of the scans
    let both = write_cen.filter(|_| !centroid && first_ms1_is_profile(&mut *l, c));
    // a sample whose index would not fit the budget of the page ends in the calm `error.memoryBudget` before a scan is decoded, not in a dead tab
    let cap = mzlab_core::budget::MemBudget::from_request(cap_mb.unwrap_or(0.0));
    mzlab_core::estimate::check_reader(
        &cap,
        &mut *l,
        c,
        (budget_mb as usize) << 20,
        1 + both.is_some() as usize,
    )
    .map_err(|e| error_json(&e))?;
    if let Some(wc) = both {
        return index_both(l, c, &mut members, name, out, wc, budget_mb, spill, &tell);
    }
    let mut l = if centroid {
        readers_centroids::centroidato(l)
    } else {
        l
    };
    tell(2);
    let (st, sha256) = upload::index_reader_with(
        &mut *l,
        c,
        &mut members,
        name,
        &mut out,
        (budget_mb as usize) << 20,
        spill,
        &mut |n| tell(n),
    )
    .map_err(|e| error_json(&e))?;
    out.fine().map_err(|e| error_json(&e))?;
    // the scans the file declares beyond the ones read (`Reader::losses`), asked once the index is built so its pass keeps the phases of the row
    let lost = l.losses(c).map_err(|e| error_json(&e))?;
    Ok(format!(
        "{{\"scans\":{},\"peaks\":{},\"sections\":{},\"passes\":{},\"bytes\":{},\"sha256\":\"{}\",\"missing\":{},\"cause\":\"{}\"}}",
        st.scans, st.peaks, st.sections, st.passes, st.bytes, sha256, lost.lost(), lost.cause
    ))
}

/// `build_mzidx_native` for a profile sample whose centroid index is wanted too: both from one decoding (`index_reader_twice`); `missing`: `Reader::losses` of sample `c`.
#[allow(clippy::too_many_arguments)]
fn index_both(
    l: Box<dyn mzlab_core::readers::Reader>,
    c: usize,
    members: &mut dyn FnMut() -> mzlab_core::Result<Vec<upload::Fingerprint>>,
    name: &str,
    mut out: Accorpa<JsOutput>,
    write_cen: Function,
    budget_mb: u32,
    spill: Option<&mut dyn Spill>,
    tell: &dyn Fn(u32),
) -> Result<String, JsValue> {
    let mut lc = readers_centroids::Centroidato::nuovo(l);
    let mut out_cen = Accorpa::flussi(JsOutput { write: write_cen });
    tell(2);
    let ((st, sha256), (sc, _)) = upload::index_reader_twice(
        &mut lc,
        c,
        members,
        name,
        &mut out,
        &mut out_cen,
        (budget_mb as usize) << 20,
        spill,
        &mut |n| tell(n),
    )
    .map_err(|e| error_json(&e))?;
    let fine = out.fine().and_then(|()| out_cen.fine());
    fine.map_err(|e| error_json(&e))?;
    let lost = lc.losses(c).map_err(|e| error_json(&e))?;
    Ok(format!(
        "{{\"scans\":{},\"peaks\":{},\"sections\":{},\"passes\":{},\"bytes\":{},\"sha256\":\"{}\",\"missing\":{},\"cause\":\"{}\",\"cen\":{{\"peaks\":{},\"bytes\":{}}}}}",
        st.scans, st.peaks, st.sections, st.passes, st.bytes, sha256, lost.lost(), lost.cause, sc.peaks, sc.bytes
    ))
}

/// The first MS1 scan of sample `c` is profile (false when it cannot be told: no MS1, a scan that does not decode).
fn first_ms1_is_profile(l: &mut dyn mzlab_core::readers::Reader, c: usize) -> bool {
    let Some(k) = l
        .index(c)
        .ok()
        .and_then(|v| v.iter().position(|e| e.livello == 1))
    else {
        return false;
    };
    l.scan(c, k).is_ok_and(|s| s.profile)
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
    let mut l = Members::new(names, sizes, read)?.open(centroid)?;
    let mut t = ReaderSource {
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
    compression: &str,
    sink: Function,
    progress: Function,
) -> Result<(), JsValue> {
    let mut l = Members::new(names, sizes, read)?.open(centroid)?;
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
    let opt = mzlab_core::mzmlwrite::compression_options(compression)
        .ok_or_else(|| error_json(&mzlab_core::Error::new("err.file.mzmlWrite")))?;
    readers_mzml::write_with(&mut *l, c as usize, &mut out, &mut tell, opt)
        .map_err(|e| error_json(&e))?;
    std::io::Write::flush(&mut out)
        .map_err(|_| error_json(&mzlab_core::Error::new("err.file.mzmlWrite")))
}
