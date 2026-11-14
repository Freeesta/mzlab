//! Channels and segments: runs of scans in one log-ppm channel and the stitching of the pieces of one ion across neighbouring channels
//! (`_segments` and `_stitch`).

use mzlab_core::error::Result;

use crate::axis::{LR_STITCH, LR_TOUCH};
use crate::trace::Params;
use crate::trace_base::{filled, reserve, TOUCH_HEIGHT};

/// A run of scans in one channel.
#[derive(Clone, Copy)]
pub(crate) struct Seg {
    pub(crate) chan: i32,
    pub(crate) mzc: f64,
    pub(crate) apex: u32,
    pub(crate) n: u32,
    pub(crate) mx: f64,
    pub(crate) tot: f64,
    pub(crate) a: u32,
    pub(crate) z: u32,
    /// Found by the broad-band pass (`broad.rs`).
    pub(crate) broad: bool,
}

// (a, last, tot, mzw, n, mx, apexI, apex): the segment being grown.
type OpenSeg = (u32, u32, f64, f64, u32, f64, f64, u32);

/// The channels that can make a run, as (first peak, end) of their block of the table.
fn find_channels(lmz: &[f64], it: &[f64], shift: f64, keep_floor: f64) -> Result<Vec<(u32, u32)>> {
    let n = lmz.len();
    let mut chans: Vec<(u32, u32)> = Vec::new();
    let mut s = 0;
    while s < n {
        let b = libm::floor(lmz[s] + shift) as i32;
        let mut e = s + 1;
        while e < n && libm::floor(lmz[e] + shift) as i32 == b {
            e += 1;
        }
        let cmax = it[s..e]
            .iter()
            .fold(f64::MIN, |m, &y| if y > m { y } else { m });
        // A lone peak cannot make a run of 2 points.
        if e - s >= 2 && cmax >= keep_floor {
            reserve(&mut chans, 1)?;
            chans.push((s as u32, e as u32));
        }
        s = e;
    }
    Ok(chans)
}

/// The peaks `s..e` of one channel in the order (scan, table position), into `order`: one sort of `(scan, position)` keys in `keys`, which stays in
/// the cache, where scattering the whole slab by scan would not.
pub(crate) fn by_scan_in(
    pos: &[u32],
    (s, e): (usize, usize),
    keys: &mut Vec<u64>,
    order: &mut Vec<u32>,
) -> Result<()> {
    keys.clear();
    reserve(keys, e - s)?;
    keys.extend((s..e).map(|i| (u64::from(pos[i]) << 32) | i as u64));
    crate::sortc::ord(keys);
    order.clear();
    reserve(order, e - s)?;
    order.extend(keys.iter().map(|&k| k as u32));
    Ok(())
}

/// Runs of consecutive scans in each channel of a slab (`_segments`). `mz` ascending; channels whose strongest peak is below
/// `keep_floor` are dropped first. Only the runs with at least 2 points and a height of `min_height`. The peaks of a channel are a block of the
/// table, walked by scan (`by_scan_in`), ties in table order.
pub(crate) fn segments(
    lmz: &[f64],
    mz: &[f64],
    it: &[f64],
    pos: &[u32],
    shift: f64,
    p: &Params,
    out: &mut Vec<Seg>,
) -> Result<()> {
    let chans = find_channels(lmz, it, shift, p.min_height / 4.0)?;
    let (mut keys, mut order) = (Vec::new(), Vec::new());
    for &(s, e) in &chans {
        let (s, e) = (s as usize, e as usize);
        let b = libm::floor(lmz[s] + shift) as i32;
        by_scan_in(pos, (s, e), &mut keys, &mut order)?;
        let order = &order[..];
        let mut k = 0;
        // current segment
        let mut cur: Option<OpenSeg> = None; // (a, last, tot, mzw, n, mx, apexI, apex)
        while k < order.len() {
            let ps = pos[order[k] as usize];
            let (mut ii, mut mw) = (0.0f64, 0.0f64);
            while k < order.len() && pos[order[k] as usize] == ps {
                let i = order[k] as usize;
                ii += it[i];
                mw += it[i] * mz[i];
                k += 1;
            }
            // `order` is sorted by scan (`by_scan_in`), so `ps >= c.1` and the subtraction cannot underflow.
            match cur.as_mut() {
                Some(c) if ps - c.1 <= p.max_gap + 1 => {
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
    Ok(())
}

fn push_seg(
    out: &mut Vec<Seg>,
    chan: i32,
    c: (u32, u32, f64, f64, u32, f64, f64, u32),
    p: &Params,
) -> Result<()> {
    let (a, z, tot, mzw, n, mx, _, apex) = c;
    // a low piece of a wandering ion counts once stitched to the rest; the rows still need `min_height` (`trace::one_grid`)
    let least = if p.stitch_wide {
        p.min_height / 4.0
    } else {
        p.min_height
    };
    if n >= 2 && mx >= least {
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
            broad: false,
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
pub(crate) fn stitch(segs: Vec<Seg>, p: &Params) -> Result<Vec<Seg>> {
    let m = segs.len();
    if m < 2 {
        return Ok(segs);
    }
    let mut parent: Vec<u32> = Vec::new();
    reserve(&mut parent, m)?;
    parent.extend(0..m as u32);
    link(&segs, p, &mut parent);
    join(&segs, &mut parent)
}

/// Union of the segments of adjacent channels that are pieces of one ion (the lowest index is the root).
fn link(segs: &[Seg], p: &Params, parent: &mut [u32]) {
    let m = segs.len();
    let (touch_ppm, touch_ratio) = (4.0f64, 10.0f64);
    // `reach` is the scan distance: `max_gap` missed scans plus one
    let reach = p.max_gap as i64 + 1;
    let touch_height = if p.stitch_wide {
        0.0
    } else if p.axis.is_da() {
        10.0 * p.min_height
    } else {
        TOUCH_HEIGHT
    };
    // the neighbour channel of the segments of one channel: (channel, first, end)
    let mut next = (i32::MIN, 0, 0);
    for i in 0..m {
        let c = segs[i].chan;
        if next.0 != c {
            let lo = segs.partition_point(|s| s.chan < c + 1);
            next = (c, lo, lo + segs[lo..].partition_point(|s| s.chan <= c + 1));
        }
        let (lo, hi) = (next.1, next.2);
        let si = &segs[i];
        // The segments of a channel come in scan order and apart (both `a` and `z` ascend), so those within reach of `si` are a run of them.
        let first = si.a as i64 - reach;
        let from = lo + segs[lo..hi].partition_point(|s| (s.z as i64) < first);
        for (j, sj) in segs.iter().enumerate().take(hi).skip(from) {
            if sj.a as i64 > si.z as i64 + reach {
                break;
            }
            let ov = sj.z.min(si.z) as i64 - sj.a.max(si.a) as i64 + 1;
            // neither the half overlap nor the touching test can hold below this
            if ov < 1 - reach {
                continue;
            }
            let short = (si.z as i64 - si.a as i64 + 1).min(sj.z as i64 - sj.a as i64 + 1);
            let (hi_mx, lo_mx) = (si.mx.max(sj.mx), si.mx.min(sj.mx));
            let near = if p.axis.is_da() {
                LR_TOUCH
            } else {
                touch_ppm * 1e-6 * si.mzc
            };
            let like = (si.mzc - sj.mzc).abs() <= near
                && hi_mx <= touch_ratio * lo_mx
                && lo_mx >= touch_height;
            // a unit channel holds distinct ions a dalton apart: pieces are one ion only if their centroids are close
            let overlap = ov as f64 >= 0.5 * short as f64
                && (!p.axis.is_da() || (si.mzc - sj.mzc).abs() <= LR_STITCH);
            let ok = overlap || (ov >= 1 - reach && ov <= reach + 1 && like);
            if ok {
                let (ra, rb) = (find(parent, i as u32), find(parent, j as u32));
                if ra != rb {
                    let (lo_r, hi_r) = (ra.min(rb), ra.max(rb));
                    parent[hi_r as usize] = lo_r;
                }
            }
        }
    }
}

/// One segment per component, numbered in the order of its lowest member: sums, extremes and the apex of the highest member.
fn join(segs: &[Seg], parent: &mut [u32]) -> Result<Vec<Seg>> {
    let m = segs.len();
    // group numbers in the order of the lowest member
    let mut gid: Vec<u32> = Vec::new();
    reserve(&mut gid, m)?;
    gid.resize(m, u32::MAX);
    let mut k = 0u32;
    let mut root_of: Vec<u32> = Vec::new();
    reserve(&mut root_of, m)?;
    for i in 0..m {
        let r = find(parent, i as u32);
        root_of.push(r);
        if gid[r as usize] == u32::MAX {
            gid[r as usize] = k;
            k += 1;
        }
    }
    let k = k as usize;
    let mut tots = filled(k, 0.0f64)?;
    let mut mzw = filled(k, 0.0f64)?;
    let mut ns = filled(k, 0u32)?;
    let mut mxs = filled(k, f64::MIN)?;
    let mut a2 = filled(k, u32::MAX)?;
    let mut z2 = filled(k, 0u32)?;
    let mut best = filled(k, usize::MAX)?; // member with the highest height (the first one on ties)
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
            broad: false,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
