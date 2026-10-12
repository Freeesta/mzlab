//! The last steps of the detection: the two grids are merged, the Fourier-transform side lobes are marked (`_merge_grids`,
//! `fourier_artefacts` of `features.py`).

use crate::sortc;
use mzlab_core::error::Result;

use crate::table::{FileFeatures, FLAG_BACKGROUND, FLAG_FT_LOBE};
use crate::trace::reserve;

/// A feature before the grids are merged (columns of `F` in `features.py`).
#[derive(Clone, Copy)]
pub(crate) struct Row {
    pub(crate) mz: f64,
    pub(crate) rt: f64,
    pub(crate) n: u32,
    pub(crate) height: f64,
    pub(crate) area: f64,
    pub(crate) apex: u32,
    pub(crate) lo: u32,
    pub(crate) hi: u32,
    pub(crate) left: f64,
    pub(crate) right: f64,
    pub(crate) long: bool,
    pub(crate) grid: u8,
}

/// Rows of the features seen on both grids: the one with more points stays (`_merge_grids`). Returns the keep mask.
fn merge_grids(f: &[Row]) -> Result<Vec<bool>> {
    let (rt_tol, ppm, slack) = (0.05f64, 5e-6f64, 0.02f64);
    let n = f.len();
    let mut keep = Vec::new();
    reserve(&mut keep, n)?;
    keep.resize(n, true);
    let mut o: Vec<u32> = Vec::new();
    reserve(&mut o, n)?;
    o.extend(0..n as u32);
    sortc::by(&mut o, |&a, &b| {
        f[a as usize].mz.total_cmp(&f[b as usize].mz)
    });
    for j in 1..6usize {
        if n <= j {
            break;
        }
        for t in 0..n - j {
            let (yi, xi) = (o[t] as usize, o[t + j] as usize);
            let (x, y) = (&f[xi], &f[yi]);
            let near = (x.mz - y.mz).abs() <= x.mz * ppm && x.grid != y.grid;
            if !near {
                continue;
            }
            let inside = (x.left >= y.left - slack && x.right <= y.right + slack)
                || (y.left >= x.left - slack && y.right <= x.right + slack);
            if (x.rt - y.rt).abs() <= rt_tol || inside {
                if x.n <= y.n {
                    keep[xi] = false;
                } else {
                    keep[yi] = false;
                }
            }
        }
    }
    Ok(keep)
}

/// Side lobes of Fourier-transform spectra (`fourier_artefacts`): a feature whose apex is within one scan of that of a feature 50 times
/// more intense, 20-200 ppm away from it.
fn fourier_artefacts(f: &[Row]) -> Result<Vec<bool>> {
    let (ratio, ppm_lo, ppm_hi) = (50.0f64, 20.0f64, 200.0f64);
    let n = f.len();
    let mut out = Vec::new();
    reserve(&mut out, n)?;
    out.resize(n, false);
    if n == 0 {
        return Ok(out);
    }
    let hmin = f
        .iter()
        .fold(f64::MAX, |m, r| if r.height < m { r.height } else { m });
    let refs: Vec<usize> = (0..n).filter(|&i| f[i].height >= ratio * hmin).collect();
    if refs.is_empty() {
        return Ok(out);
    }
    let mut o: Vec<u32> = Vec::new();
    reserve(&mut o, n)?;
    o.extend(0..n as u32);
    sortc::by(&mut o, |&a, &b| {
        f[a as usize].mz.total_cmp(&f[b as usize].mz)
    });
    for &r in &refs {
        let m = f[r].mz;
        let lo = o.partition_point(|&i| f[i as usize].mz < m * (1.0 - ppm_hi * 1e-6));
        let hi = o.partition_point(|&i| f[i as usize].mz <= m * (1.0 + ppm_hi * 1e-6));
        for &c in &o[lo..hi] {
            let c = c as usize;
            let d = (f[c].mz - m).abs() / m * 1e6;
            let da = (f[c].apex as f64 - f[r].apex as f64).abs();
            if d >= ppm_lo && d <= ppm_hi && da <= 1.0 && f[r].height >= ratio * f[c].height {
                out[c] = true;
            }
        }
    }
    Ok(out)
}

pub(crate) fn median_interval(rt: &[f64]) -> f64 {
    if rt.len() < 2 {
        return 1.0;
    }
    let mut d: Vec<f64> = rt.windows(2).map(|w| w[1] - w[0]).collect();
    sortc::by(&mut d, |a, b| a.total_cmp(b));
    let n = d.len();
    let med = if n % 2 == 1 {
        d[n / 2]
    } else {
        (d[n / 2 - 1] + d[n / 2]) / 2.0
    };
    med * 60.0
}

/// The merged and sorted table: grids merged, side lobes marked, rows in m/z order.
pub(crate) fn finish(rows: [Vec<Row>; 2], dt: f64) -> Result<FileFeatures> {
    let [r0, r1] = rows;
    let mut f = r0;
    reserve(&mut f, r1.len())?;
    f.extend(r1);
    let keep = merge_grids(&f)?;
    let mut k = keep.iter();
    f.retain(|_| *k.next().unwrap());
    let art = fourier_artefacts(&f)?;
    let mut idx: Vec<u32> = Vec::new();
    reserve(&mut idx, f.len())?;
    idx.extend(0..f.len() as u32);
    sortc::by(&mut idx, |&a, &b| {
        f[a as usize].mz.total_cmp(&f[b as usize].mz)
    });
    let n = f.len();
    let mut out = FileFeatures {
        dt,
        ..Default::default()
    };
    macro_rules! cols {
        ($($c:ident),*) => { $( reserve(&mut out.$c, n)?; )* };
    }
    cols!(
        mz, rt, left, right, apex, scan_lo, scan_hi, area, height, npts, flags, iso_group, charge,
        shape, snr, fwhm, asym, gaps, qreason, ms2_n, purity, area_win, family_id, role, alt
    );
    for &i in &idx {
        let r = &f[i as usize];
        out.mz.push(r.mz);
        out.rt.push(r.rt as f32);
        out.left.push(r.left as f32);
        out.right.push(r.right as f32);
        out.apex.push(r.apex);
        out.scan_lo.push(r.lo);
        out.scan_hi.push(r.hi);
        out.area.push(r.area);
        out.height.push(r.height as f32);
        out.npts.push(r.n.min(u16::MAX as u32) as u16);
        let mut fl = 0u8;
        if r.long {
            fl |= FLAG_BACKGROUND;
        }
        if art[i as usize] {
            fl |= FLAG_FT_LOBE;
        }
        out.flags.push(fl);
        out.iso_group.push(0);
        out.charge.push(0);
        out.shape.push(0.0);
        out.snr.push(0.0);
        out.fwhm.push(0.0);
        out.asym.push(0.0);
        out.gaps.push(0);
        out.qreason.push(0);
        out.ms2_n.push(0);
        out.purity.push(-1.0);
        out.area_win.push(0.0);
        out.family_id.push(0);
        out.role.push(0);
        out.alt.push(0);
    }
    Ok(out)
}
