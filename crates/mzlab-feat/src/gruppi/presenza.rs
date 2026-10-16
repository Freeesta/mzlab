//! The conditions of a series and the presence of a row in them: replicates grouped by (condition, time), their mean area and the rule of presence.
//! Every list of files is sorted by (injection order, file name), so the sums do not depend on the order of the columns.

use super::{Ctx, GroupOpts, GroupTable, ParentOut};
use crate::legacy::lr::kin::{decay, Point};
use crate::table::{CELL_DETECTED, CELL_TARGETED};
use mzlab_core::campioni::Campione;
use mzlab_core::json::round;

/// The controls of a condition: its own by kind, else the shared ones (`serie == ""`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Controls {
    pub t0: Vec<usize>,
    pub blank: Vec<usize>,
    pub std: Vec<usize>,
    pub dark: Vec<usize>,
}

/// The replicates of one condition at one time (`None` = time not known).
pub(crate) struct Group {
    pub t: Option<f64>,
    pub files: Vec<usize>,
}

pub(crate) struct CondModel {
    pub id: String,
    pub files: Vec<usize>,
    pub groups: Vec<Group>,
    pub ctrl: Controls,
}

impl CondModel {
    /// The known treatment times, ascending.
    pub fn times(&self) -> Vec<f64> {
        self.groups.iter().filter_map(|g| g.t).collect()
    }
}

/// Mean area of one group of replicates in one row.
#[derive(Debug, Clone)]
pub(crate) struct GStat {
    pub t: Option<f64>,
    pub n: usize,
    pub n_det: usize,
    /// Replicates averaged: the detected ones, else the filled ones.
    pub n_used: usize,
    pub mean: f64,
    pub sd: Option<f64>,
}

pub(crate) fn detected(state: u8) -> bool {
    state == CELL_DETECTED || state == CELL_TARGETED
}

fn sort_files(v: &mut [usize], cs: &[Campione]) {
    mzlab_core::sortc::by(v, |&a, &b| {
        cs[a]
            .order
            .cmp(&cs[b].order)
            .then_with(|| cs[a].file.cmp(&cs[b].file))
            .then(a.cmp(&b))
    });
}

fn is_treated(c: &Campione) -> bool {
    c.role() == "sample"
}

fn groups_of(files: &[usize], cs: &[Campione]) -> Vec<Group> {
    let mut gs: Vec<Group> = Vec::new();
    for &f in files {
        let t = cs[f].time_min;
        match gs.iter_mut().find(|g| g.t == t) {
            Some(g) => g.files.push(f),
            None => gs.push(Group { t, files: vec![f] }),
        }
    }
    let mut ix: Vec<usize> = (0..gs.len()).collect();
    let key = |i: usize| gs[i].t.unwrap_or(f64::INFINITY);
    mzlab_core::sortc::by(&mut ix, |&a, &b| key(a).total_cmp(&key(b)));
    let mut slots: Vec<Option<Group>> = gs.into_iter().map(Some).collect();
    ix.into_iter().filter_map(|i| slots[i].take()).collect()
}

fn controls_of(cs: &[Campione], id: &str) -> Controls {
    let pick = |role: &str| {
        let by = |own: bool| {
            let mut v: Vec<usize> = (0..cs.len())
                .filter(|&i| {
                    cs[i].role() == role
                        && (cs[i].series == id) == own
                        && (own || cs[i].series.is_empty())
                })
                .collect();
            sort_files(&mut v, cs);
            v
        };
        let own = by(true);
        if own.is_empty() {
            by(false)
        } else {
            own
        }
    };
    Controls {
        t0: pick("t0"),
        blank: pick("blank"),
        std: pick("std"),
        dark: pick("dark"),
    }
}

/// The conditions in order of first appearance (smallest injection order of their samples).
pub(crate) fn conditions(cs: &[Campione]) -> Vec<CondModel> {
    let mut found: Vec<(u32, String)> = Vec::new();
    for c in cs.iter().filter(|c| is_treated(c) && !c.series.is_empty()) {
        match found.iter_mut().find(|(_, s)| s == &c.series) {
            Some(e) => e.0 = e.0.min(c.order),
            None => found.push((c.order, c.series.clone())),
        }
    }
    let mut ix: Vec<usize> = (0..found.len()).collect();
    mzlab_core::sortc::by(&mut ix, |&a, &b| found[a].cmp(&found[b]));
    ix.into_iter()
        .map(|i| {
            let id = found[i].1.clone();
            let mut files: Vec<usize> = (0..cs.len())
                .filter(|&f| is_treated(&cs[f]) && cs[f].series == id)
                .collect();
            sort_files(&mut files, cs);
            CondModel {
                groups: groups_of(&files, cs),
                ctrl: controls_of(cs, &id),
                files,
                id,
            }
        })
        .collect()
}

/// Mean and sample standard deviation of `v` (None for fewer than two values).
fn mean_sd(v: &[f64]) -> (f64, Option<f64>) {
    let n = v.len() as f64;
    let m = v.iter().sum::<f64>() / n;
    let sd =
        (v.len() >= 2).then(|| (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt());
    (m, sd)
}

/// Mean area of the detected replicates; when none is detected, of the filled ones (to show how absent "absent" is); else 0.
pub(crate) fn group_stat(tb: &GroupTable, r: usize, g: &Group) -> GStat {
    let cell = |f: usize| r * tb.n_files + f;
    let det: Vec<f64> = g
        .files
        .iter()
        .filter(|&&f| detected(tb.state[cell(f)]))
        .map(|&f| tb.area[cell(f)])
        .collect();
    let fill: Vec<f64> = if det.is_empty() {
        g.files
            .iter()
            .filter(|&&f| tb.state[cell(f)] == crate::table::CELL_FILLED)
            .map(|&f| tb.area[cell(f)])
            .collect()
    } else {
        Vec::new()
    };
    let used = if det.is_empty() { &fill } else { &det };
    let (mean, sd) = if used.is_empty() {
        (0.0, None)
    } else {
        mean_sd(used)
    };
    GStat {
        t: g.t,
        n: g.files.len(),
        n_det: det.len(),
        n_used: used.len(),
        mean,
        sd,
    }
}

/// Replicates needed to call a group present: `ceil(frac * n)`, at least one.
fn need(n: usize, frac: f64) -> usize {
    ((frac * n as f64) - 1e-9).ceil().max(1.0) as usize
}

/// Present: detected in enough replicates of at least one treated time, with a mean area of at least `min_area`. Filled cells never count.
pub(crate) fn is_present(stats: &[GStat], o: &GroupOpts) -> bool {
    stats
        .iter()
        .any(|s| s.n_det > 0 && s.n_det >= need(s.n, o.min_rep_frac) && s.mean >= o.min_area)
}

/// Decay of the parent in a condition and the fraction left at its last time compared with the time zero of the condition.
pub(crate) fn parent(cx: &Ctx, p: usize, c: &CondModel) -> ParentOut {
    let stats: Vec<GStat> = c.groups.iter().map(|g| group_stat(cx.tb, p, g)).collect();
    let pts: Vec<Point> = stats
        .iter()
        .filter_map(|s| {
            s.t.map(|t| Point {
                time: t,
                area: if s.n_det > 0 { s.mean } else { 0.0 },
                height: 0.0,
            })
        })
        .collect();
    let d = decay(&pts);
    let t0 = Group {
        t: Some(0.0),
        files: c.ctrl.t0.clone(),
    };
    let base = group_stat(cx.tb, p, &t0);
    let last = pts.last().map(|q| q.area);
    let remaining = match last {
        Some(a) if base.n_det > 0 && base.mean > 0.0 => Some(round(a / base.mean, 6)),
        _ => None,
    };
    ParentOut {
        k: d.map(|d| round(d.k, 6)),
        half_life: d.and_then(|d| d.half_life).map(|h| round(h, 6)),
        remaining,
    }
}
