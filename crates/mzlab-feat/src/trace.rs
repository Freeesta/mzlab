//! Ion traces of the MS1 peaks of one file and polarity: port of `TP_Mine/py/tpmine/hr/features.py::detect`.
//!
//! Log-ppm channels on two shifted grids; (channel, scan) cells; a segment is a run of scans in one channel with gaps <= `max_gap`.
//! Segments of neighbouring channels that cover the same scans are stitched, the two grids are merged and the Fourier-transform side
//! lobes of very intense ions are marked. The peaks come from the index in slabs of m/z (a slab ends only at a gap of 50 ppm or more
//! between two peaks above the floor, where no trace can pass), so the memory is bounded and the result does not depend on the slabs.
//! Sums run in a fixed order and `ln` comes from `libm`: native and WASM give the same bits.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};

use crate::table::{FileFeatures, FLAG_BACKGROUND, FLAG_FT_LOBE};

pub const CHANNEL_PPM: f64 = 4e-6;
pub const FLOOR: f64 = 2e4;
pub const MIN_HEIGHT: f64 = 2e5;
pub const MIN_POINTS: u32 = 5;
pub const LONG_TRACE: u32 = 400;
pub const MAX_GAP: u32 = 2;
pub const SLAB_PEAKS: usize = 1_500_000;
pub const TOUCH_HEIGHT: f64 = 2e6;
/// A slab never ends at a smaller gap (ppm) between neighbouring peaks.
pub const SLAB_GAP_PPM: f64 = 50.0;
/// Bytes of working memory per peak of a slab (m/z, intensity, scan, log m/z, channel, ...).
const BYTES_PER_PEAK: usize = 64;

#[derive(Debug, Clone)]
pub struct Params {
    /// Polarity of the MS1 scans (1 or -1); `None` = all the MS1 scans.
    pub polarity: Option<i8>,
    pub floor: f64,
    pub min_points: u32,
    pub min_height: f64,
    pub max_gap: u32,
    pub long_trace: u32,
    /// Working memory of the detection, bytes: it sets the size of the slabs (at most `SLAB_PEAKS` peaks).
    pub budget: usize,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            polarity: Some(1),
            floor: FLOOR,
            min_points: MIN_POINTS,
            min_height: MIN_HEIGHT,
            max_gap: MAX_GAP,
            long_trace: LONG_TRACE,
            budget: SLAB_PEAKS * BYTES_PER_PEAK,
        }
    }
}

impl Params {
    fn slab_peaks(&self) -> usize {
        (self.budget / BYTES_PER_PEAK).clamp(1024, SLAB_PEAKS)
    }
}

/// A run of scans in one channel.
#[derive(Clone, Copy)]
struct Seg {
    chan: i64,
    mzc: f64,
    apex: u32,
    n: u32,
    mx: f64,
    tot: f64,
    a: u32,
    z: u32,
}

/// A feature before the grids are merged (columns of `F` in `features.py`).
#[derive(Clone, Copy)]
struct Row {
    mz: f64,
    rt: f64,
    n: u32,
    height: f64,
    area: f64,
    apex: u32,
    lo: u32,
    hi: u32,
    left: f64,
    right: f64,
    long: bool,
    grid: u8,
}

fn reserve<T>(v: &mut Vec<T>, extra: usize) -> Result<()> {
    v.try_reserve(extra).map_err(|_| Error::memory_budget())
}

/// Runs of consecutive scans in each channel of a slab (`_segments`). `mz` ascending; channels whose strongest peak is below
/// `keep_floor` are dropped first. Only the runs with at least 2 points and a height of `min_height`.
fn segments(
    lmz: &[f64],
    mz: &[f64],
    it: &[f64],
    pos: &[u32],
    shift: f64,
    p: &Params,
    out: &mut Vec<Seg>,
) -> Result<()> {
    let keep_floor = p.min_height / 4.0;
    let n = lmz.len();
    let mut order: Vec<(u32, u32)> = Vec::new();
    let mut s = 0;
    while s < n {
        let b = libm::floor(lmz[s] + shift) as i64;
        let mut e = s + 1;
        while e < n && libm::floor(lmz[e] + shift) as i64 == b {
            e += 1;
        }
        let cmax = it[s..e]
            .iter()
            .fold(f64::MIN, |m, &y| if y > m { y } else { m });
        if cmax >= keep_floor {
            order.clear();
            reserve(&mut order, e - s)?;
            order.extend((s..e).map(|i| (pos[i], i as u32)));
            order.sort_by_key(|&(p, _)| p); // stable: ties keep the table order
            let mut k = 0;
            // current segment
            let mut cur: Option<(u32, u32, f64, f64, u32, f64, f64, u32)> = None; // (a, last, tot, mzw, n, mx, apexI, apex)
            while k < order.len() {
                let ps = order[k].0;
                let (mut ii, mut mw) = (0.0f64, 0.0f64);
                while k < order.len() && order[k].0 == ps {
                    let i = order[k].1 as usize;
                    ii += it[i];
                    mw += it[i] * mz[i];
                    k += 1;
                }
                match cur.as_mut() {
                    Some(c) if ps - c.1 <= p.max_gap => {
                        c.1 = ps;
                        c.2 += ii;
                        c.3 += mw;
                        c.4 += 1;
                        if ii > c.5 {
                            c.5 = ii;
                            c.7 = ps;
                        }
                    }
                    _ => {
                        if let Some(c) = cur.take() {
                            push_seg(out, b, c, p)?;
                        }
                        cur = Some((ps, ps, ii, mw, 1, ii, ii, ps));
                    }
                }
            }
            if let Some(c) = cur.take() {
                push_seg(out, b, c, p)?;
            }
        }
        s = e;
    }
    Ok(())
}

fn push_seg(
    out: &mut Vec<Seg>,
    chan: i64,
    c: (u32, u32, f64, f64, u32, f64, f64, u32),
    p: &Params,
) -> Result<()> {
    let (a, z, tot, mzw, n, mx, _, apex) = c;
    if n >= 2 && mx >= p.min_height {
        reserve(out, 1)?;
        out.push(Seg {
            chan,
            mzc: mzw / tot,
            apex,
            n,
            mx,
            tot,
            a,
            z,
        });
    }
    Ok(())
}

fn find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        let g = parent[parent[x as usize] as usize];
        parent[x as usize] = g;
        x = g;
    }
    x
}

/// Merge the segments of adjacent channels that are pieces of one ion (`_stitch`): scan ranges overlapping by at least half of the
/// shorter one, or touching, alike and intense. Components keep the order of their lowest member.
fn stitch(segs: Vec<Seg>, p: &Params) -> Result<Vec<Seg>> {
    let m = segs.len();
    if m < 2 {
        return Ok(segs);
    }
    let mut parent: Vec<u32> = Vec::new();
    reserve(&mut parent, m)?;
    parent.extend(0..m as u32);
    let (touch_ppm, touch_ratio) = (4.0f64, 10.0f64);
    let max_gap = p.max_gap as i64;
    for i in 0..m {
        let c = segs[i].chan;
        let lo = segs.partition_point(|s| s.chan < c + 1);
        let hi = segs.partition_point(|s| s.chan <= c + 1);
        let si = &segs[i];
        for j in lo..hi {
            let sj = &segs[j];
            let ov = sj.z.min(si.z) as i64 - sj.a.max(si.a) as i64 + 1;
            let short = (si.z as i64 - si.a as i64 + 1).min(sj.z as i64 - sj.a as i64 + 1);
            let (hi_mx, lo_mx) = (si.mx.max(sj.mx), si.mx.min(sj.mx));
            let like = (si.mzc - sj.mzc).abs() <= touch_ppm * 1e-6 * si.mzc
                && hi_mx <= touch_ratio * lo_mx
                && lo_mx >= TOUCH_HEIGHT;
            let ok =
                ov as f64 >= 0.5 * short as f64 || (ov >= 1 - max_gap && ov <= max_gap + 1 && like);
            if ok {
                let (ra, rb) = (find(&mut parent, i as u32), find(&mut parent, j as u32));
                if ra != rb {
                    let (lo_r, hi_r) = (ra.min(rb), ra.max(rb));
                    parent[hi_r as usize] = lo_r;
                }
            }
        }
    }
    // group numbers in the order of the lowest member
    let mut gid: Vec<u32> = Vec::new();
    reserve(&mut gid, m)?;
    gid.resize(m, u32::MAX);
    let mut k = 0u32;
    let mut root_of: Vec<u32> = Vec::new();
    reserve(&mut root_of, m)?;
    for i in 0..m {
        let r = find(&mut parent, i as u32);
        root_of.push(r);
        if gid[r as usize] == u32::MAX {
            gid[r as usize] = k;
            k += 1;
        }
    }
    let k = k as usize;
    let mut tots = vec![0.0f64; k];
    let mut mzw = vec![0.0f64; k];
    let mut ns = vec![0u32; k];
    let mut mxs = vec![f64::MIN; k];
    let mut a2 = vec![u32::MAX; k];
    let mut z2 = vec![0u32; k];
    let mut best = vec![usize::MAX; k]; // member with the highest height (the first one on ties)
    for (i, s) in segs.iter().enumerate() {
        let g = gid[root_of[i] as usize] as usize;
        tots[g] += s.tot;
        mzw[g] += s.mzc * s.tot;
        ns[g] += s.n;
        if s.mx > mxs[g] {
            mxs[g] = s.mx;
        }
        a2[g] = a2[g].min(s.a);
        z2[g] = z2[g].max(s.z);
        if best[g] == usize::MAX || s.mx > segs[best[g]].mx {
            best[g] = i;
        }
    }
    let mut out = Vec::new();
    reserve(&mut out, k)?;
    for g in 0..k {
        let b = &segs[best[g]];
        out.push(Seg {
            chan: b.chan,
            mzc: mzw[g] / tots[g],
            apex: b.apex,
            n: ns[g],
            mx: mxs[g],
            tot: tots[g],
            a: a2[g],
            z: z2[g],
        });
    }
    Ok(out)
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
    o.sort_by(|&a, &b| f[a as usize].mz.total_cmp(&f[b as usize].mz));
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
    o.sort_by(|&a, &b| f[a as usize].mz.total_cmp(&f[b as usize].mz));
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

fn median_interval(rt: &[f64]) -> f64 {
    if rt.len() < 2 {
        return 1.0;
    }
    let mut d: Vec<f64> = rt.windows(2).map(|w| w[1] - w[0]).collect();
    d.sort_by(|a, b| a.total_cmp(b));
    let n = d.len();
    let med = if n % 2 == 1 {
        d[n / 2]
    } else {
        (d[n / 2 - 1] + d[n / 2]) / 2.0
    };
    med * 60.0
}

/// One slab of peaks (m/z ascending, above the floor) through both grids.
struct Slab {
    mz: Vec<f64>,
    it: Vec<f64>,
    pos: Vec<u32>,
}

fn run_slab(slab: &Slab, rt: &[f64], dt: f64, p: &Params, rows: &mut [Vec<Row>; 2]) -> Result<()> {
    if slab.mz.is_empty() {
        return Ok(());
    }
    let mut lmz: Vec<f64> = Vec::new();
    reserve(&mut lmz, slab.mz.len())?;
    let den = libm::log1p(CHANNEL_PPM);
    lmz.extend(slab.mz.iter().map(|&m| libm::log(m) / den));
    for (g, shift) in [0.0f64, 0.5].into_iter().enumerate() {
        let mut segs = Vec::new();
        segments(&lmz, &slab.mz, &slab.it, &slab.pos, shift, p, &mut segs)?;
        let segs = stitch(segs, p)?;
        for s in segs {
            if s.n >= p.min_points && s.mx >= p.min_height {
                reserve(&mut rows[g], 1)?;
                rows[g].push(Row {
                    mz: s.mzc,
                    rt: rt[s.apex as usize],
                    n: s.n,
                    height: s.mx,
                    area: s.tot * dt,
                    apex: s.apex,
                    lo: s.a,
                    hi: s.z,
                    left: rt[s.a as usize],
                    right: rt[s.z as usize],
                    long: s.z - s.a + 1 > p.long_trace,
                    grid: g as u8,
                });
            }
        }
    }
    Ok(())
}

/// Ion traces of the MS1 scans of an index (one polarity, or all).
pub fn detect<R: ReadAt>(index: &mut Index<R>, p: &Params) -> Result<FileFeatures> {
    let Some(si) = index.section_of(1, p.polarity) else {
        return Ok(FileFeatures::default());
    };
    let rt: Vec<f64> = index.arrays(si)?.rt.clone();
    let dt = median_interval(&rt);
    let nscan = rt.len();
    let n_peaks = index.sections[si].n_peaks;
    let max_slab = p.slab_peaks();
    let mut rows: [Vec<Row>; 2] = [Vec::new(), Vec::new()];
    let mut slab = Slab {
        mz: Vec::new(),
        it: Vec::new(),
        pos: Vec::new(),
    };
    const STEP: u64 = 1 << 16;
    let mut a = 0u64;
    let mut chunk: Vec<(u32, f64, f64)> = Vec::new();
    while a < n_peaks {
        let b = (a + STEP).min(n_peaks);
        chunk.clear();
        index.each_peak(si, a, b, |pos, mz, it| {
            if it >= p.floor {
                chunk.push((pos as u32, mz, it));
            }
        })?;
        for &(pos, mz, it) in &chunk {
            if (pos as usize) >= nscan {
                continue;
            }
            if slab.mz.len() >= max_slab {
                let last = slab.mz[slab.mz.len() - 1];
                if (mz - last) / mz * 1e6 >= SLAB_GAP_PPM {
                    run_slab(&slab, &rt, dt, p, &mut rows)?;
                    slab.mz.clear();
                    slab.it.clear();
                    slab.pos.clear();
                }
            }
            reserve(&mut slab.mz, 1)?;
            reserve(&mut slab.it, 1)?;
            reserve(&mut slab.pos, 1)?;
            slab.mz.push(mz);
            slab.it.push(it);
            slab.pos.push(pos);
        }
        a = b;
    }
    run_slab(&slab, &rt, dt, p, &mut rows)?;
    drop(slab);

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
    idx.sort_by(|&a, &b| f[a as usize].mz.total_cmp(&f[b as usize].mz));
    let n = f.len();
    let mut out = FileFeatures {
        dt,
        ..Default::default()
    };
    macro_rules! cols {
        ($($c:ident),*) => { $( reserve(&mut out.$c, n)?; )* };
    }
    cols!(
        mz, rt, left, right, apex, scan_lo, scan_hi, area, height, npts, flags, iso_group, charge
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
    }
    Ok(out)
}
