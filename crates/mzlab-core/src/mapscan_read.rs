//! Reads of the whole-file maps, the run background and the aligned regions from the per-scan cells of `mapscan` (the cells plus the edge peaks put back with the grid's own arithmetic).

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::mapcells::{cell_of, clipped_cell, frac};
use crate::mapgrid::Grid;
use crate::mapscan::phase_of;
use crate::routes::Routes;

impl<R: ReadAt> Routes<R> {
    /// The cells (as `accumulate` gives them, maximum) of the whole-file map of the session grid `g` from the table; None when the grid or the file
    /// has no table. `bins` = RT row of each scan, `at` = the column of a m/z as the grid works it out.
    pub(crate) fn session_from_cells(
        &mut self,
        level: u8,
        g: &Grid,
        bins: &[usize],
        at: &dyn Fn(f64) -> f64,
    ) -> Result<Option<Vec<f64>>> {
        let Some((code, w)) = phase_of(g.mz0, g.dmz).filter(|_| !self.cells.off) else {
            return Ok(None);
        };
        let Some(sc) = self.scan_cells(level, code)? else {
            return Ok(None);
        };
        let first = (g.mz0 - sc.phase).round() as i64;
        let mut acc = sc.fold(g.nrt, (g.nmz, w), first, |p| Some(bins[p]));
        for e in &sc.edge {
            if let Some(c) = cell_of(bins[e.pos], at(e.mz), g.nmz) {
                if (e.y as f32) as f64 > acc[c] {
                    acc[c] = (e.y as f32) as f64;
                }
            }
        }
        Ok(Some(acc))
    }

    /// The cells of `run_background` (maximum, groups of `grp` scans, `nmz` cells from `mz0` to `mz1`) from the table, for the peaks `a..b` of the window.
    pub(crate) fn background_from_cells(
        &mut self,
        level: u8,
        (mz0, mz1): (f64, f64),
        nmz: usize,
        (a, b): (u64, u64),
        grp: usize,
    ) -> Result<Option<Vec<f64>>> {
        let Some((code, w)) = phase_of(mz0, (mz1 - mz0) / nmz as f64).filter(|_| !self.cells.off)
        else {
            return Ok(None);
        };
        let Some(sc) = self.scan_cells(level, code)? else {
            return Ok(None);
        };
        let first = (mz0 - sc.phase).round() as i64;
        let mut acc = sc.fold(sc.n_scans.div_ceil(grp), (nmz, w), first, |p| Some(p / grp));
        for e in sc.edge.iter().filter(|e| e.at >= a && e.at < b) {
            let c = clipped_cell(e.pos / grp, frac(e.mz, mz0, mz1, false) * nmz as f64, nmz);
            if (e.y as f32) as f64 > acc[c] {
                acc[c] = (e.y as f32) as f64;
            }
        }
        Ok(Some(acc))
    }

    /// The cells of `region_map` (maximum, linear m/z) from the table when the cells are whole nominal masses; `bins` = RT row of each scan, `inside` = the scans in the RT range.
    pub(crate) fn region_from_cells(
        &mut self,
        level: u8,
        (mz0, mz1): (f64, f64),
        (nrt, nmz): (usize, usize),
        (a, b): (u64, u64),
        (bins, inside): (&[usize], &[bool]),
    ) -> Result<Option<Vec<f64>>> {
        let Some((code, w)) = phase_of(mz0, (mz1 - mz0) / nmz as f64).filter(|_| !self.cells.off)
        else {
            return Ok(None);
        };
        let Some(sc) = self.scan_cells(level, code)? else {
            return Ok(None);
        };
        let first = (mz0 - sc.phase).round() as i64;
        let mut acc = sc.fold(nrt, (nmz, w), first, |p| inside[p].then_some(bins[p]));
        for e in sc
            .edge
            .iter()
            .filter(|e| e.at >= a && e.at < b && inside[e.pos])
        {
            let c = clipped_cell(bins[e.pos], frac(e.mz, mz0, mz1, false) * nmz as f64, nmz);
            if (e.y as f32) as f64 > acc[c] {
                acc[c] = (e.y as f32) as f64;
            }
        }
        Ok(Some(acc))
    }
}
