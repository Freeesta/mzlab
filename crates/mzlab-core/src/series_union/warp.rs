//! The RT map of one series onto the first one (`rt_warp = piecewise`). Anchors are the rows that both series hold: for a row of the series, its nearest row
//! (in RT) of the reference within `da` and `rt_gap`, kept only when the choice is mutual. Their RT differences are cut into blocks of near-equal count along
//! the RT, each block gives its median point, and the map is the line through those points (flat outside). Fewer than 3 anchors: no map, said in the log.

use super::params::Params;
use super::read::Row;

pub struct Curve {
    /// (RT in the series, shift to add), by RT.
    pub nodes: Vec<(f64, f64)>,
    pub anchors: usize,
}

impl Curve {
    pub fn shift(&self, rt: f64) -> f64 {
        let n = &self.nodes;
        let Some(first) = n.first() else { return 0.0 };
        if rt <= first.0 {
            return first.1;
        }
        match n.windows(2).find(|w| rt <= w[1].0) {
            Some(w) => {
                let t = if w[1].0 > w[0].0 {
                    (rt - w[0].0) / (w[1].0 - w[0].0)
                } else {
                    0.0
                };
                w[0].1 + t * (w[1].1 - w[0].1)
            }
            None => n[n.len() - 1].1,
        }
    }

    /// (median, largest) absolute shift of the nodes, min.
    pub fn size(&self) -> (f64, f64) {
        let mut a: Vec<f64> = self.nodes.iter().map(|n| n.1.abs()).collect();
        crate::sortc::slice_by(&mut a, f64::total_cmp);
        (median(&a), a.last().copied().unwrap_or(0.0))
    }
}

pub fn median(sorted: &[f64]) -> f64 {
    match sorted.len() {
        0 => 0.0,
        n if n % 2 == 1 => sorted[n / 2],
        n => (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0,
    }
}

/// The indices of `rows` sorted by m/z, to scan the rows inside a window.
pub fn by_mz(rows: &[Row]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..rows.len()).collect();
    crate::sortc::slice_by(&mut idx, |&a, &b| {
        rows[a].mz.total_cmp(&rows[b].mz).then(a.cmp(&b))
    });
    idx
}

/// The rows of `sorted` (from `by_mz`) with m/z within `da` of `mz` and the same polarity, with their RT gap, the nearest RT first.
fn near(
    rows: &[Row],
    sorted: &[usize],
    mz: f64,
    pol: Option<i8>,
    rt: f64,
    da: f64,
) -> Vec<(f64, usize)> {
    let from = sorted.partition_point(|&i| rows[i].mz < mz - da);
    let mut hit: Vec<(f64, usize)> = sorted[from..]
        .iter()
        .take_while(|&&i| rows[i].mz <= mz + da)
        .filter(|&&i| rows[i].polarity == pol)
        .map(|&i| ((rows[i].rt - rt).abs(), i))
        .collect();
    crate::sortc::slice_by(&mut hit, |a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    hit
}

pub fn build(reference: &[Row], rows: &[Row], p: &Params) -> Option<Curve> {
    let (ref_sorted, own_sorted) = (by_mz(reference), by_mz(rows));
    let mut anchors: Vec<(f64, f64)> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let Some(&(gap, j)) = near(reference, &ref_sorted, r.mz, r.polarity, r.rt, p.da).first()
        else {
            continue;
        };
        let back = near(
            rows,
            &own_sorted,
            reference[j].mz,
            reference[j].polarity,
            reference[j].rt,
            p.da,
        );
        if gap <= p.rt_gap && back.first().map(|b| b.1) == Some(i) {
            anchors.push((r.rt, reference[j].rt - r.rt));
        }
    }
    if anchors.len() < 3 {
        return None;
    }
    crate::sortc::slice_by(&mut anchors, |a, b| {
        a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1))
    });
    let (n, blocks) = (anchors.len(), (anchors.len() / 3).clamp(1, 8));
    let nodes = (0..blocks)
        .map(|b| {
            let block = &anchors[b * n / blocks..(b + 1) * n / blocks];
            let mid = |f: fn(&(f64, f64)) -> f64| {
                let mut v: Vec<f64> = block.iter().map(f).collect();
                crate::sortc::slice_by(&mut v, f64::total_cmp);
                median(&v)
            };
            (mid(|a| a.0), mid(|a| a.1))
        })
        .collect();
    Some(Curve { nodes, anchors: n })
}
