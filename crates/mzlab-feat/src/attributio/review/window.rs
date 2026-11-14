//! The MS1 ions of the isolation window of the MS2 scans of a row, judged as the review of Attributio judges them (`raw::ms1_metrics`). Called by
//! the chimeric-spectrum step of the CLI (`flow_chimeric`) on the scans that `interp::ms2link` picks; it lives here because it needs the review settings.

use crate::interp::ms2link::Ms2Scan;
use crate::sortc::SortC;
use crate::sweep::windows;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use super::raw::{ms1_metrics, Ms1Metrics, ScanInfo};
use super::settings::ReviewSettings;

/// The ions of the isolation window of each of `scans` (all of file `ix`) in the MS1 scan before it, or after it when none lies before, judged by
/// `ms1_metrics` with a co-isolation threshold of `chim_rel`. `None` where the polarity has no MS1 or the scan has no precursor peak.
pub fn ms1_window_metrics<R: ReadAt>(
    ix: &mut Index<R>,
    polarity: i8,
    scans: &[Ms2Scan],
    chim_rel: f64,
) -> Result<Vec<Option<Ms1Metrics>>> {
    let mut out = vec![None; scans.len()];
    let Some(si) = ix.section_of(1, Some(polarity)) else {
        return Ok(out);
    };
    let rt = ix.arrays(si)?.rt.clone();
    let mut asks: Vec<(f64, f64, usize, u32)> = Vec::new();
    for (i, s) in scans.iter().enumerate() {
        let after = rt.partition_point(|&t| t <= s.rt);
        let pos = if after > 0 { after - 1 } else { after };
        if pos < rt.len() {
            asks.push((s.iso.0, s.iso.1, i, pos as u32));
        }
    }
    asks.sortc_by(|a, b| a.0.total_cmp(&b.0));
    let ranges: Vec<(f64, f64)> = asks.iter().map(|a| (a.0, a.1)).collect();
    let settings = ReviewSettings {
        chim_rel,
        ..ReviewSettings::default()
    };
    windows(ix, si, &ranges, 1 << 20, |k, a, b, slab| {
        let (lo, hi, i, pos) = asks[k];
        let (mut mz, mut it) = (vec![], vec![]);
        for j in (a..b).filter(|&j| slab.pos[j] == pos) {
            mz.push(slab.mz[j]);
            it.push(slab.it[j]);
        }
        let prec = scans[i].precursor;
        let info = ScanInfo {
            low: lo,
            high: hi,
            target: prec,
            lo: prec - lo,
            hi: hi - prec,
            ms1_mz: mz,
            ms1_it: it,
        };
        out[i] = ms1_metrics(Some(&info), prec, None, &settings);
        Ok(())
    })?;
    Ok(out)
}
