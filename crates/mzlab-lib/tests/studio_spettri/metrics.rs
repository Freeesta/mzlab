//! Metrics of the spectral study: Recall@1, MRR, the area under the precision-recall curve up to 1 % true FDP, the calibration error of the
//! estimated q against the true FDP, the share of queries passing the `shared` gate of `livelli-1.js`, the q of true hits against traps and
//! the separation of q between right and wrong top hits.

use super::arms::Row;
use super::scen::Q;
use serde_json::{json, Value};

/// Gate `shared` of `mzlab/web/livelli-1.js`: `picchi_min` 6, or `picchi_min_pochi` 3 when the library spectrum has fewer than 6 peaks.
pub fn gate(r: &Row) -> bool {
    !r.ranked.is_empty() && r.shared >= if r.nlib < 6 { 3 } else { 6 }
}

fn correct(r: &Row, q: &Q) -> bool {
    q.truth.is_some() && r.ranked.first().map(|x| x.0) == q.truth
}

/// Area under the precision-recall curve of the top hits ranked by score, over the longest prefix (whole groups of equal scores) whose true FDP
/// stays <= 1 %; the maximum is the recall reached at 1 % FDP.
pub fn pauprc(rows: &[&Row], qs: &[&Q]) -> f64 {
    let n_pos = qs.iter().filter(|q| q.truth.is_some()).count();
    let mut idx: Vec<usize> = (0..rows.len())
        .filter(|&i| !rows[i].ranked.is_empty())
        .collect();
    idx.sort_by(|&a, &b| rows[b].ranked[0].1.total_cmp(&rows[a].ranked[0].1));
    let (mut tp, mut fp, mut area, mut best) = (0usize, 0usize, 0.0, 0.0);
    let mut k = 0;
    while k < idx.len() && n_pos > 0 {
        let s = rows[idx[k]].ranked[0].1;
        let tp0 = tp;
        while k < idx.len() && rows[idx[k]].ranked[0].1 == s {
            if correct(rows[idx[k]], qs[idx[k]]) {
                tp += 1
            } else {
                fp += 1
            }
            k += 1;
        }
        area += (tp - tp0) as f64 / n_pos as f64 * tp as f64 / (tp + fp) as f64;
        if fp as f64 <= 0.01 * (tp + fp) as f64 {
            best = area;
        }
    }
    best
}

/// Calibration of q: at 20 thresholds tau log-spaced in [0.001, 0.1] the hits with q <= tau are accepted; their estimated FDR is the largest q
/// among them, their true FDP the share of wrong top hits. Returns the mean |FDP - estimate| and the true FDP at q <= 0.01.
pub fn calibration(rows: &[&Row], qs: &[&Q]) -> (f64, f64) {
    let at = |tau: f64| {
        let acc: Vec<usize> = (0..rows.len())
            .filter(|&i| !rows[i].ranked.is_empty() && rows[i].q as f64 <= tau)
            .collect();
        let wrong = acc.iter().filter(|&&i| !correct(rows[i], qs[i])).count();
        let est = acc.iter().map(|&i| rows[i].q as f64).fold(0.0, f64::max);
        (!acc.is_empty()).then(|| (wrong as f64 / acc.len() as f64, est))
    };
    let errs: Vec<f64> = (0..20)
        .filter_map(|j| {
            at(libm::pow(10.0, -3.0 + 2.0 * j as f64 / 19.0)).map(|(f, e)| (f - e).abs())
        })
        .collect();
    let ece = if errs.is_empty() {
        0.0
    } else {
        errs.iter().sum::<f64>() / errs.len() as f64
    };
    (ece, at(0.01).map_or(0.0, |x| x.0))
}

/// Separation of q: the probability that a wrong top hit (a trap or an error) gets a higher q than a right one, ties counting half; 1 is a
/// perfect separation, 0.5 none. `None` when either side is empty.
pub fn q_auc(rows: &[&Row], qs: &[&Q]) -> Option<f64> {
    let (mut right, mut wrong) = (Vec::new(), Vec::new());
    for i in (0..rows.len()).filter(|&i| !rows[i].ranked.is_empty()) {
        if correct(rows[i], qs[i]) {
            &mut right
        } else {
            &mut wrong
        }
        .push(rows[i].q);
    }
    if right.is_empty() || wrong.is_empty() {
        return None;
    }
    right.sort_by(f32::total_cmp);
    let mut sum = 0.0;
    for w in &wrong {
        let lo = right.partition_point(|x| x < w);
        let eq = right.partition_point(|x| x <= w) - lo;
        sum += lo as f64 + 0.5 * eq as f64;
    }
    Some(sum / (right.len() * wrong.len()) as f64)
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn r4(x: f64) -> Value {
    if x.is_finite() {
        json!((x * 1e4).round() / 1e4 + 0.0)
    } else {
        Value::Null
    }
}

/// Metrics of the rows of one (sub)group.
pub fn metrics(rows: &[&Row], qs: &[&Q]) -> Value {
    let pos: Vec<usize> = (0..qs.len()).filter(|&i| qs[i].truth.is_some()).collect();
    let np = pos.len().max(1) as f64;
    let recall = pos.iter().filter(|&&i| correct(rows[i], qs[i])).count() as f64 / np;
    let mrr = pos
        .iter()
        .map(|&i| {
            let r = rows[i].ranked.iter().position(|x| Some(x.0) == qs[i].truth);
            r.map_or(0.0, |r| 1.0 / (r + 1) as f64)
        })
        .sum::<f64>()
        / np;
    let (ece, fdp1) = calibration(rows, qs);
    // power of the null: right top hits accepted at q <= 0.01, over the queries whose truth is in the library
    let acc1 = pos
        .iter()
        .filter(|&&i| correct(rows[i], qs[i]) && rows[i].q <= 0.01)
        .count() as f64
        / np;
    let share = |f: &dyn Fn(&Row) -> bool| {
        rows.iter().filter(|r| f(r)).count() as f64 / rows.len().max(1) as f64
    };
    let (gate, hit) = (share(&|r| gate(r)), share(&|r| !r.ranked.is_empty()));
    let q_of = |trap: bool| {
        median(
            (0..qs.len())
                .filter(|&i| qs[i].truth.is_none() == trap)
                .map(|i| rows[i].q as f64)
                .collect(),
        )
    };
    let score = median(
        rows.iter()
            .filter_map(|r| r.ranked.first().map(|x| x.1 as f64))
            .collect(),
    );
    let auc = q_auc(rows, qs).unwrap_or(f64::NAN);
    json!({
        "n": qs.len(), "score": r4(score), "n_pos": pos.len(), "recall1": r4(recall), "mrr": r4(mrr), "pauprc": r4(pauprc(rows, qs)),
        "ece": r4(ece), "fdp_q01": r4(fdp1), "recall_q01": r4(acc1), "gate": r4(gate), "hit": r4(hit), "q_true": r4(q_of(false)), "q_trap": r4(q_of(true)),
        "q_auc": r4(auc),
    })
}

/// `all` plus one entry per subgroup of the queries and per prefix before `/` (`e1` of `e1/true` and `e1/trap`), sorted by name.
pub fn report(rows: &[Row], qs: &[Q]) -> Value {
    let all = metrics(
        &rows.iter().collect::<Vec<_>>(),
        &qs.iter().collect::<Vec<_>>(),
    );
    let prefix = |g: &str| g.split_once('/').map(|x| x.0.to_string());
    let mut groups: Vec<String> = qs.iter().map(|q| q.group.clone()).collect();
    groups.extend(qs.iter().filter_map(|q| prefix(&q.group)));
    groups.sort();
    groups.dedup();
    let mut g = serde_json::Map::new();
    for name in groups {
        let i: Vec<usize> = (0..qs.len())
            .filter(|&i| {
                qs[i].group == name || prefix(&qs[i].group).as_deref() == Some(name.as_str())
            })
            .collect();
        let r: Vec<&Row> = i.iter().map(|&k| &rows[k]).collect();
        let q: Vec<&Q> = i.iter().map(|&k| &qs[k]).collect();
        g.insert(name, metrics(&r, &q));
    }
    json!({ "all": all, "groups": g })
}
