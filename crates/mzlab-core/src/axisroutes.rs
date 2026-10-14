//! `/api/scan_axis` (RT and index of every scan of a level) and `/api/ms2tr` (precursor > fragment pairs of the MS2 scans) from the
//! index: `Item.scan_axis` and `Item.ms2_transitions` of `mzlab/explore.py`.

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::json::Out;
use crate::routes::{Query, Routes};
use std::collections::BTreeMap;

/// How many fragments each precursor keeps (`top` of `ms2_transitions`).
const TOP: usize = 5;

impl<R: ReadAt> Routes<R> {
    /// `/api/scan_axis`: `{"rt":[...],"sid":[...]}` of the scans of the level, in the order of the peak table's scans.
    pub(crate) fn scan_axis(&mut self, q: &Query) -> Result<Option<String>> {
        let Some(si) = self.idx.section_of(q.level(), None) else {
            return Ok(Some("{\"rt\":[],\"sid\":[]}".into()));
        };
        let ar = self.idx.arrays(si)?;
        let mut o = Out::default();
        o.raw("{\"rt\":").floats(&ar.rt).raw(",\"sid\":[");
        for (i, &s) in ar.scan_ids.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).int(s);
        }
        o.raw("]}");
        Ok(Some(o.0))
    }

    /// The transitions of the MS2 scans: `[{"q1":..,"q3":..,"y":..}]` (the caller wraps them per file, as for `mrm`).
    pub fn ms2_transitions(&mut self) -> Result<String> {
        let Some(si) = self.idx.section_of(2, self.pol) else {
            return Ok("[]".into());
        };
        let ids = self.idx.arrays(si)?.scan_ids.clone();
        if ids.is_empty() {
            return Ok("[]".into());
        }
        // precursor of each scan in tenths of a Da (`np.round(p, 1)`): <= 0 = none
        let tenths: Vec<i64> = ids
            .iter()
            .map(|&i| (self.idx.scans[i as usize].prec(-1.0) * 10.0).round_ties_even() as i64)
            .collect();
        let n = self.idx.sections[si].n_peaks;
        let mut tot: BTreeMap<(i64, i64), f64> = BTreeMap::new();
        self.idx.each_peak(si, 0, n, |pos, mz, y| {
            let k1 = tenths[pos];
            if k1 > 0 {
                *tot.entry((k1, mz.round_ties_even() as i64)).or_insert(0.0) += y;
            }
        })?;
        let mut o = Out::default();
        o.raw("[");
        let mut first = true;
        let keys: Vec<i64> = {
            let mut k: Vec<i64> = tot.keys().map(|k| k.0).collect();
            k.dedup();
            k
        };
        for k1 in keys {
            let mut g: Vec<(i64, f64)> = tot.range((k1, i64::MIN)..=(k1, i64::MAX)).map(|(k, &v)| (k.1, v)).collect();
            g.sort_by(|a, b| b.1.total_cmp(&a.1)); // stable: ties keep the ascending q3
            g.truncate(TOP);
            g.sort_by_key(|x| x.0);
            for (q3, y) in g {
                o.raw(if first { "{\"q1\":" } else { ",{\"q1\":" });
                first = false;
                o.num(k1 as f64 / 10.0).raw(",\"q3\":").num(q3 as f64).raw(",\"y\":").num(crate::json::round(y, 1)).raw("}");
            }
        }
        o.raw("]");
        Ok(o.0)
    }
}
