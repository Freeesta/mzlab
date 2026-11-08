//! `features.align` with the algorithm `join` and `features.gapfill` with `peak_finder`: the join aligner and the peak finder of mzmine 3.9.0
//! (`join.rs`, `gapfill.rs`) on the tables of the files that `features.detect` wrote, with the common parameters of the steps and
//! `align_mzmine::{JoinAlign, PeakFinder}`. The join gives the groups; each row is then written by `align::push_group`, so the table has the columns
//! of every per-ion table of mzLab (row m/z and RT are area-weighted means, as there). Called by `series::align_rows` and `series::finish`.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use super::gapfill::{fill_file, GapParams};
use super::join::{align, JoinParams};
use super::Feature;
use crate::align_mzmine::{JoinAlign, PeakFinder};
use crate::prog::Prog;
use crate::recipe::Recipe;
use crate::table::{Aligned, FileFeatures, CELL_ABSENT, CELL_FILLED};

/// Counts x seconds per intensity x minute (the areas of `gapfill.rs` are in intensity x minutes).
const MIN_TO_S: f64 = 60.0;

/// The m/z tolerance (ppm, Da) of a step whose `ppm` is `ppm` (minus Da on the unit axis) and whose absolute bound is `tol_da`.
fn tolerances(ppm: f64, tol_da: f64) -> (f64, f64) {
    if ppm < 0.0 {
        (0.0, tol_da.max(-ppm))
    } else {
        (ppm, tol_da)
    }
}

/// The parameters of the join for the recipe `r` (`ppm` is that of `features.align`).
pub fn join_params(r: &Recipe, j: &JoinAlign) -> JoinParams {
    let (tol_ppm, tol_da) = tolerances(r.align.ion.ppm, j.tol_da);
    JoinParams {
        tol_da,
        tol_ppm,
        w_mz: j.mz_weight,
        w_rt: j.rt_weight,
        rt_tol: j.rt_tol,
    }
}

/// The per-ion table of the join of `files`: the row of each feature of file `k` is found by `join::align`, its position travels in `Feature::apex`.
pub fn join_table(files: &[&FileFeatures], r: &Recipe, j: &JoinAlign) -> Result<Aligned> {
    let lists: Vec<Vec<Feature>> = files
        .iter()
        .map(|f| {
            (0..f.mz.len())
                .map(|i| Feature {
                    mz: f.mz[i],
                    rt: f.rt[i] as f64,
                    apex: i as u32,
                    ..Feature::default()
                })
                .collect()
        })
        .collect();
    let rows = align(&lists, &join_params(r, j));
    let mut out = Aligned::with_capacity(files.len(), rows.rows.len());
    out.n_files = files.len();
    let mut items = Vec::new();
    for row in &rows.rows {
        items.clear();
        for (k, c) in row.cells.iter().enumerate() {
            if let Some(f) = c {
                items.push((k as u32, f.apex));
            }
        }
        crate::align::push_group(files, &items, &mut out, None);
    }
    Ok(out)
}

/// The parameters of the peak finder for the recipe `r` (`floor`, `ppm`, `rt_tol` and `min_points` of `features.gapfill`).
pub fn gap_params(r: &Recipe, f: &PeakFinder) -> GapParams {
    let (tol_ppm, tol_da) = tolerances(r.fill.ppm, f.tol_da);
    GapParams {
        intensity_tol: f.intensity_tol,
        tol_da,
        tol_ppm,
        rt_tol: r.fill.rt_tol,
        min_points: r.fill.min_points,
        noise: r.fill.floor,
        polarity: r.detect.polarity,
    }
}

/// Fills the empty cells of `table` with the peak finder, one file after the other (`open(k)` = the index of file `k`) at the m/z and RT of the row.
/// A filled cell gets the area (counts x s), height, apex RT and width of the peak found and `CELL_FILLED`; the row m/z and RT do not move.
pub(crate) fn peak_finder<R: ReadAt>(
    table: &mut Aligned,
    n: usize,
    r: &Recipe,
    f: &PeakFinder,
    open: &mut dyn FnMut(usize) -> Result<Index<R>>,
    prog: &mut Prog,
) -> Result<usize> {
    let p = gap_params(r, f);
    let nf = table.n_files;
    let mut filled = 0;
    for k in 0..n {
        let rows: Vec<usize> = (0..table.mz.len())
            .filter(|&row| table.state[row * nf + k] == CELL_ABSENT)
            .collect();
        let gaps: Vec<(f64, f64)> = rows
            .iter()
            .map(|&row| (table.mz[row], table.rt[row]))
            .collect();
        for (&row, found) in rows.iter().zip(fill_file(&mut open(k)?, &gaps, &p)?) {
            let Some(c) = found else { continue };
            let at = row * nf + k;
            table.area[at] = c.area * MIN_TO_S;
            table.height[at] = c.height as f32;
            table.apex_rt[at] = c.rt as f32;
            table.width[at] = (c.rt_end - c.rt_start) as f32;
            table.state[at] = CELL_FILLED;
            filled += 1;
        }
        prog.tick(1);
    }
    Ok(filled)
}
