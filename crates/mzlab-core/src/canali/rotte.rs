//! The routes of the channels: `api/canali` and the `canale=<id>` parameter of `chrom`, `xic`, `scanlist`, `spectrum` and `nearest_scan`.
//! `canale=ms:<key>` is `filt=<key>` of the same level; the old parameters (`level`, `prec`, `filt`) answer as before.

use std::sync::Arc;

use super::{Canali, Famiglia};
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::iteminfo::Facts;
use crate::msnroute::Filtro;
use crate::mzmlhead::Chrom;
use crate::routes::{trace, Query, Routes};

/// The routes that take `canale=`.
const CON_CANALE: [&str; 5] = [
    "/api/chrom",
    "/api/xic",
    "/api/scanlist",
    "/api/spectrum",
    "/api/nearest_scan",
];

impl<R: ReadAt> Routes<R> {
    /// The headers of the chromatograms of the mzML (SRM, SIC) and its facts; the channels are built again at the next request.
    pub fn pon_cromatogrammi(&mut self, nome: &str, chroms: &[Chrom], facts: &Facts) {
        self.msn.canali = Stato {
            nome: nome.into(),
            chroms: chroms.to_vec(),
            facts: facts.clone(),
            cache: None,
        };
    }

    /// The channels of the file, built on first use.
    pub fn canali(&mut self) -> Arc<Canali> {
        if let Some(c) = &self.msn.canali.cache {
            return c.clone();
        }
        let s = &self.msn.canali;
        let c = Arc::new(Canali::da(&self.idx.scans, &s.chroms, &s.facts, &s.nome));
        self.msn.canali.cache = Some(c.clone());
        c
    }

    /// `filt=canale:<id>`: the scans of the channel as a mask over the scan table.
    pub(crate) fn filtro_canale(&mut self, id: &str) -> Result<Filtro<'static>> {
        let c = self.canali();
        let i = c
            .trova(id)
            .ok_or_else(|| Error::new("err.canali.unknown").with("id", id))?;
        let mut mask = vec![false; self.idx.scans.len()];
        c.scansioni(i)
            .into_iter()
            .for_each(|k| mask[k as usize] = true);
        Ok(Filtro::Nodo(Arc::new(mask)))
    }

    /// The request `path` with `canale=<id>`: the same request with the level and the filter of the channel; `None` for a channel with no scans here
    /// (a transition, a UV trace): the readers of the chromatograms answer it.
    pub(crate) fn con_canale(&mut self, path: &str, q: &Query, id: &str) -> Result<Option<String>> {
        let canali = self.canali();
        let i = canali
            .trova(id)
            .ok_or_else(|| Error::new("err.canali.unknown").with("id", id))?;
        let c = &canali.v[i];
        let mut r: Vec<(String, String)> =
            q.0.iter().filter(|(k, _)| k != "canale").cloned().collect();
        let ha = |r: &[(String, String)], k: &str| r.iter().any(|(n, _)| n == k);
        match c.famiglia {
            Famiglia::Srm | Famiglia::Sim | Famiglia::Uv | Famiglia::Analog => return Ok(None),
            Famiglia::Tic | Famiglia::Bpc => {
                if path == "/api/chrom" && !ha(&r, "kind") {
                    r.push(("kind".into(), c.famiglia.nome().into()));
                }
                if matches!(path, "/api/chrom" | "/api/xic") && !ha(&r, "level") {
                    return self.tutti_i_livelli(path, &Query(r));
                }
            }
            Famiglia::Ms | Famiglia::Dd => {
                if !ha(&r, "level") {
                    r.push(("level".into(), c.livello.to_string()));
                }
                let f = if path == "/api/nearest_scan" {
                    "filter"
                } else {
                    "filt"
                };
                if !ha(&r, f) {
                    r.push((f.into(), format!("canale:{id}")));
                }
            }
        }
        self.handle(path, &Query(r))
    }

    /// The trace of `tic` / `bpc` over every level: the traces of the levels joined in time order.
    fn tutti_i_livelli(&mut self, path: &str, q: &Query) -> Result<Option<String>> {
        let mut livelli: Vec<u8> = self.idx.sections.iter().map(|s| s.level).collect();
        crate::sortc::slice_ord(&mut livelli);
        livelli.dedup();
        let mut punti: Vec<(f64, f64)> = Vec::new();
        for l in livelli {
            let (rt, y) = if path == "/api/xic" {
                let Some(mz) = q.num("mz")? else {
                    return Ok(None);
                };
                self.xic_raw(mz, q.num("tol")?.unwrap_or(0.35), l)?
            } else {
                let mut r = q.0.clone();
                r.push(("level".into(), l.to_string()));
                match self.chrom_raw(&Query(r))? {
                    Some(t) => t,
                    None => return Ok(None),
                }
            };
            punti.extend(rt.into_iter().zip(y));
        }
        crate::sortc::slice_by(&mut punti, |a, b| a.0.total_cmp(&b.0));
        let (rt, y): (Vec<f64>, Vec<f64>) = punti.into_iter().unzip();
        Ok(Some(trace(&rt, &y)))
    }

    /// `/api/xic?mz&tol&level&filt`: the XIC inside the scans that pass `filt` (a scan key, `msn:<id>` or `canale:<id>`); without `filt` it is not this route's.
    pub(crate) fn xic_route(&mut self, q: &Query) -> Result<Option<String>> {
        let (Some(mz), Some(f)) = (q.num("mz")?, q.some("filt")) else {
            return Ok(None);
        };
        let (tol, level) = (q.num("tol")?.unwrap_or(0.35), q.level());
        let (rt, y) = self.xic_raw(mz, tol, level)?;
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok(Some(trace(&rt, &y)));
        };
        let ids = self.idx.arrays(si)?.scan_ids.clone();
        let Some(fl) = self.filtro(Some(f))? else {
            return Ok(None);
        };
        let keep: Vec<bool> = ids
            .iter()
            .map(|&id| fl.passa(id as usize, &self.idx.scans[id as usize]))
            .collect();
        let pick = |v: &[f64]| -> Vec<f64> {
            v.iter()
                .zip(&keep)
                .filter(|(_, k)| **k)
                .map(|(x, _)| *x)
                .collect()
        };
        Ok(Some(trace(&pick(&rt), &pick(&y))))
    }

    /// `/api/canali` and the routes that take `canale=`; `None` = not a channel request.
    pub(crate) fn canali_route(&mut self, path: &str, q: &Query) -> Option<Result<Option<String>>> {
        if path == "/api/canali" {
            return Some(Ok(Some(self.canali().json())));
        }
        let id = q.some("canale").filter(|_| CON_CANALE.contains(&path))?;
        Some(self.con_canale(path, q, id))
    }
}

/// What a file keeps for its channels: the chromatogram headers the page read from the mzML, and the channels built from them (once).
#[derive(Default)]
pub(crate) struct Stato {
    nome: String,
    chroms: Vec<Chrom>,
    facts: Facts,
    cache: Option<Arc<Canali>>,
}
