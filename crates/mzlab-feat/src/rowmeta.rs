//! The summaries of the rows of an aligned table (`table::RowMeta`): for each row the peak of its reference ion in every file that detected it is found in
//! the table of that file (m/z within `WINDOW_PPM` and the apex RT of the cell, bit for bit), and the edges, the charge, the quality, the M+1/M and the best
//! MS2 are taken from there. The files are read in blocks of rows (the table is sorted by m/z) so that the scratch stays small whatever the series.

use mzlab_core::error::Result;

use crate::trace::filled;
use crate::axis::half;
use crate::table::{Aligned, FileFeatures, RowMeta, RowQ, CELL_DETECTED, NO_FILE, ROLE_ISOTOPE};

/// Distance in m/z (ppm) between the mean m/z of a row and the peak of a file that belongs to it (the alignment accepts up to 10 ppm of span).
pub const WINDOW_PPM: f64 = 15.0;
/// Distance of the M+1 peak from the position expected for an isotopologue (ppm).
const ISO_PPM: f64 = 10.0;
/// Mass of the extra neutron between isotopologues (Da).
const NEUTRON: f64 = 1.003355;
/// Scratch of a block (five values per cell), bytes.
const SCRATCH: usize = 16_000_000;

/// The peak of `f` at `(mz, apex)`: same RT as the apex of the cell, m/z within the window, the largest area if several.
fn find(f: &FileFeatures, mz: f64, apex: f32, win: f64) -> Option<usize> {
    let tol = half(mz, win);
    let (a, b) = (f.mz.partition_point(|&m| m < mz - tol), f.mz.partition_point(|&m| m <= mz + tol));
    (a..b).filter(|&j| f.rt[j].to_bits() == apex.to_bits()).max_by(|&x, &y| f.area[x].total_cmp(&f.area[y]))
}

/// Median of a small list (mean of the two middle values for an even count); NaN when empty.
fn median(v: &mut [f32]) -> f32 {
    for i in 1..v.len() {
        let mut j = i;
        while j > 0 && v[j - 1] > v[j] {
            v.swap(j - 1, j);
            j -= 1;
        }
    }
    match v.len() {
        0 => f32::NAN,
        n if n % 2 == 1 => v[n / 2],
        n => (v[n / 2 - 1] + v[n / 2]) / 2.0,
    }
}

/// M+1/M of the peak `j` of `f` when it belongs to a family: the area of the isotopologue one neutron up over its own, both from the window of the peak.
fn m_plus_one(f: &FileFeatures, j: usize) -> f32 {
    let fam = f.family_id.get(j).copied().unwrap_or(0);
    let z = f.charge.get(j).copied().unwrap_or(1).unsigned_abs().max(1) as f64;
    if fam == 0 || f.area_win.get(j).copied().unwrap_or(0.0) <= 0.0 {
        return f32::NAN;
    }
    let want = f.mz[j] + NEUTRON / z;
    let tol = want * ISO_PPM * 1e-6;
    let (a, b) = (f.mz.partition_point(|&m| m < want - tol), f.mz.partition_point(|&m| m <= want + tol));
    (a..b)
        .filter(|&i| f.family_id[i] == fam && f.role[i] == ROLE_ISOTOPE)
        .min_by(|&x, &y| (f.mz[x] - want).abs().total_cmp(&(f.mz[y] - want).abs()))
        .map_or(f32::NAN, |i| (f.area_win[i] / f.area_win[j]) as f32)
}

/// Computes `table.meta`. `load(k, range)` gives file `k` with at least the rows whose m/z is in `range` (all its columns). The table must be sorted by m/z.
pub fn compute(table: &mut Aligned, load: &mut dyn FnMut(usize, (f64, f64)) -> Result<FileFeatures>) -> Result<()> {
    compute_in(table, WINDOW_PPM, load)
}

/// `compute` with the window of m/z of a peak of a file around the mean m/z of a row (ppm, or minus Da: unit resolution, where M+1/M is left undefined).
pub fn compute_in(table: &mut Aligned, win: f64, load: &mut dyn FnMut(usize, (f64, f64)) -> Result<FileFeatures>) -> Result<()> {
    debug_assert!(table.is_id_ordered(), "the rows are sorted before their summaries are computed");
    let (rows, nf) = (table.len(), table.n_files);
    let mut meta = RowMeta::default();
    let block = (SCRATCH / (20 * nf.max(1))).clamp(64, 1 << 20);
    let mut a = 0;
    while a < rows {
        let b = (a + block).min(rows);
        let (lo, hi) = if win < 0.0 {
            (table.mz[a] + 2.0 * win, table.mz[b - 1] - 2.0 * win + 1.1)
        } else {
            (table.mz[a] * (1.0 - 2.0 * win * 1e-6), table.mz[b - 1] * (1.0 + 2.0 * win * 1e-6) + 1.1)
        };
        let n = b - a;
        // left, right, shape, snr, fwhm of every cell, then the best peak of each row
        let mut grid = filled(5 * n * nf, f32::NAN)?;
        let mut best: Vec<(f64, f32, i8, u32, f32)> = filled(n, (-1.0, f32::NAN, 0, NO_FILE, -1.0))?;
        let mut qreason = filled(n, 0u8)?;
        for k in 0..nf {
            let f = load(k, (lo, hi))?;
            for r in a..b {
                let c = table.cell(r, k);
                if table.state[c] != CELL_DETECTED || table.apex_rt[c].is_nan() {
                    continue;
                }
                let Some(j) = find(&f, table.mz[r], table.apex_rt[c], win) else { continue };
                let (i, g) = (r - a, (r - a) * nf + k);
                let cols = [f.left[j], f.right[j], f.shape[j], f.snr[j], f.fwhm[j]];
                for (q, v) in cols.iter().enumerate() {
                    grid[q * n * nf + g] = *v;
                }
                qreason[i] |= f.qreason[j];
                let e = &mut best[i];
                if f.area[j] > e.0 {
                    *e = (f.area[j], if win < 0.0 { f32::NAN } else { m_plus_one(&f, j) }, f.charge[j], e.3, e.4);
                }
                if f.ms2_n[j] > 0 && f.purity[j] > e.4 {
                    e.3 = k as u32;
                    e.4 = f.purity[j];
                }
            }
        }
        for i in 0..n {
            let med = |q: usize| {
                let mut v: Vec<f32> = grid[q * n * nf + i * nf..q * n * nf + (i + 1) * nf].iter().copied().filter(|x| !x.is_nan()).collect();
                median(&mut v)
            };
            let (area, m1, charge, ms2_file, ms2_purity) = best[i];
            meta.push(&RowQ {
                rt_lo: med(0),
                rt_hi: med(1),
                charge: if area >= 0.0 { charge } else { 0 },
                q_shape: med(2),
                q_snr: med(3),
                q_fwhm: med(4),
                qreason: qreason[i],
                iso_m1: m1,
                ms2_file,
                ms2_purity,
            });
        }
        a = b;
    }
    table.meta = meta;
    Ok(())
}
