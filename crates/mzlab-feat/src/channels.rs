//! Channels and segments: runs of scans in one log-ppm channel and the stitching of the pieces of one ion across neighbouring channels
//! (`_segments` and `_stitch` of `features.py`).

use crate::sortc;
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
}

/// Runs of consecutive scans in each channel of a slab (`_segments`). `mz` ascending; channels whose strongest peak is below
/// `keep_floor` are dropped first. Only the runs with at least 2 points and a height of `min_height`.
pub(crate) fn segments(
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
            sortc::by_key(&mut order, |&(p, _)| p); // stable: ties keep the table order
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
        });
    }
    Ok(out)
}
