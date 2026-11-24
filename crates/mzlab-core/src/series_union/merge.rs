//! The matching: series after series against the targets built so far. A row joins the target whose first row is the nearest in RT (then in m/z) within
//! `da` and `rt_tol`, one row per series per target, one target per row; the rest start new targets. Matches are made in order of distance, ties by position,
//! so the result does not depend on anything but the order of the series.

use super::params::{Params, Warp};
use super::read::Series;
use super::warp::{build, Curve};

pub struct Member {
    pub series: usize,
    pub row: usize,
    /// RT in the frame of the first series.
    pub rt: f64,
}

pub struct Target {
    pub members: Vec<Member>,
    pub polarity: Option<i8>,
    /// m/z and RT of its first row, the one the others are compared with.
    pub mz0: f64,
    pub rt0: f64,
}

/// What the RT map did to one series.
pub struct Warped {
    pub curve: Option<Curve>,
}

pub struct Merged {
    pub targets: Vec<Target>,
    pub warps: Vec<Warped>,
}

fn pairs(s: &Series, rts: &[f64], targets: &[Target], p: &Params) -> Vec<(f64, f64, usize, usize)> {
    let mut order: Vec<usize> = (0..targets.len()).collect();
    crate::sortc::slice_by(&mut order, |&a, &b| {
        targets[a].mz0.total_cmp(&targets[b].mz0).then(a.cmp(&b))
    });
    let mut out = Vec::new();
    for (i, r) in s.rows.iter().enumerate() {
        let from = order.partition_point(|&t| targets[t].mz0 < r.mz - p.da);
        for &t in order[from..]
            .iter()
            .take_while(|&&t| targets[t].mz0 <= r.mz + p.da)
        {
            let (dr, dm) = (
                (rts[i] - targets[t].rt0).abs(),
                (r.mz - targets[t].mz0).abs(),
            );
            if targets[t].polarity == r.polarity && dr <= p.rt_tol {
                out.push((dr, dm, i, t));
            }
        }
    }
    crate::sortc::slice_by(&mut out, |a, b| {
        a.0.total_cmp(&b.0)
            .then(a.1.total_cmp(&b.1))
            .then(a.2.cmp(&b.2))
            .then(a.3.cmp(&b.3))
    });
    out
}

pub fn merge(series: &[Series], p: &Params) -> Merged {
    let mut targets: Vec<Target> = Vec::new();
    let mut warps = Vec::new();
    for (k, s) in series.iter().enumerate() {
        let curve = if k > 0 && p.warp == Warp::Piecewise {
            build(&series[0].rows, &s.rows, p)
        } else {
            None
        };
        let rts: Vec<f64> = s
            .rows
            .iter()
            .map(|r| r.rt + curve.as_ref().map_or(0.0, |c| c.shift(r.rt)))
            .collect();
        let mut taken = vec![false; s.rows.len()];
        let mut used = vec![false; targets.len()];
        for (_, _, i, t) in pairs(s, &rts, &targets, p) {
            if !taken[i] && !used[t] {
                taken[i] = true;
                used[t] = true;
                targets[t].members.push(Member {
                    series: k,
                    row: i,
                    rt: rts[i],
                });
            }
        }
        for (i, r) in s.rows.iter().enumerate().filter(|(i, _)| !taken[*i]) {
            targets.push(Target {
                members: vec![Member {
                    series: k,
                    row: i,
                    rt: rts[i],
                }],
                polarity: r.polarity,
                mz0: r.mz,
                rt0: rts[i],
            });
        }
        warps.push(Warped { curve });
    }
    Merged { targets, warps }
}
