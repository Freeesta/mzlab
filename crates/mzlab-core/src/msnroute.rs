//! `/api/msn_tree` (the tree of the precursors from the index), the filter `msn:<id>` of the other routes and `/api/msntree` of a file with an index: the scan table gives the fragmentation path of every scan and the spectra come from the mzML by offset
//! (`source::Scansioni::spettro`), so the tree needs no Python. `Ok(None)` = Python's (a `ppm` Python must reject); the errors of the tree are keyed.

use std::sync::Arc;

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::json::{round, Out};
use crate::msntree_build::{tree, blocks, Tree, Grouping, Tol};
use crate::msntree::{msn_tree, TreeScan};
use crate::routes::{Query, Routes};
use crate::source::Scansioni;

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

    /// Runs `f` with the options of `q` (`Opts`), then puts back the ones before: what a request asks never leaks into the next.
    pub(crate) fn with_opts<T>(&mut self, q: &Query, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let opts = Opts {
            group: group_of(q, "mtol", "mce")?,
            sum: q.get("mode") == Some("sum"),
        };
        let before = std::mem::replace(&mut self.msn.opts, opts);
        let out = f(self);
        self.msn.opts = before;
        out
    }

    /// The tree of the file for a grouping, built once (memory: the scan indices of the nodes).
    pub(crate) fn tree_per(&mut self, r: &Grouping) -> Arc<Tree> {
        if let Some((_, a)) = self.msn.trees.iter().find(|(k, _)| k == r) {
            return a.clone();
        }
        let a = Arc::new(tree(&self.idx.scans, r));
        if self.msn.trees.len() >= TREES {
            self.msn.trees.remove(0);
        }
        self.msn.trees.push((r.clone(), a.clone()));
        a
    }

    /// The filter of a request; `msn:<id>` is a node of the tree of the grouping of the request (`err.msn.node` when there is none).
    pub(crate) fn filtro<'a>(&mut self, f: Option<&'a str>) -> Result<Option<Filter<'a>>> {
        let Some(f) = f else { return Ok(None) };
        if let Some(id) = f.strip_prefix("channel:") {
            return self.channel_filter(id).map(Some);
        }
        let Some(id) = f.strip_prefix("msn:") else {
            return Ok(Some(Filter::Testo(f)));
        };
        let r = self.msn.opts.group.clone();
        let a = self.tree_per(&r);
        let Some(n) = a.nodes.iter().find(|n| n.id == id) else {
            return Err(Error::new("err.msn.node").with("id", id));
        };
        let mut mask = vec![false; self.idx.scans.len()];
        n.scans.iter().for_each(|&i| mask[i] = true);
        Ok(Some(Filter::Node(Arc::new(mask))))
    }

    /// `/api/msn_tree?k&tol=nom|<Da>|<n>ppm&ce=1|0`: the tree of the precursors of the file from its index alone (no spectrum), `msn-tree/1`.
    pub(crate) fn msn_tree_route(&mut self, q: &Query) -> Result<Option<String>> {
        let r = group_of(q, "tol", "ce")?;
        let a = self.tree_per(&r);
        if a.nodes.len() > MAX_NODES {
            return Err(Error::new("err.tree.tooMany").with("n", a.nodes.len()));
        }
        let scans = &self.idx.scans;
        let span = |ix: &mut dyn Iterator<Item = usize>| {
            ix.map(|i| scans[i].rt).fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), t| (a.min(t), b.max(t)))
        };
        let mut o = Out::default();
        o.raw("{\"schema\":\"msn-tree/1\",\"ms1\":{\"n\":").int(a.ms1.len() as i64);
        let (t0, t1) = span(&mut a.ms1.iter().copied());
        o.raw(",\"rt0\":").opt(t0.is_finite().then(|| round(t0, 4))).raw(",\"rt1\":").opt(t1.is_finite().then(|| round(t1, 4)));
        // mean total ion current per scan of a group of scans
        let mean_tic = |ix: &[usize]| if ix.is_empty() { 0.0 } else { ix.iter().map(|&i| scans[i].tic).sum::<f64>() / ix.len() as f64 };
        o.raw(",\"tic\":").num(round(mean_tic(&a.ms1), 1));
        let mut levels: Vec<u8> = a.nodes.iter().map(|n| n.level).collect();
        crate::sortc::slice_ord(&mut levels);
        levels.dedup();
        o.raw("},\"levels\":[").raw(&levels.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(",")).raw("],\"nodes\":[");
        for (k, (n, bl)) in a.nodes.iter().zip(blocks(&a, scans.len())).enumerate() {
            let (m, act, ce) = n.key.last().cloned().unwrap_or((0.0, String::new(), f64::NAN));
            o.raw(if k > 0 { "," } else { "" }).raw("{\"id\":").str(&n.id).raw(",\"parent\":");
            match n.parent {
                Some(g) => o.str(&a.nodes[g].id),
                None => o.raw("null"),
            };
            o.raw(",\"skip\":").int(n.skip as i64).raw(",\"level\":").int(n.level as i64);
            o.raw(",\"precursor\":").num(m).raw(",\"activation\":").str(&act).raw(",\"ce\":").opt((!ce.is_nan()).then_some(ce));
            let (t0, t1) = span(&mut n.scans.iter().copied());
            o.raw(",\"n\":").int(n.scans.len() as i64).raw(",\"rt0\":").num(round(t0, 4)).raw(",\"rt1\":").num(round(t1, 4));
            o.raw(",\"tic\":").num(round(mean_tic(&n.scans), 1));
            o.raw(",\"blocks\":[");
            for (j, (b0, b1)) in bl.iter().enumerate() {
                o.raw(if j > 0 { ",[" } else { "[" }).num(round(scans[*b0].rt, 4)).raw(",").num(round(scans[*b1].rt, 4)).raw("]");
            }
            o.raw("]}");
        }
        o.raw("],\"discarded\":").int(a.discarded as i64).raw("}");
        Ok(Some(o.0))
    }
}

/// What a request asks of the helpers deep in the routes: the grouping that `filt=msn:<id>` refers to (`mtol=nom|<Da>|<n>ppm`, `mce=1|0`)
/// and `mode=sum` (the spectra of a window summed per cell instead of averaged). Set for the time of one request (`Routes::with_opts`).
#[derive(Debug, Clone, Default)]
pub(crate) struct Opts {
    pub group: Grouping,
    pub sum: bool,
}

/// What `filt` / `filter` keeps: a filter string or scan key, or the scans of a node of the MSn tree (`msn:<id>`, a mask over the scans).
pub(crate) enum Filter<'a> {
    Testo(&'a str),
    Node(Arc<Vec<bool>>),
}

/// The MSn state of a file: the trees already built, by grouping (the filter `msn:<id>` reads its scans from here), and the options of the request.
#[derive(Default)]
pub(crate) struct Msn {
    pub trees: Vec<(Grouping, Arc<Tree>)>,
    pub opts: Opts,
    /// The chromatogram headers the page gave and the channels built from them (`channels::routes`).
    pub channels: crate::channels::State,
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

fn group_of(q: &Query, tol: &str, ce: &str) -> Result<Grouping> {
    Ok(Grouping {
        tol: tol_of(q.get(tol))?,
        energies: q.get(ce) != Some("0"),
    })
}

/// The trees already built, at most this many groupings per file.
const TREES: usize = 4;


/// More nodes than this: `err.tree.tooMany` (energies merged or a wider tolerance make fewer).
const MAX_NODES: usize = 400;

#[cfg(test)]
mod tests;
