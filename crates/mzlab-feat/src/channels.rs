//! Channels and segments: runs of scans in one log-ppm channel and the stitching of the pieces of one ion across neighbouring channels
//! (`_segments` and `_stitch`).

use mzlab_core::error::Result;

use crate::axis::{LR_STITCH, LR_TOUCH};
use crate::trace::{filled, reserve, Params, TOUCH_HEIGHT};

/// A run of scans in one channel.
#[derive(Clone, Copy)]
pub(crate) struct Seg {
    pub(crate) chan: i64,
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

/// The peaks of a slab in the order (scan, table position): a stable counting sort of `pos`, which `segments` uses instead of sorting each channel.
pub(crate) fn by_scan(pos: &[u32]) -> Result<Vec<u32>> {
    let nscan = pos.iter().max().map_or(0, |&m| m as usize + 1);
    let mut next = filled(nscan + 1, 0u32)?;
    for &s in pos {
        next[s as usize + 1] += 1;
    }
    for k in 0..nscan {
        next[k + 1] += next[k];
    }
    let mut out = filled(pos.len(), 0u32)?;
    for (i, &s) in pos.iter().enumerate() {
        out[next[s as usize] as usize] = i as u32;
        next[s as usize] += 1;
    }
    Ok(out)
}

/// The channels that can make a run: (first peak, end, next free slot of its block of the order) and the channel of every peak.
type Chans = (Vec<(u32, u32, u32)>, Vec<u32>);

fn find_channels(lmz: &[f64], it: &[f64], shift: f64, keep_floor: f64) -> Result<Chans> {
    let n = lmz.len();
    let mut chans: Vec<(u32, u32, u32)> = Vec::new();
    let mut chan_of = filled(n, u32::MAX)?;
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
        // A lone peak cannot make a run of 2 points.
        if e - s >= 2 && cmax >= keep_floor {
            reserve(&mut chans, 1)?;
            chan_of[s..e].fill(chans.len() as u32);
            chans.push((s as u32, e as u32, s as u32));
        }
        s = e;
    }
    Ok((chans, chan_of))
}

/// Runs of consecutive scans in each channel of a slab (`_segments`). `mz` ascending; channels whose strongest peak is below
/// `keep_floor` are dropped first. Only the runs with at least 2 points and a height of `min_height`. `scan_order` is `by_scan(pos)`:
/// the peaks of a channel are a block of the table, and walking `scan_order` once puts each of them in its channel by scan, ties in table order.
#[allow(clippy::too_many_arguments)]
pub(crate) fn segments(
    lmz: &[f64],
    mz: &[f64],
    it: &[f64],
    pos: &[u32],
    scan_order: &[u32],
    shift: f64,
    p: &Params,
    out: &mut Vec<Seg>,
) -> Result<()> {
    let n = lmz.len();
    let (chans, chan_of) = find_channels(lmz, it, shift, p.min_height / 4.0)?;
    let mut chans = chans;
    let mut order = filled(n, 0u32)?;
    for &i in scan_order {
        let c = chan_of[i as usize];
        if c != u32::MAX {
            let slot = &mut chans[c as usize].2;
            order[*slot as usize] = i;
            *slot += 1;
        }
    }
    for &(s, e, _) in &chans {
        let (s, e) = (s as usize, e as usize);
        let b = libm::floor(lmz[s] + shift) as i64;
        let order = &order[s..e];
        let mut k = 0;
        // current segment
        let mut cur: Option<(u32, u32, f64, f64, u32, f64, f64, u32)> = None; // (a, last, tot, mzw, n, mx, apexI, apex)
        while k < order.len() {
            let ps = pos[order[k] as usize];
            let (mut ii, mut mw) = (0.0f64, 0.0f64);
            while k < order.len() && pos[order[k] as usize] == ps {
                let i = order[k] as usize;
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
    let max_gap = p.max_gap as i64;
    let touch_height = if p.axis.is_da() {
        10.0 * p.min_height
    } else {
        TOUCH_HEIGHT
    };
    // the neighbour channel of the segments of one channel: (channel, first, end)
    let mut next = (i64::MIN, 0, 0);
    for i in 0..m {
        let c = segs[i].chan;
        if next.0 != c {
            let lo = segs.partition_point(|s| s.chan < c + 1);
            next = (c, lo, lo + segs[lo..].partition_point(|s| s.chan <= c + 1));
        }
        let (lo, hi) = (next.1, next.2);
        let si = &segs[i];
        // The segments of a channel come in scan order and apart (both `a` and `z` ascend), so those within reach of `si` are a run of them.
        let reach = si.a as i64 - max_gap;
        let from = lo + segs[lo..hi].partition_point(|s| (s.z as i64) < reach);
        for j in from..hi {
            let sj = &segs[j];
            if sj.a as i64 > si.z as i64 + max_gap {
                break;
            }
            let ov = sj.z.min(si.z) as i64 - sj.a.max(si.a) as i64 + 1;
            // neither the half overlap nor the touching test can hold below this
            if ov < 1 - max_gap {
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
            let ok = overlap || (ov >= 1 - max_gap && ov <= max_gap + 1 && like);
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
