//! Per-scan cells of one file for the maps of the session grid: for each scan the maximum intensity in every cell of 1 Da whose edges are whole numbers
//! plus a `phase` (0 for high resolution, 0.8 = `n - 0.2` for unit resolution, as `session_grid` lays them out), built in one pass over the peaks.
//! A whole-file map (maximum) and the run background are then rows of this table, not another pass over the peaks. The table does not depend on the
//! files of the session, only on the phase of its grid. The cells are exact: a peak that lies within rounding distance of a cell edge is not in the table
//! but in a short list that is put in the cells with the very arithmetic of the grid asked, so the map is the one the peaks give, bit by bit.

use std::sync::Arc;

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::mapgrid::Grid;
use crate::routes::Routes;

/// Tables over this many bytes are not built (the map is summed from the peaks).
const MAX_BYTES: usize = 64 << 20;
/// The engine's own ceiling for a table (a quarter of its cache budget); a table over it is not built, so a tiny budget never grows the memory.
static TABLE_CAP: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);

/// Sets the largest table the engine may build (bytes); the wasm engine ties it to its cache budget.
pub fn set_table_cap(bytes: usize) {
    TABLE_CAP.store(bytes, std::sync::atomic::Ordering::Relaxed);
}
/// The phases a session grid has: high resolution cells start at whole numbers, unit resolution ones at `n - 0.2`.
const PHASES: [f64; 2] = [0.0, 0.8];

/// A peak too close to a cell edge for the table: its place in the index, scan, m/z and intensity.
pub struct Edge {
    pub(crate) at: u64,
    pub(crate) pos: usize,
    pub(crate) mz: f64,
    pub(crate) y: f64,
}

/// The tables of one file, by level and phase (code, tick of last use), and the switch of the parity tests (`off` = every map is summed from the peaks).
#[derive(Default)]
pub struct CellStore {
    pub(crate) tables: Vec<(u8, u8, Arc<ScanCells>, u64)>,
    pub(crate) off: bool,
}

pub struct ScanCells {
    pub(crate) phase: f64,
    /// Number of the first cell: cell `k` is `[k + phase, k + phase + 1)`.
    pub(crate) k0: i64,
    pub(crate) ncell: usize,
    pub(crate) n_scans: usize,
    pub(crate) cells: Vec<f32>,
    pub(crate) edge: Vec<Edge>,
}

impl ScanCells {
    pub fn bytes(&self) -> usize {
        self.cells.len() * 4 + self.edge.len() * std::mem::size_of::<Edge>()
    }

    /// The maximum of scan `p` in cell `k` (0 outside the table).
    fn at(&self, p: usize, k: i64) -> f32 {
        let c = k - self.k0;
        if c < 0 || c as usize >= self.ncell {
            0.0
        } else {
            self.cells[p * self.ncell + c as usize]
        }
    }

    /// Per cell `row_of(p) * nmz + j` the maximum over the scans `p` that `row_of` gives a row, of the table cells `first + j * w ..first + (j + 1) * w`.
    pub(crate) fn fold(
        &self,
        rows: usize,
        (nmz, w): (usize, i64),
        first: i64,
        row_of: impl Fn(usize) -> Option<usize>,
    ) -> Vec<f64> {
        let mut acc = vec![0.0f64; rows * nmz];
        for p in (0..self.n_scans).filter_map(|p| row_of(p).map(|r| (p, r))) {
            let (p, row) = p;
            let out = &mut acc[row * nmz..][..nmz];
            for (j, o) in out.iter_mut().enumerate() {
                let c0 = first + j as i64 * w;
                for k in c0..c0 + w {
                    let v = self.at(p, k) as f64;
                    if v > *o {
                        *o = v;
                    }
                }
            }
        }
        acc
    }
}

/// The phase of a grid whose cells are `w` Da wide from `mz0`: `Some(index in PHASES)` when `w` is a whole number of Da and the edges are whole numbers
/// or `n - 0.2` (the grid of the session, or the region of a zoomed-out view whose cells are whole nominal masses); returns the width as a whole number.
pub(crate) fn phase_of(mz0: f64, w: f64) -> Option<(usize, i64)> {
    if !(w >= 1.0 - 1e-12 && (w - w.round()).abs() < 1e-9 && w < 1e4) {
        return None;
    }
    let f = mz0 - mz0.floor();
    PHASES
        .iter()
        .position(|p| (f - p).abs() < 1e-6)
        .map(|c| (c, w.round() as i64))
}

/// Peaks closer to an edge than this are kept apart: more than the float32 rounding of `(m/z as f32 - mz0 as f32)`; `f` = distance above the cell's lower edge.
fn near_edge(f: f64, mz: f64) -> bool {
    let band = 4e-7 * mz.abs() + 1e-6;
    f < band || 1.0 - f < band
}

impl<R: ReadAt> Routes<R> {
    /// For the parity tests: false = every map is summed from the peaks, as before the cells.
    pub fn set_scan_cells(&mut self, on: bool) {
        self.cells.off = !on;
        self.cells.tables.clear();
    }

    /// Number of per-scan tables held.
    pub fn cell_tables(&self) -> usize {
        self.cells.tables.len()
    }
}

/// Bytes of the per-scan cells of all the files; they go in the same LRU as the maps.
pub fn cells_cache_bytes<R: ReadAt>(files: &std::collections::HashMap<u32, Routes<R>>) -> usize {
    files
        .values()
        .flat_map(|r| r.cells.tables.iter())
        .map(|c| c.2.bytes())
        .sum()
}

impl<R: ReadAt> Routes<R> {
    /// The table of a level and phase, built on first use; None when there is none to build (no peaks, too big, an odd m/z range).
    pub(crate) fn scan_cells(&mut self, level: u8, code: usize) -> Result<Option<Arc<ScanCells>>> {
        if let Some(c) = self
            .cells
            .tables
            .iter_mut()
            .find(|c| c.0 == level && c.1 as usize == code)
        {
            c.3 = crate::routes::next_tick();
            return Ok(Some(c.2.clone()));
        }
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok(None);
        };
        let (n_scans, n_peaks) = (self.idx.sections[si].n_scans, self.idx.sections[si].n_peaks);
        if n_scans == 0 || n_peaks == 0 {
            return Ok(None);
        }
        let Some((lo, hi)) = self.idx.mz_range(si)? else {
            return Ok(None);
        };
        let phase = PHASES[code];
        let (k0, k1) = ((lo - phase).floor(), (hi - phase).floor());
        if !(k0.is_finite() && k1.is_finite() && k1 >= k0 && k1 - k0 < 1e7) {
            return Ok(None);
        }
        let (k0, ncell) = (k0 as i64, (k1 - k0) as usize + 1);
        if n_scans.saturating_mul(ncell).saturating_mul(4)
            > MAX_BYTES.min(TABLE_CAP.load(std::sync::atomic::Ordering::Relaxed))
        {
            return Ok(None);
        }
        let mut cells = vec![0.0f32; n_scans * ncell];
        let mut edge = Vec::new();
        let mut at = 0u64;
        self.idx.each_peak(si, 0, n_peaks, |p, mz, y| {
            let x = mz - phase;
            let kf = x.floor();
            let k = kf as i64;
            if !x.is_finite() || near_edge(x - kf, mz) || k < k0 || (k - k0) as usize >= ncell {
                edge.push(Edge { at, pos: p, mz, y });
            } else {
                let v = &mut cells[p * ncell + (k - k0) as usize];
                if y as f32 > *v {
                    *v = y as f32;
                }
            }
            at += 1;
        })?;
        let sc = Arc::new(ScanCells {
            phase,
            k0,
            ncell,
            n_scans,
            cells,
            edge,
        });
        self.cells
            .tables
            .push((level, code as u8, sc.clone(), crate::routes::next_tick()));
        Ok(Some(sc))
    }

    /// Builds the table the whole-file map of grid `g` reads, ahead of the first request (the page asks for it while the file is idle).
    pub fn warm_scan_cells(&mut self, level: u8, g: &Grid) -> Result<bool> {
        let Some((code, _)) = phase_of(g.mz0, g.dmz).filter(|_| !self.cells.off) else {
            return Ok(false);
        };
        Ok(self.scan_cells(level, code)?.is_some())
    }
}
