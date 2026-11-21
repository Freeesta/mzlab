//! The routes of the channels: `api/channels` and the `channel=<id>` parameter of `chrom`, `xic`, `scanlist`, `spectrum` and `nearest_scan`.
//! `channel=ms:<key>` is `filt=<key>` of the same level; the old parameters (`level`, `prec`, `filt`) answer as before.

use std::sync::Arc;

use super::{Channels, Famiglia};
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::iteminfo::Facts;
use crate::msnroute::Filter;
use crate::mzmlhead::Chrom;
use crate::routes::{trace, Query, Routes};

/// The routes that take `channel=`.
const WITH_CHANNEL: [&str; 5] = [
    "/api/chrom",
    "/api/xic",
    "/api/scanlist",
    "/api/spectrum",
    "/api/nearest_scan",
];

impl<R: ReadAt> Routes<R> {
    /// The headers of the chromatograms of the mzML (SRM, SIC) and its facts; the channels are built again at the next request.
    pub fn pon_cromatogrammi(&mut self, name: &str, chroms: &[Chrom], facts: &Facts) {
        self.msn.channels = State {
            nome: name.into(),
            chroms: chroms.to_vec(),
            facts: facts.clone(),
            cache: None,
        };
    }

    /// The channels of the file, built on first use.
    pub fn canali(&mut self) -> Arc<Channels> {
        if let Some(c) = &self.msn.channels.cache {
            return c.clone();
        }
        let s = &self.msn.channels;
        let c = Arc::new(Channels::da(&self.idx.scans, &s.chroms, &s.facts, &s.nome));
        self.msn.channels.cache = Some(c.clone());
        c
    }

    /// `filt=channel:<id>`: the scans of the channel as a mask over the scan table.
    pub(crate) fn channel_filter(&mut self, id: &str) -> Result<Filter<'static>> {
        let c = self.canali();
        let i = c
            .find(id)
            .ok_or_else(|| Error::new("err.canali.unknown").with("id", id))?;
        let mut mask = vec![false; self.idx.scans.len()];
        c.scansioni(i)
            .into_iter()
            .for_each(|k| mask[k as usize] = true);
        Ok(Filter::Node(Arc::new(mask)))
    }

    /// The request `path` with `channel=<id>`: the same request with the level and the filter of the channel; `None` for a channel with no scans here
    /// (a transition, a UV trace): the readers of the chromatograms answer it.
    pub(crate) fn with_channel(
        &mut self,
        path: &str,
        q: &Query,
        id: &str,
    ) -> Result<Option<String>> {
        let channels = self.canali();
        let i = channels
            .find(id)
            .ok_or_else(|| Error::new("err.canali.unknown").with("id", id))?;
        let c = &channels.v[i];
        let mut r: Vec<(String, String)> =
            q.0.iter()
                .filter(|(k, _)| k != "channel")
                .cloned()
                .collect();
        let ha = |r: &[(String, String)], k: &str| r.iter().any(|(n, _)| n == k);
        match c.famiglia {
            Famiglia::Srm | Famiglia::Sim | Famiglia::Uv | Famiglia::Analog => return Ok(None),
            Famiglia::Tic | Famiglia::Bpc => {
                if path == "/api/chrom" && !ha(&r, "kind") {
                    r.push(("kind".into(), c.famiglia.name().into()));
                }
                if matches!(path, "/api/chrom" | "/api/xic") && !ha(&r, "level") {
                    return self.all_levels(path, &Query(r));
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
                    r.push((f.into(), format!("channel:{id}")));
                }
            }
        }
        self.handle(path, &Query(r))
    }

    /// The trace of `tic` / `bpc` over every level: the traces of the levels joined in time order.
    fn all_levels(&mut self, path: &str, q: &Query) -> Result<Option<String>> {
        let mut levels: Vec<u8> = self.idx.sections.iter().map(|s| s.level).collect();
        crate::sortc::slice_ord(&mut levels);
        levels.dedup();
        let mut points: Vec<(f64, f64)> = Vec::new();
        for l in levels {
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
            points.extend(rt.into_iter().zip(y));
        }
        crate::sortc::slice_by(&mut points, |a, b| a.0.total_cmp(&b.0));
        let (rt, y): (Vec<f64>, Vec<f64>) = points.into_iter().unzip();
        Ok(Some(trace(&rt, &y)))
    }

    /// `/api/xic?mz&tol&level[&filt][&smooth=3|5]`: the XIC inside the scans that pass `filt` (a scan key, `msn:<id>` or `channel:<id>`), then smoothed
    /// by `smooth` points (`peak1d::smooth`); with neither it is not this route's.
    pub(crate) fn xic_route(&mut self, q: &Query) -> Result<Option<String>> {
        if let Some(sp) = q.some("sweep") {
            return if q.some("filt").is_some() {
                Ok(None)
            } else {
                self.sweep_route(sp, q)
            };
        }
        let Some(mz) = q.num("mz")? else {
            return Ok(None);
        };
        let smooth = match q.some("smooth") {
            None => None,
            Some("3") => Some(3),
            Some("5") => Some(5),
            Some(_) => return Ok(None),
        };
        let filt = q.some("filt");
        if filt.is_none() && smooth.is_none() {
            return Ok(None);
        }
        let (tol, level) = (q.num("tol")?.unwrap_or(0.35), q.level());
        let (mut rt, mut y) = self.xic_raw(mz, tol, level)?;
        if let (Some(f), Some(si)) = (filt, self.idx.section_of(level, self.pol)) {
            let ids = self.idx.arrays(si)?.scan_ids.clone();
            let Some(fl) = self.filtro(Some(f))? else {
                return Ok(None);
            };
            let keep: Vec<bool> = ids
                .iter()
                .map(|&id| fl.passes(id as usize, &self.idx.scans[id as usize]))
                .collect();
            let pick = |v: &[f64]| -> Vec<f64> {
                v.iter()
                    .zip(&keep)
                    .filter(|(_, k)| **k)
                    .map(|(x, _)| *x)
                    .collect()
            };
            (rt, y) = (pick(&rt), pick(&y));
        }
        if let Some(k) = smooth {
            y = crate::peak1d::smooth(&y, k);
        }
        Ok(Some(trace(&rt, &y)))
    }

    /// `/api/channels` and the routes that take `channel=`; `None` = not a channel request.
    pub(crate) fn route_channels(
        &mut self,
        path: &str,
        q: &Query,
    ) -> Option<Result<Option<String>>> {
        if path == "/api/channels" {
            return Some(Ok(Some(self.canali().json())));
        }
        let id = q.some("channel").filter(|_| WITH_CHANNEL.contains(&path))?;
        Some(self.with_channel(path, q, id))
    }
}

/// What a file keeps for its channels: the chromatogram headers the page read from the mzML, and the channels built from them (once).
#[derive(Default)]
pub(crate) struct State {
    nome: String,
    chroms: Vec<Chrom>,
    facts: Facts,
    cache: Option<Arc<Channels>>,
}
