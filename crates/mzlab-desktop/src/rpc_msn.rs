//! The ops of `rpc` that the site serves with the `msngrafo`, `msnregole` and centroid code of `rust-worker.js`: `msngrafo` (the fragmentation graph, its exports
//! and the library entry/search of `api/msn_grafo*`, `api/msnlib/*`), `mzml2cen` (a profile mzML as a centroid copy), `lr_index_cen` and `natsrm`.
//! `msngrafo` is the twin of `crates/mzlab-msngrafo-wasm` (same `mzlab-core` / `mzlab-feat` calls, files read through memory maps); `tests/parita_msn.rs` holds the bytes together.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use mzlab_core::budget::sha256_hex;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;
use mzlab_core::msngrafo::{
    aggiungi, cerca, con_interpretazione, da_richiesta, da_richiesta_export, esporta, grafo_json,
    library_text, opzioni_cerca, tabella_regole, table_text, Grafo, Opzioni, PPM_PERDITA,
};
use mzlab_core::routes::Query;
use mzlab_core::sorgente::SorgenteLettore;
use mzlab_core::{Error, Result};
use mzlab_feat::chem::strutture::strutture_testo;
use serde_json::{json, Value};

use crate::engine::{CountWrite, Picked, Progress, State};
use crate::nativi;
use crate::rpc::{io_error, map_file, text, Answer, Eng, MapAt};
use crate::store::Nat;

/// Default window of the cleavages (`ppm_struttura`), ppm; as in the `.wasm`.
const PPM_STRUTTURA: f64 = 10.0;

impl State {
    /// `msngrafo {slot, name, fid, query, esporta?, cerca?, libreria?, molecola?}`: the graph of the file `name` (its `.mzidx` and the mzML at `slot`), or its export as text,
    /// or the library text with an entry added (`libreria` given), or the library search (`cerca`). A sample of a maker's file has no mzML to read here: `err.route.unavailable`.
    pub(crate) fn op_msngrafo(&self, eng: &mut Eng, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        let slot = p["slot"].as_u64().unwrap_or(0) as u32;
        if eng.nat.is_native(slot) {
            return Err(Error::new("err.route.unavailable"));
        }
        let (mut mzml, _) =
            self.src_slot(&mut eng.nat, slot, p["fid"].as_u64().unwrap_or(0) as u32)?;
        let (idx_map, idx_size) = map_file(&self.indexer()?.path(s("name"))?)?;
        let mut q = Query::parse(s("query"));
        add_expected(&mut q);
        let name = s("name");
        let mut run = || graph(MapAt(idx_map.0.clone()), idx_size, &mut mzml, name, &q);
        if p["cerca"].as_bool().unwrap_or(false) {
            let o = opzioni_cerca(&|k| q.get(k).map(String::from))?;
            let (_, j, _) = run()?;
            return text(cerca(s("libreria"), &j, &o)?);
        }
        if p["libreria"].is_null() {
            return if p["esporta"].as_bool().unwrap_or(false) {
                text(export(&q, &mut run)?)
            } else {
                text(graph_text(&q, &mut run)?)
            };
        }
        let (g, j, _) = run()?;
        let (l, doppione) = aggiungi(s("libreria"), s("molecola"), &g, &j)?;
        let mut o = Out::default();
        o.raw("{\"libreria\":").str(&l).raw(",\"doppione\":");
        o.raw(if doppione { "true}" } else { "false}" });
        text(o.0)
    }

    /// `lr_index_cen {name}`: the index of a sample of a maker's file with its scans as centroids, as `<name>.cen.mzidx` apart from the one of the Data tab.
    pub(crate) fn lr_index_cen(&self, name: &str) -> Result<Answer> {
        let nat = self.nat_of(name)?;
        self.indexer()?.build_nat(
            &Nat {
                centroid: true,
                ..nat
            },
            &format!("{name}.cen"),
        )?;
        text("{}")
    }

    /// `natsrm {name}`: the SRM traces of a sample of a maker's file as `srm::impacchetta` writes them.
    pub(crate) fn nat_srm(&self, p: &Value) -> Result<Answer> {
        let nat = self.nat_of(p["name"].as_str().unwrap_or(""))?;
        let mut l = nativi::open(&nat.members, nat.centroid)?;
        let mut t = SorgenteLettore {
            l: &mut *l,
            c: nat.c,
        };
        Ok(Answer::Bytes(mzlab_core::srm::impacchetta(
            &mzlab_core::srm::leggi(&mut t)?,
        )))
    }

    /// `mzml2cen {name, pid}`: a profile mzML of the store as a copy with one point per peak in the temporary folder, registered like a picked file: `{"id", "size"}`;
    /// `{"none": true}` when the file has no profile spectrum. Progress by `rawprogress?id=pid`.
    pub(crate) fn mzml_to_cen(&self, p: &Value) -> Result<Answer> {
        let name = p["name"].as_str().unwrap_or("");
        let pid = p["pid"].as_u64().unwrap_or(0) as u32;
        let (map, size) = self.mzml_named(name)?;
        let progress = Arc::new(Progress::default());
        progress.total.store(size, Ordering::Relaxed);
        self.lock_inner().progress.insert(pid, progress.clone());
        let tmp = std::env::temp_dir().join(format!("mzlab-{}-c{}.mzML", std::process::id(), pid));
        let mut out = CountWrite {
            inner: std::io::BufWriter::new(std::fs::File::create(&tmp).map_err(|e| io_error(&e))?),
            progress: progress.clone(),
        };
        let pr = progress.clone();
        let mut tell = move |done: usize, total: usize| {
            pr.scans.store(done as u64, Ordering::Relaxed);
            pr.scans_total.store(total as u64, Ordering::Relaxed);
        };
        let bytes = map.0.clone();
        let did = mzlab_core::centroidi::mzml_to_centroids(
            || Ok(std::io::Cursor::new(bytes.bytes())),
            &mut out,
            name,
            &mut tell,
        );
        let flushed = std::io::Write::flush(&mut out);
        drop(out);
        let did = match did {
            Ok(d) => d,
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(e);
            }
        };
        flushed.map_err(|e| io_error(&e))?;
        if !did {
            let _ = std::fs::remove_file(&tmp);
            return text(json!({ "none": true }).to_string());
        }
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

    /// The reader description of the native sample `name`.
    fn nat_of(&self, name: &str) -> Result<Nat> {
        self.store
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            .and_then(|e| e.nat)
            .ok_or_else(|| Error::new("err.session.fileNotFound").with("name", name))
    }
}

/// `regole=1` with a `smiles`: the expectations of the structure rules go into the query as `attese` (the `.wasm` `msnregole`); a SMILES that does not parse is left to the graph.
fn add_expected(q: &mut Query) {
    if q.get("regole") != Some("1") || q.some("smiles").is_none() || q.some("attese").is_some() {
        return;
    }
    let Ok(a) = mzlab_feat::chem::regole::attese(q.get("smiles").unwrap_or(""), tabella_regole())
    else {
        return;
    };
    let mut o = Out::default();
    o.raw("[");
    for (i, x) in a.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("[")
            .str(&x.regola)
            .raw(",")
            .str(&x.formula);
        o.raw(if x.ione { ",1]" } else { ",0]" });
    }
    o.raw("]");
    q.0.push(("attese".to_string(), o.0));
}

type Computed = Result<(Grafo, String, Opzioni)>;

/// The graph of the query over the index and the mzML (`calcola_con` of the `.wasm`).
fn graph(mut idx: MapAt, idx_size: u64, mzml: &mut dyn ReadAt, name: &str, q: &Query) -> Computed {
    let req = da_richiesta(&|k| q.get(k).map(String::from))?;
    let mut bytes = vec![0u8; idx_size as usize];
    idx.read_at(0, &mut bytes)?;
    let sha = sha256_hex(&bytes);
    drop(bytes);
    let scans = Index::open(idx, idx_size)?.scans;
    let (g, j) = grafo_json(&scans, &mut DynRead(mzml), (name, &sha), (&req.0, &req.1))?;
    let j = with_keys(&g, j, &req.0, q)?;
    Ok((g, j, req.0))
}

/// A `&mut dyn ReadAt` as a sized `ReadAt`.
struct DynRead<'a>(&'a mut dyn ReadAt);

impl ReadAt for DynRead<'_> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.0.read_at(off, buf)
    }
}

/// The optional keys of the query on the graph text: `interpreta=1` and `rotture=1`, shared by the graph and its exports.
fn with_keys(g: &Grafo, mut j: String, o: &Opzioni, q: &Query) -> Result<String> {
    if q.get("interpreta") == Some("1") {
        let ppm = match q.some("ppm_perdita") {
            None => PPM_PERDITA,
            Some(v) => v
                .parse::<f64>()
                .ok()
                .filter(|p| p.is_finite() && *p > 0.0)
                .ok_or_else(|| Error::new("err.request.badNumber").with("name", "ppm_perdita"))?,
        };
        j = con_interpretazione(&j, g, ppm);
    }
    if q.get("rotture") == Some("1") {
        let num = |k: &str, d: f64| -> Result<f64> {
            match q.some(k) {
                None => Ok(d),
                Some(v) => v
                    .parse::<f64>()
                    .ok()
                    .filter(|p| p.is_finite() && *p >= 0.0)
                    .ok_or_else(|| Error::new("err.request.badNumber").with("name", k)),
            }
        };
        let par = mzlab_feat::chem::rotture::Parametri {
            ppm: num("ppm_struttura", PPM_STRUTTURA)?,
            h_max: num("h_max", 2.0)?.min(4.0) as u8,
            max_legami: num("max_legami", 2.0)?.clamp(1.0, 2.0) as u8,
        };
        j = mzlab_feat::chem::rotture::rotture_testo(g, &j, o.smiles.as_deref(), par)?;
    }
    Ok(j)
}

/// `msn_grafo` of the `.wasm`: the graph text, with the cleavages of a node when `strutture` asks.
fn graph_text(q: &Query, run: &mut dyn FnMut() -> Computed) -> Result<String> {
    let (g, j, o) = run()?;
    match q.some("strutture") {
        None => Ok(j),
        Some(nodo) => {
            let ppm = match q.some("ppm_struttura") {
                None => PPM_STRUTTURA,
                Some(v) => v
                    .parse::<f64>()
                    .ok()
                    .filter(|p| p.is_finite() && *p > 0.0)
                    .ok_or_else(|| {
                        Error::new("err.request.badNumber").with("name", "ppm_struttura")
                    })?,
            };
            strutture_testo(&g, &j, o.smiles.as_deref(), (nodo, ppm))
        }
    }
}

/// `msn_grafo_export` of the `.wasm`: the spectra of the graph as text, a table of it (`vista`) or the tree as a library (`formato=massbank`, `libreria=1`).
fn export(q: &Query, run: &mut dyn FnMut() -> Computed) -> Result<String> {
    let clean = q.get("pulito") == Some("1");
    if let Some(vista) = q.some("vista") {
        let (_, j, _) = run()?;
        return Ok(table_text(
            &mzlab_core::json::parse(j.as_bytes())?,
            vista,
            clean,
        ));
    }
    let formato = q.get("formato").unwrap_or("");
    if formato == "massbank" || q.get("libreria") == Some("1") {
        let (_, j, _) = run()?;
        let v = mzlab_core::json::parse(j.as_bytes())?;
        let get = |k: &str, d: &'static str| q.some(k).unwrap_or(d);
        let formato = if formato == "massbank" {
            "massbank"
        } else {
            "msp"
        };
        return library_text(
            &v,
            &j,
            (
                get("id", ""),
                formato,
                get("livelli", "all"),
                clean,
                get("annota", "1") != "0",
            ),
        );
    }
    let (f, pulito, nodo) = da_richiesta_export(&|k| q.get(k).map(String::from))?;
    let (g, _, _) = run()?;
    Ok(esporta(&g, f, pulito, nodo.as_deref()))
}
