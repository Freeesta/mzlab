//! Arm D of the spectral study, analogues with two modifications: candidates from an index of the inter-peak differences (the edge channel),
//! scored by the overlap of the differences (`nl`), by a cosine with the shifts {0, δ1, δ2, Δprec} (`mc4`) and by a 2-shift Sankoff DP on the
//! max-likelihood arborescence of the candidate (`sankoff`); δ1 comes from the top of a vote histogram, δ2 = Δprec - δ1. Test code only.

use super::arms::{self, Lib, Row, FRAG_DA};
use mzlab_lib::decoy::quantize;
use mzlab_lib::flash::FlashAcc;
use mzlab_lib::search::{analogs_with, AnalogOpts, Query};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const ARMS: [&str; 4] = ["nl", "mc4", "sankoff", "mc4_flash"];
/// Window, minimum shift, candidates and matched pairs of a returned analogue: those of `search::AnalogOpts::default`.
pub const MAX_DELTA: f64 = 200.0;
pub const MIN_DELTA: f64 = 0.5;
pub const CANDIDATES: usize = 300;
pub const MIN_MATCHED: usize = 4;
pub const TOP: usize = 10;
/// Bin of the difference index and of the vote histogram, in Da.
pub const BIN: f64 = 0.01;

/// A spectrum as the arms see it: precursor and peaks sorted by m/z.
pub struct S {
    pub prec: f64,
    pub p: Vec<(f64, f64)>,
}

impl S {
    pub fn new(prec: f64, mz: &[f64], it: &[f64]) -> S {
        let mut p: Vec<(f64, f64)> = mz.iter().copied().zip(it.iter().copied()).collect();
        p.sort_by(|a, b| a.0.total_cmp(&b.0));
        S { prec, p }
    }
}

fn key(d: f64) -> i64 {
    (d / BIN).round() as i64
}

/// The differences of a spectrum: every pair of peaks and every peak against the precursor (losses), with weight sqrt(Ii Ij) (a loss weighs
/// the peak's intensity), sorted by mass.
pub fn diffs(s: &S) -> Vec<(f64, f64)> {
    let mut v = Vec::new();
    for j in 0..s.p.len() {
        v.push((s.prec - s.p[j].0, s.p[j].1));
        for i in 0..j {
            v.push((s.p[j].0 - s.p[i].0, (s.p[i].1 * s.p[j].1).sqrt()));
        }
    }
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    v
}

/// Peak bins (offset by `PEAK`, the engine's fragment index), loss bins (precursor - peak, its neutral-loss index) and inter-peak difference
/// bins (the edge channel) of a spectrum.
fn bins(s: &S) -> BTreeSet<i64> {
    let mut k: BTreeSet<i64> = diffs(s).iter().map(|d| key(d.0)).collect();
    k.extend(s.p.iter().map(|x| key(x.0) + PEAK));
    k
}

/// Offset of the peak bins: far above any difference bin.
const PEAK: i64 = 1 << 40;

/// The library as the arms see it, with the edge channel: per difference bin the spectra that have it (once each).
pub struct Index {
    pub specs: Vec<S>,
    post: HashMap<i64, Vec<u32>>,
    /// Distinct inter-peak difference bins per spectrum: the entries the edge channel adds to the engine's peak and loss index.
    pub keys: Vec<usize>,
    /// Max-likelihood arborescence of every spectrum (`arborescence`).
    parents: Vec<Vec<Option<usize>>>,
}

impl Index {
    pub fn new(l: &Lib) -> Index {
        let n = l.lib.n_spectra();
        let specs: Vec<S> = (0..n)
            .map(|i| {
                let (mz, it) = l.lib.peaks(i).unwrap();
                S::new(l.lib.rec(i).unwrap().prec, &mz, &it)
            })
            .collect();
        let mut post: HashMap<i64, Vec<u32>> = HashMap::new();
        let mut keys = Vec::with_capacity(specs.len());
        for (i, s) in specs.iter().enumerate() {
            let inter = (0..s.p.len()).flat_map(|j| (0..j).map(move |i| (j, i)));
            keys.push(
                inter
                    .map(|(j, i)| key(s.p[j].0 - s.p[i].0))
                    .collect::<BTreeSet<_>>()
                    .len(),
            );
            for k in bins(s) {
                post.entry(k).or_default().push(i as u32);
            }
        }
        let mut ix = Index {
            specs,
            post,
            keys,
            parents: Vec::new(),
        };
        ix.parents = ix.specs.iter().map(|c| arborescence(&ix, c)).collect();
        ix
    }

    /// Spectra of the library with a difference in the bins of `d` ± 1 (the frequency of a loss).
    fn df(&self, d: f64) -> usize {
        let k = key(d);
        (k - 1..=k + 1)
            .map(|k| self.post.get(&k).map_or(0, |v| v.len()))
            .sum()
    }

    /// The `CANDIDATES` spectra of the analogue window sharing the most bins (± 1) with the query; ties by library index.
    fn candidates(&self, q: &S) -> Vec<u32> {
        let ks: BTreeSet<i64> = bins(q)
            .into_iter()
            .flat_map(|k| [k - 1, k, k + 1])
            .collect();
        let mut count = vec![0u32; self.specs.len()];
        for k in ks {
            for &i in self.post.get(&k).map_or(&[][..], |v| v.as_slice()) {
                count[i as usize] += 1;
            }
        }
        let win = |i: usize| {
            let d = (q.prec - self.specs[i].prec).abs();
            count[i] > 0 && (MIN_DELTA..=MAX_DELTA).contains(&d)
        };
        let mut c: Vec<u32> = (0..self.specs.len())
            .filter(|&i| win(i))
            .map(|i| i as u32)
            .collect();
        c.sort_by(|&a, &b| count[b as usize].cmp(&count[a as usize]).then(a.cmp(&b)));
        c.truncate(CANDIDATES);
        c
    }
}

/// Cosine of `a` against `b` (both sorted by mass) with `a = b + shift` for any of `shifts`: pairs within `FRAG_DA`, one-to-one greedy by
/// decreasing sqrt(wa wb) as `scores::modified_cosine`, normalised by sqrt(sum wa * sum wb). Returns the score and the pairs.
fn cos(a: &[(f64, f64)], b: &[(f64, f64)], shifts: &[f64]) -> (f64, usize) {
    let mut c: Vec<(f64, usize, usize)> = Vec::new();
    for (i, x) in a.iter().enumerate() {
        for &sh in shifts {
            let t = x.0 - sh;
            let mut j = b.partition_point(|y| y.0 < t - FRAG_DA);
            while j < b.len() && b[j].0 <= t + FRAG_DA {
                c.push(((x.1 * b[j].1).sqrt(), i, j));
                j += 1;
            }
        }
    }
    c.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2)));
    let (mut ua, mut ub) = (vec![false; a.len()], vec![false; b.len()]);
    let (mut dot, mut n) = (0.0, 0);
    for (w, i, j) in c {
        if !ua[i] && !ub[j] {
            (ua[i], ub[j]) = (true, true);
            dot += w;
            n += 1;
        }
    }
    let norm = (a.iter().map(|x| x.1).sum::<f64>() * b.iter().map(|x| x.1).sum::<f64>()).sqrt();
    (if norm > 0.0 { dot / norm } else { 0.0 }, n)
}

/// δ1 and δ2 = Δprec - δ1: every query - candidate peak difference outside `MIN_DELTA` of 0 and of Δprec votes in its bin; a bin scores its
/// votes ± 1 bin plus those of the bin of Δprec - δ (the other modification); δ1 is the mean difference of the winning bins (ties: more own votes,
/// then the smaller |δ|). `None` without a vote.
pub fn vote(q: &S, c: &S) -> Option<(f64, f64)> {
    let dp = q.prec - c.prec;
    let mut hist: BTreeMap<i64, (usize, f64)> = BTreeMap::new();
    for x in &q.p {
        for y in &c.p {
            let d = x.0 - y.0;
            if d.abs() >= MIN_DELTA && (d - dp).abs() >= MIN_DELTA && d.abs() <= MAX_DELTA {
                let e = hist.entry(key(d)).or_insert((0, 0.0));
                (e.0, e.1) = (e.0 + 1, e.1 + d);
            }
        }
    }
    let n3 = |k: i64| {
        (k - 1..=k + 1)
            .map(|k| hist.get(&k).map_or(0, |e| e.0))
            .sum::<usize>()
    };
    let best = hist
        .keys()
        .map(|&k| (n3(k) + n3(key(dp - k as f64 * BIN)), n3(k), k))
        .max_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.1.cmp(&b.1))
                .then(b.2.abs().cmp(&a.2.abs()))
                .then(b.2.cmp(&a.2))
        })?;
    let (n, s) = (best.2 - 1..=best.2 + 1)
        .filter_map(|k| hist.get(&k))
        .fold((0, 0.0), |a, e| (a.0 + e.0, a.1 + e.1));
    let d1 = s / n as f64;
    Some((d1, dp - d1))
}

/// Parent of every candidate peak in the max-likelihood arborescence: the heavier peak or the precursor (`None`) whose difference is the most
/// frequent loss of the library (`Index::df`); ties go to the lighter parent, the precursor last.
fn arborescence(ix: &Index, c: &S) -> Vec<Option<usize>> {
    (0..c.p.len())
        .map(|i| {
            let mut best = (ix.df(c.prec - c.p[i].0), None);
            for j in (i + 1..c.p.len()).rev() {
                let f = ix.df(c.p[j].0 - c.p[i].0);
                if c.p[j].0 - c.p[i].0 > FRAG_DA && f >= best.0 {
                    best = (f, Some(j));
                }
            }
            best.1
        })
        .collect()
}

/// States of a node: the modifications it carries (bit 0: δ1, bit 1: δ2); a child carries a subset of its parent's, the precursor both.
const STATES: [u8; 4] = [0, 1, 2, 3];

/// Sankoff DP of the query on the arborescence of `c`: every node takes the state that maximises the matched sqrt(Iq Ic) of its subtree, a
/// node in state s pairing with a query peak at its m/z + shift(s); the pairs are then made one-to-one (a query peak keeps its best node).
fn sankoff(q: &S, c: &S, parent: &[Option<usize>], d: Option<(f64, f64)>) -> (f64, usize) {
    let dp = q.prec - c.prec;
    let shift = |s: u8| match (s, d) {
        (0, _) => Some(0.0),
        (3, _) => Some(dp),
        (1, Some(d)) => Some(d.0),
        (2, Some(d)) => Some(d.1),
        _ => None,
    };
    let n = c.p.len();
    // best query peak of node i in state s
    let pick = |i: usize, s: u8| -> Option<(f64, usize)> {
        let t = c.p[i].0 + shift(s)?;
        let lo = q.p.partition_point(|x| x.0 < t - FRAG_DA);
        (lo..q.p.len())
            .take_while(|&k| q.p[k].0 <= t + FRAG_DA)
            .map(|k| ((q.p[k].1 * c.p[i].1).sqrt(), k))
            .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
    };
    let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n + 1];
    for i in 0..n {
        kids[parent[i].unwrap_or(n)].push(i);
    }
    // children are lighter than their parent, so ascending m/z visits them first
    let mut best = vec![[0.0f64; 4]; n];
    for i in 0..n {
        for s in STATES {
            let own = pick(i, s).map_or(0.0, |x| x.0);
            let sub: f64 = kids[i].iter().map(|&k| sub_best(&best[k], s).1).sum();
            best[i][s as usize] = own + sub;
        }
    }
    // traceback from the precursor (state 3); ties go to the lower state (fewer modifications)
    let mut state = vec![0u8; n];
    let mut stack: Vec<(usize, u8)> = kids[n].iter().map(|&k| (k, 3)).collect();
    while let Some((i, ps)) = stack.pop() {
        state[i] = sub_best(&best[i], ps).0;
        stack.extend(kids[i].iter().map(|&k| (k, state[i])));
    }
    let mut used: BTreeMap<usize, f64> = BTreeMap::new();
    for (i, &s) in state.iter().enumerate() {
        if let Some((w, k)) = pick(i, s) {
            let e = used.entry(k).or_insert(0.0);
            *e = e.max(w);
        }
    }
    let norm = (q.p.iter().map(|x| x.1).sum::<f64>() * c.p.iter().map(|x| x.1).sum::<f64>()).sqrt();
    let dot: f64 = used.values().sum();
    (if norm > 0.0 { dot / norm } else { 0.0 }, used.len())
}

/// The best state of a child under a parent in state `ps` (a subset of its modifications) and its value; ties to the lower state.
fn sub_best(b: &[f64; 4], ps: u8) -> (u8, f64) {
    STATES
        .iter()
        .filter(|&&s| s & ps == s)
        .fold((0, f64::NEG_INFINITY), |a, &s| {
            if b[s as usize] > a.1 {
                (s, b[s as usize])
            } else {
                a
            }
        })
}

/// Score and matched pairs (differences for `nl`) of candidate `c` under `arm`.
fn score(arm: &str, ix: &Index, q: &S, qd: &[(f64, f64)], i: usize) -> (f64, usize) {
    let c = &ix.specs[i];
    match arm {
        "nl" => cos(qd, &diffs(c), &[0.0]),
        "mc4" => {
            let dp = q.prec - c.prec;
            let mut sh = vec![0.0, dp];
            if let Some(d) = vote(q, c) {
                sh.extend([d.0, d.1]);
            }
            cos(&q.p, &c.p, &sh)
        }
        _ => sankoff(q, c, &ix.parents[i], vote(q, c)),
    }
}

/// The row of one query: candidates with >= `MIN_MATCHED` pairs ranked by the quantized score, the first `TOP` kept (targets and decoys, as
/// `arms::analog_row` sees them).
fn row(arm: &str, l: &Lib, ix: &Index, q: &Query, acc: &mut FlashAcc) -> Row {
    let qs = S::new(q.prec, &q.spec.mz, &q.spec.it);
    let qd = diffs(&qs);
    let mut hits: Vec<(f32, usize, u32)> = Vec::new();
    let cands = if arm == "mc4_flash" {
        flash(l, q, acc)
    } else {
        ix.candidates(&qs)
    };
    for i in cands {
        let (s, m) = score(arm.trim_end_matches("_flash"), ix, &qs, &qd, i as usize);
        if m >= MIN_MATCHED {
            hits.push((quantize(s), m, i));
        }
    }
    hits.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.2.cmp(&b.2)));
    hits.truncate(TOP);
    let mut r = Row::default();
    for (s, m, i) in hits {
        match l.ids[i as usize] {
            Some(id) => {
                if r.ranked.is_empty() {
                    (r.shared, r.nlib) = (m, ix.specs[i as usize].p.len());
                }
                r.ranked.push((id, s));
            }
            None => r.decoy = r.decoy.max(s),
        }
    }
    r
}

/// The candidates of the engine's analogue search (Flash hybrid, >= 3 pairs at shift 0 or Δprec) without its modified-cosine cut: `mc4_flash`
/// rescores them, so it shows what the edge channel adds to the candidates.
fn flash(l: &Lib, q: &Query, acc: &mut FlashAcc) -> Vec<u32> {
    let o = AnalogOpts {
        frag_da: FRAG_DA,
        pol: 1,
        min_matched: 0,
        top: CANDIDATES,
        ..AnalogOpts::default()
    };
    let mut c: Vec<u32> = analogs_with(&l.lib, q, &o, acc)
        .unwrap()
        .iter()
        .map(|h| h.index)
        .collect();
    c.sort();
    c
}

/// Share of the E5 queries whose voted pair {δ1, δ2} against the true parent misses the true shifts by more than `FRAG_DA`.
fn delta_wrong(st: &super::Setup, l: &Lib, ix: &Index, qs: &[Option<Query>]) -> Value {
    let at: HashMap<usize, usize> = (0..l.ids.len())
        .filter_map(|i| l.ids[i].map(|id| (id, i)))
        .collect();
    let (mut n, mut bad) = (0usize, 0usize);
    for (q, sq) in qs.iter().zip(&st.s.queries) {
        let (Some(q), Some(id), [a, b]) = (q, sq.truth, sq.deltas.as_slice()) else {
            continue;
        };
        n += 1;
        let qs = S::new(q.prec, &q.spec.mz, &q.spec.it);
        let ok = vote(&qs, &ix.specs[at[&id]]).is_some_and(|(x, y)| {
            let near = |u: f64, v: f64| (u - v).abs() <= FRAG_DA;
            (near(x, *a) && near(y, *b)) || (near(x, *b) && near(y, *a))
        });
        bad += !ok as usize;
    }
    json!((bad as f64 / n.max(1) as f64 * 1e4).round() / 1e4 + 0.0)
}

/// Share of the queries with a target at the top and q <= 0.01 (on E0 every analogue is a false one).
fn hit_q01(rows: &[Row]) -> Value {
    let k = rows
        .iter()
        .filter(|r| !r.ranked.is_empty() && r.q <= 0.01)
        .count();
    json!((k as f64 / rows.len().max(1) as f64 * 1e4).round() / 1e4 + 0.0)
}

/// At top scores >= 0.7, 0.8 and 0.9: the share of queries with a right top analogue (`recall1_sNN`, over those with a truth) and with any
/// (`hit_sNN`; on E0 every one is false).
fn by_score(v: &mut Value, rows: &[Row], qs: &[super::scen::Q]) {
    let np = qs.iter().filter(|q| q.truth.is_some()).count().max(1) as f64;
    let r4 = |x: f64| json!((x * 1e4).round() / 1e4 + 0.0);
    for t in [70u8, 80, 90] {
        let top = |r: &Row| {
            r.ranked
                .first()
                .filter(|x| x.1 as f64 >= t as f64 / 100.0)
                .map(|x| x.0)
        };
        let hit =
            rows.iter().filter(|r| top(r).is_some()).count() as f64 / rows.len().max(1) as f64;
        let right = rows
            .iter()
            .zip(qs)
            .filter(|(r, q)| q.truth.is_some() && top(r) == q.truth)
            .count() as f64
            / np;
        v["all"][format!("hit_s{t}")] = r4(hit);
        v["all"][format!("recall1_s{t}")] = r4(right);
    }
}

/// One JSON object per arm on scenario `name`: the engine's analogue search, then `ARMS`; `nl` carries the edge-channel size (distinct bins per
/// target and its peaks), `sankoff` on E5 the wrong-δ share.
pub fn run_shift(name: &str, seed: u64, n_lib: usize, n: usize) -> Vec<Value> {
    let st = super::Setup::new(name, seed, n_lib, n);
    let l = arms::open(&st.bytes);
    let qs = st.queries();
    let ix = Index::new(&l);
    let base = arms::baseline("analogs", &l, &qs);
    let mut v = super::tag(&base, &st.s, "analogs", name, seed);
    v["all"]["hit_q01"] = hit_q01(&base);
    by_score(&mut v, &base, &st.s.queries);
    let mut out = vec![v];
    let mut acc = FlashAcc::new();
    for arm in ARMS {
        let mut rows: Vec<Row> = qs
            .iter()
            .map(|q| {
                q.as_ref()
                    .map_or(Row::default(), |q| row(arm, &l, &ix, q, &mut acc))
            })
            .collect();
        arms::add_q(&mut rows);
        let mut v = super::tag(&rows, &st.s, arm, name, seed);
        v["all"]["hit_q01"] = hit_q01(&rows);
        by_score(&mut v, &rows, &st.s.queries);
        if arm == "nl" {
            let t: Vec<usize> = (0..l.ids.len()).filter(|&i| l.ids[i].is_some()).collect();
            let mean = |f: &dyn Fn(usize) -> usize| {
                t.iter().map(|&i| f(i)).sum::<usize>() as f64 / t.len().max(1) as f64
            };
            v["all"]["index_keys"] = json!((mean(&|i| ix.keys[i]) * 10.0).round() / 10.0);
            v["all"]["index_peaks"] = json!((mean(&|i| ix.specs[i].p.len()) * 10.0).round() / 10.0);
        }
        if arm == "sankoff" && name == "e5" {
            v["all"]["delta_wrong"] = delta_wrong(&st, &l, &ix, &qs);
        }
        out.push(v);
    }
    out
}
