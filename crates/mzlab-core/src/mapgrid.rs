//! Arithmetic of the ion map (RT × m/z) that needs no file: the grid of the session, the m/z cells of a region, the constant background,
//! the tolerant difference and the groups of marked points. Ports of `Session.grid`, `map_cells`, `subtract_constant`, `diff_tolerant`,
//! `rt_groups` of `mzlab/explore.py`, in the same order of operations (numpy float32 sums are pairwise).

/// The common RT × m/z grid of the maps of a session (`Session.grid`).
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    pub rt0: f64,
    pub rt1: f64,
    pub nrt: usize,
    pub mz0: f64,
    pub dmz: f64,
    pub nmz: usize,
}

/// Python's `round()`: halves go to the even number.
pub fn py_round(x: f64) -> f64 {
    if (x - x.trunc()).abs() == 0.5 {
        2.0 * (x / 2.0).round()
    } else {
        x.round()
    }
}

/// `Session.grid` for low resolution from the RT limits (min and max of every file) and the m/z limits of the tables.
pub fn session_grid(rts: &[f64], mzs: &[f64]) -> Grid {
    if rts.is_empty() || mzs.is_empty() {
        return Grid { rt0: 0.0, rt1: 1.0, nrt: 1, mz0: 0.0, dmz: 1.0, nmz: 1 };
    }
    let lo = |v: &[f64]| v.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let (rt0, rt1) = (lo(rts).floor(), hi(rts).ceil());
    let nrt = py_round((rt1 - rt0) / 0.04).clamp(50.0, 600.0) as usize;
    let mz0 = lo(mzs).floor() - 0.2;
    let nmz = ((hi(mzs) - mz0) / 1.0 + 1e-9).ceil().max(1.0) as usize;
    Grid { rt0, rt1, nrt, mz0, dmz: 1.0, nmz }
}

/// `map_cells` for low resolution: when the cells are wider than 0.5 Da they are whole numbers of nominal masses `[n - 0.2; n + 0.8]`.
pub fn map_cells(mz0: f64, mz1: f64, nmz: usize) -> (f64, f64, usize) {
    let nmz = nmz.max(1);
    if (mz1 - mz0) / nmz as f64 >= 0.5 {
        let lo = (mz0 + 0.2).floor() - 0.2;
        let hi = (mz1 + 0.2).ceil() - 0.2;
        let w = ((hi - lo) / nmz as f64).ceil().max(1.0);
        let n = ((hi - lo) / w - 1e-9).ceil();
        return (lo, lo + w * n, n as usize);
    }
    (mz0, mz1, nmz)
}

/// `subtract_constant`: the background of each m/z column taken away from a map (rows = RT), what is below it stays 0.
pub fn subtract_constant(m: &[f32], nmz: usize, bg: &[f32]) -> Vec<f32> {
    m.iter().enumerate().map(|(i, &v)| (v - bg[i % nmz]).max(0.0)).collect()
}

/// `diff_tolerant` along RT only (low resolution): A − B where B is replaced by its local maximum over ±`w` rows (edges repeated).
pub fn diff_tolerant(a: &[f32], b: &[f32], nrt: usize, nmz: usize, w: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(a.len());
    for i in 0..nrt {
        let (lo, hi) = if w > 0 && nrt > 1 { (i.saturating_sub(w), (i + w).min(nrt - 1)) } else { (i, i) };
        for j in 0..nmz {
            let m = (lo..=hi).map(|r| b[r * nmz + j]).fold(f32::NEG_INFINITY, f32::max);
            out.push(a[i * nmz + j] - m);
        }
    }
    out
}

/// `ndarray.sum()` of a contiguous float32 array: pairwise in float32, blocks of 8 accumulators (as `stats::psum`, in single precision).
pub fn psum32(a: &[f32]) -> f32 {
    let n = a.len();
    if n < 8 {
        return a.iter().fold(0.0, |r, v| r + v);
    }
    if n <= 128 {
        let mut r: [f32; 8] = a[..8].try_into().unwrap();
        let mut i = 8;
        while i + 8 <= n - n % 8 {
            for k in 0..8 {
                r[k] += a[i + k];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        while i < n {
            res += a[i];
            i += 1;
        }
        return res;
    }
    let mut n2 = n / 2;
    n2 -= n2 % 8;
    psum32(&a[..n2]) + psum32(&a[n2..])
}

/// `np.median(full, axis=0)` of a row-major `rows × nmz` float32 matrix (the mean of the two middle values in single precision).
pub fn median_cols(full: &[f32], rows: usize, nmz: usize) -> Vec<f32> {
    (0..nmz)
        .map(|j| {
            let mut c: Vec<f32> = (0..rows).map(|r| full[r * nmz + j]).collect();
            c.sort_by(f32::total_cmp);
            match c.len() {
                0 => 0.0,
                n if n % 2 == 1 => c[n / 2],
                n => (c[n / 2 - 1] + c[n / 2]) / 2.0,
            }
        })
        .collect()
}

/// A marked point with its measures (`map_measure`) and, after `rt_groups`, its group and its m/z offset from the strongest one.
#[derive(Clone, Debug)]
pub struct Point {
    pub rt: f64,
    pub mz: f64,
    pub ia: f64,
    pub ib: Option<f64>,
    pub sn: Option<f64>,
    pub g: usize,
    pub dm: f64,
}

/// `rt_groups`: points whose apexes are within ±`tol` minutes chain into one group (1, 2, ... in RT order).
pub fn rt_groups(points: &mut [Point], tol: f64) {
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|&a, &b| points[a].rt.total_cmp(&points[b].rt));
    let (mut g, mut last) = (0, None::<f64>);
    for &i in &order {
        if last.map_or(true, |l| points[i].rt - l > tol) {
            g += 1;
        }
        points[i].g = g;
        last = Some(points[i].rt);
    }
    for k in 1..=g {
        let mem: Vec<usize> = (0..points.len()).filter(|&i| points[i].g == k).collect();
        let top = mem.iter().fold(mem[0], |t, &i| if points[i].ia > points[t].ia { i } else { t });
        let tmz = points[top].mz;
        for &i in &mem {
            points[i].dm = points[i].mz - tmz;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_goes_to_even() {
        assert_eq!([0.5, 1.5, 2.5, -2.5, 2.4].map(py_round), [0.0, 2.0, 2.0, -2.0, 2.0]);
    }

    #[test]
    fn nominal_cells_cover_whole_masses() {
        let (lo, hi, n) = map_cells(100.0, 500.0, 100);
        assert_eq!((n, (lo * 10.0).round(), (hi * 10.0).round()), (81, 998.0, 5048.0));
        assert_eq!(map_cells(150.0, 151.0, 4), (150.0, 151.0, 4));
    }

    #[test]
    fn tolerant_difference_cancels_a_drift() {
        let (mut a, mut b) = (vec![0.0f32; 6], vec![0.0f32; 6]);
        a[2] = 100.0;
        b[3] = 100.0;
        assert_eq!(diff_tolerant(&a, &b, 6, 1, 1)[2], 0.0);
        assert_eq!(diff_tolerant(&a, &b, 6, 1, 0)[2], 100.0);
    }
}
