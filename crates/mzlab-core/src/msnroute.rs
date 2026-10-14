//! `/api/msn_albero` (the tree of the precursors from the index), the filter `msn:<id>` of the other routes and `/api/msntree` of a file with an index: the scan table gives the fragmentation path of every scan and the spectra come from the mzML by offset
//! (`sorgente::Scansioni::spettro`), so the tree needs no Python. `Ok(None)` = Python's (a `ppm` Python must reject); the errors of the tree are keyed.

use std::sync::Arc;

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::json::{round, Out};
use crate::msnalbero::{albero, blocchi, Albero, Raggruppa, Tol};
use crate::msntree::{msn_tree, TreeScan};
use crate::routes::{Query, Routes};
use crate::sorgente::Scansioni;

impl<R: ReadAt> Routes<R> {
    /// The tree of the fragmentation paths of the file (`App.msn_tree`: the whole run, whatever part of a mixed file `k` is).
    pub(crate) fn msntree_route<M: Scansioni + ?Sized>(&mut self, q: &Query, mzml: &mut M) -> Result<Option<String>> {
        let Ok(ppm) = q.num("ppm") else { return Ok(None) };
        let scans: Vec<TreeScan> = self
            .idx
            .scans
            .iter()
            .enumerate()
            .map(|(index, s)| TreeScan { index, level: s.level, polarity: s.polarity, path: s.path.clone().unwrap_or_default() })
            .collect();
        let table = &self.idx.scans;
        msn_tree(&scans, |i| mzml.spettro(&table[i]), q.some("formula"), ppm.unwrap_or(5.0)).map(Some)
    }

    /// Runs `f` with the options of `q` (`Opz`), then puts back the ones before: what a request asks never leaks into the next.
    pub(crate) fn con_opz<T>(&mut self, q: &Query, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let opz = Opz {
            raggruppa: raggruppa_of(q, "mtol", "mce")?,
            somma: q.get("modo") == Some("somma"),
        };
        let before = std::mem::replace(&mut self.msn.opz, opz);
        let out = f(self);
        self.msn.opz = before;
        out
    }

    /// The tree of the file for a grouping, built once (memory: the scan indices of the nodes).
    pub(crate) fn albero_per(&mut self, r: &Raggruppa) -> Arc<Albero> {
        if let Some((_, a)) = self.msn.alberi.iter().find(|(k, _)| k == r) {
            return a.clone();
        }
        let a = Arc::new(albero(&self.idx.scans, r));
        if self.msn.alberi.len() >= ALBERI {
            self.msn.alberi.remove(0);
        }
        self.msn.alberi.push((r.clone(), a.clone()));
        a
    }

    /// The filter of a request; `msn:<id>` is a node of the tree of the grouping of the request (`err.msn.node` when there is none).
    pub(crate) fn filtro<'a>(&mut self, f: Option<&'a str>) -> Result<Option<Filtro<'a>>> {
        let Some(f) = f else { return Ok(None) };
        let Some(id) = f.strip_prefix("msn:") else {
            return Ok(Some(Filtro::Testo(f)));
        };
        let r = self.msn.opz.raggruppa.clone();
        let a = self.albero_per(&r);
        let Some(n) = a.nodi.iter().find(|n| n.id == id) else {
            return Err(Error::new("err.msn.node").with("id", id));
        };
        let mut mask = vec![false; self.idx.scans.len()];
        n.scansioni.iter().for_each(|&i| mask[i] = true);
        Ok(Some(Filtro::Nodo(Arc::new(mask))))
    }

    /// `/api/msn_albero?k&tol=nom|<Da>|<n>ppm&ce=1|0`: the tree of the precursors of the file from its index alone (no spectrum), `msn_albero/1`.
    pub(crate) fn msn_albero_route(&mut self, q: &Query) -> Result<Option<String>> {
        let r = raggruppa_of(q, "tol", "ce")?;
        let a = self.albero_per(&r);
        if a.nodi.len() > MAX_NODI {
            return Err(Error::new("err.tree.tooMany").with("n", a.nodi.len()));
        }
        let scans = &self.idx.scans;
        let span = |ix: &mut dyn Iterator<Item = usize>| {
            ix.map(|i| scans[i].rt).fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), t| (a.min(t), b.max(t)))
        };
        let mut o = Out::default();
        o.raw("{\"formato\":\"msn_albero/1\",\"ms1\":{\"n\":").int(a.ms1.len() as i64);
        let (t0, t1) = span(&mut a.ms1.iter().copied());
        o.raw(",\"rt0\":").opt(t0.is_finite().then(|| round(t0, 4))).raw(",\"rt1\":").opt(t1.is_finite().then(|| round(t1, 4)));
        let mut livelli: Vec<u8> = a.nodi.iter().map(|n| n.livello).collect();
        crate::sortc::slice_ord(&mut livelli);
        livelli.dedup();
        o.raw("},\"livelli\":[").raw(&livelli.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(",")).raw("],\"nodi\":[");
        for (k, (n, bl)) in a.nodi.iter().zip(blocchi(&a, scans.len())).enumerate() {
            let (m, act, ce) = n.chiave.last().cloned().unwrap_or((0.0, String::new(), f64::NAN));
            o.raw(if k > 0 { "," } else { "" }).raw("{\"id\":").str(&n.id).raw(",\"genitore\":");
            match n.genitore {
                Some(g) => o.str(&a.nodi[g].id),
                None => o.raw("null"),
            };
            o.raw(",\"salto\":").int(n.salto as i64).raw(",\"livello\":").int(n.livello as i64);
            o.raw(",\"prec\":").num(m).raw(",\"att\":").str(&act).raw(",\"ce\":").opt((!ce.is_nan()).then_some(ce));
            let (t0, t1) = span(&mut n.scansioni.iter().copied());
            o.raw(",\"n\":").int(n.scansioni.len() as i64).raw(",\"rt0\":").num(round(t0, 4)).raw(",\"rt1\":").num(round(t1, 4));
            o.raw(",\"blocchi\":[");
            for (j, (b0, b1)) in bl.iter().enumerate() {
                o.raw(if j > 0 { ",[" } else { "[" }).num(round(scans[*b0].rt, 4)).raw(",").num(round(scans[*b1].rt, 4)).raw("]");
            }
            o.raw("]}");
        }
        o.raw("],\"scartate\":").int(a.scartate as i64).raw("}");
        Ok(Some(o.0))
    }
}

/// What a request asks of the helpers deep in the routes: the grouping that `filt=msn:<id>` refers to (`mtol=nom|<Da>|<n>ppm`, `mce=1|0`)
/// and `modo=somma` (the spectra of a window summed per cell instead of averaged). Set for the time of one request (`Routes::con_opz`).
#[derive(Debug, Clone, Default)]
pub(crate) struct Opz {
    pub raggruppa: Raggruppa,
    pub somma: bool,
}

/// What `filt` / `filter` keeps: a filter string or scan key, or the scans of a node of the MSn tree (`msn:<id>`, a mask over the scans).
pub(crate) enum Filtro<'a> {
    Testo(&'a str),
    Nodo(Arc<Vec<bool>>),
}

/// The MSn state of a file: the trees already built, by grouping (the filter `msn:<id>` reads its scans from here), and the options of the request.
#[derive(Default)]
pub(crate) struct Msn {
    pub alberi: Vec<(Raggruppa, Arc<Albero>)>,
    pub opz: Opz,
}

/// `nom` (or absent), `0.01` (Da) or `10ppm`.
fn tol_of(v: Option<&str>) -> Result<Tol> {
    let Some(t) = v.filter(|t| !t.is_empty() && *t != "nom") else {
        return Ok(Tol::Nominale);
    };
    let (num, ppm) = match t.strip_suffix("ppm") {
        Some(n) => (n, true),
        None => (t, false),
    };
    match num.trim().parse::<f64>() {
        Ok(x) if x > 0.0 && x.is_finite() && ppm => Ok(Tol::Ppm(x)),
        Ok(x) if x > 0.0 && x.is_finite() => Ok(Tol::Da(x)),
        _ => Err(Error::new("err.msn.tol").with("tol", t)),
    }
}

fn raggruppa_of(q: &Query, tol: &str, ce: &str) -> Result<Raggruppa> {
    Ok(Raggruppa {
        tol: tol_of(q.get(tol))?,
        energie: q.get(ce) != Some("0"),
    })
}

/// The trees already built, at most this many groupings per file.
const ALBERI: usize = 4;


/// More nodes than this: `err.tree.tooMany` (energies merged or a wider tolerance make fewer).
const MAX_NODI: usize = 400;

#[cfg(test)]
mod tests;
