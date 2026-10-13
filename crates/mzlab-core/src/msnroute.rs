//! `/api/msntree` of a file with an index: the scan table gives the fragmentation path of every scan and the spectra come from the mzML by offset
//! (`sorgente::Scansioni::spettro`), so the tree needs no Python. `Ok(None)` = Python's (a `ppm` Python must reject); the errors of the tree are keyed.

use crate::error::Result;
use crate::idxread::ReadAt;
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
}
