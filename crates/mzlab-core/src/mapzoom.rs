//! Per-scan columns of one region window: for each scan the maximum intensity in every m/z cell of a window `(mz0, mz1, nmz, log)`, built on the first
//! big region map of that window. A region that only changes its RT stretch (zoom in RT, pan in RT) is then folded from this table instead of read
//! from all the peaks of the band again. The cells use the arithmetic of `region_map`, so the map is the one the peaks give, bit by bit.

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::mapcells::{clipped_cell, frac};
use crate::routes::Routes;

/// Windows with fewer peaks than this are read straight from the index (a table would cost more than it saves).
const MIN_PEAKS: u64 = 1 << 21;
/// Largest table kept (bytes); the engine's cache budget may lower it (`mapscan::table_cap`).
const MAX_BYTES: usize = 32 << 20;

/// The window a table belongs to: level, bits of `mz0` and `mz1`, number of cells, logarithmic axis.
pub(crate) type Key = (u8, u64, u64, usize, bool);

/// The columns of one window: `cells[j * n_scans + p]` = maximum of scan `p` in cell `j` (cell-major: the peaks come in m/z order, so a cell's scans are written close together); `tick` = last use.
pub struct ZoomCols {
    key: Key,
    n_scans: usize,
    cells: Vec<f32>,
    pub(crate) tick: u64,
}

impl ZoomCols {
    pub fn bytes(&self) -> usize {
        self.cells.len() * 4
    }
}

impl<R: ReadAt> Routes<R> {
    /// The cells of `region_map` (maximum) for the scans `inside`, in rows `bins`, from the table of the window; builds the table when the window holds
    /// many peaks. None when the window is small or the table would not fit (the caller reads the peaks).
    pub(crate) fn region_from_columns(
        &mut self,
        level: u8,
        (mz0, mz1): (f64, f64),
        (nrt, nmz, log): (usize, usize, bool),
        (a, b): (u64, u64),
        (bins, inside): (&[usize], &[bool]),
    ) -> Result<Option<Vec<f64>>> {
        if self.cells.off || nmz == 0 {
            return Ok(None);
        }
        let key: Key = (level, mz0.to_bits(), mz1.to_bits(), nmz, log);
        if !self.cells.zoom.as_ref().is_some_and(|z| z.key == key) {
            let cap = MAX_BYTES.min(crate::mapscan::table_cap());
            if b.saturating_sub(a) < MIN_PEAKS
                || bins.len().saturating_mul(nmz).saturating_mul(4) > cap
            {
                return Ok(None);
            }
            // the table is built when the window comes a second time: a window asked once is read straight from the peaks
            if self.cells.zoom_seen != Some(key) {
                self.cells.zoom_seen = Some(key);
                return Ok(None);
            }
            let Some(si) = self.idx.section_of(level, self.pol) else {
                return Ok(None);
            };
            let n_scans = bins.len();
            let mut cells = vec![0.0f32; n_scans * nmz];
            self.idx.each_peak(si, a, b, |p, mz, y| {
                let c = clipped_cell(0, frac(mz, mz0, mz1, log) * nmz as f64, nmz) * n_scans + p;
                if y as f32 > cells[c] {
                    cells[c] = y as f32;
                }
            })?;
            self.cells.zoom = Some(ZoomCols {
                key,
                n_scans,
                cells,
                tick: 0,
            });
        }
        let Some(z) = self.cells.zoom.as_mut() else {
            return Ok(None);
        };
        z.tick = crate::routes::next_tick();
        let mut acc = vec![0.0f64; nrt * nmz];
        for (j, col) in z.cells.chunks_exact(z.n_scans).enumerate() {
            for (p, v) in col.iter().enumerate() {
                if inside[p] {
                    let o = &mut acc[bins[p] * nmz + j];
                    if *v as f64 > *o {
                        *o = *v as f64;
                    }
                }
            }
        }
        Ok(Some(acc))
    }
}
