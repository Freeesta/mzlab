//! Arm C of the spectral study, sub-formula validation as de-chimerization: before the entropy against a candidate, the query keeps only the
//! peaks that are sub-formulas of the candidate's ion (`mzlab_core::subformulas::submasses`, 5 or 10 ppm); the NNLS of `spectra.chimeric`
//! (`ms2_engine/deconv.rs`) is the reference. The candidates are those of the engine's identity window. Test code only.

use super::arms::{self, identity_row, Lib, Row, FRAG_DA};
use super::deconv::{deconvolve, Candidate};
use super::r#gen::{add, text, Peaks, H1};
use super::scen::Scen;
use mzlab_core::subformulas::submasses;
use mzlab_lib::frag_tol::FragTol;
use mzlab_lib::scores::{pair_scores, Spec};
use mzlab_lib::search::Query;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// `sub5` / `sub10`: entropy of the filtered query; `sub5x` / `sub10x`: the same times the share of the query intensity the filter keeps;
/// `nnls`: the share of the query intensity `deconvolve` gives the candidate among the targets within `ISOLATION` Da and the window's decoys.
pub const ARMS: [&str; 5] = ["sub5", "sub10", "sub5x", "sub10x", "nnls"];
/// Half width of the isolation window of the NNLS reference, in Da (the isobaric interferents are 0.02-0.5 Da away).
pub const ISOLATION: f64 = 1.0;

/// The query of one candidate after the filter, and the share of the fragment intensity it keeps (`skip`: the formula has more sub-formulas
/// than `subformulas::MAX_COMBINATIONS`, nothing is filtered).
pub struct Filtered {
    pub query: Option<Query>,
    pub kept: f64,
    pub keep: Vec<bool>,
    pub skip: bool,
}

thread_local! {
    // the arms and the peak report ask for the same ions again: the enumeration runs once per ion
    static SUB: RefCell<HashMap<String, Option<Rc<Vec<f64>>>>> = RefCell::new(HashMap::new());
}

/// Sorted sub-formula m/z of `ion` (`None` past `subformulas::MAX_COMBINATIONS`).
fn sub_of(ion: &str) -> Option<Rc<Vec<f64>>> {
    SUB.with(|c| {
        c.borrow_mut()
            .entry(ion.to_string())
            .or_insert_with(|| submasses(ion, 1, 0.0).ok().map(Rc::new))
            .clone()
    })
}

/// Keeps the fragments of `p` (below the precursor - 1.6 Da, as the cleaning) within `ppm` of a sub-formula of the ion `ion`.
pub fn filter(p: &Peaks, ion: &str, ppm: f64) -> Filtered {
    let frag = |i: usize| p.mz[i] < p.prec - 1.6;
    let sub = sub_of(ion);
    let keep: Vec<bool> = (0..p.mz.len())
        .map(|i| {
            let Some(s) = &sub else { return true };
            let (m, w) = (p.mz[i], ppm * 1e-6 * p.mz[i]);
            let k = s.partition_point(|x| *x < m - w);
            frag(i) && k < s.len() && s[k] <= m + w
        })
        .collect();
    let it = |sel: &dyn Fn(usize) -> bool| {
        (0..p.mz.len())
            .filter(|&i| frag(i) && sel(i))
            .map(|i| p.it[i])
            .sum::<f64>()
    };
    let pick = |v: &[f64]| {
        (0..v.len())
            .filter(|&i| keep[i])
            .map(|i| v[i])
            .collect::<Vec<f64>>()
    };
    Filtered {
        query: Query::new(p.prec, 1, &pick(&p.mz), &pick(&p.it)),
        kept: it(&|i| keep[i]) / it(&|_| true).max(1e-300),
        keep,
        skip: sub.is_none(),
    }
}

/// Ion formula (Hill) of library entry `id`.
fn ion_of(s: &Scen, id: usize) -> String {
    text(&add(&s.lib[id].neutral, &H1))
}

/// The entry a candidate stands for: itself, or the target a decoy was made from (its accession is `DECOY:SYN-<id>`).
fn entry_of(l: &Lib, index: u32) -> usize {
    l.ids[index as usize].unwrap_or_else(|| {
        let a = l.lib.meta(index).unwrap().accession;
        a.trim_start_matches("DECOY:SYN-").parse().unwrap()
    })
}

/// The rows of a filter arm: every candidate is scored against the query filtered by its own formula.
fn filter_rows(arm: &str, st: &super::Setup, l: &Lib, qs: &[Option<Query>]) -> Vec<Row> {
    let ppm = if arm.starts_with("sub5") { 5.0 } else { 10.0 };
    let frag = FragTol::from(FRAG_DA);
    let mut out: Vec<Row> = qs
        .iter()
        .zip(&st.s.queries)
        .map(|(q, sq)| {
            let Some(q) = q else { return Row::default() };
            // one filter per entry: a decoy shares it with its target
            let seen: RefCell<HashMap<usize, Filtered>> = RefCell::new(HashMap::new());
            identity_row(l, q, &|_, h| {
                let id = entry_of(l, h.index);
                let mut seen = seen.borrow_mut();
                let f = seen
                    .entry(id)
                    .or_insert_with(|| filter(&sq.peaks, &ion_of(&st.s, id), ppm));
                let Some(fq) = &f.query else { return (0.0, 0) };
                let p = pair_scores(&fq.spec, &h.spec, frag);
                let w = if arm.ends_with('x') { f.kept } else { 1.0 };
                (p.entropy * w, p.shared)
            })
        })
        .collect();
    arms::add_q(&mut out);
    out
}

/// The rows of the NNLS reference.
fn nnls_rows(l: &Lib, qs: &[Option<Query>]) -> Vec<Row> {
    let n = l.lib.n_spectra();
    let all: Vec<(f64, Spec)> = (0..n)
        .map(|i| {
            let (mz, it) = l.lib.peaks(i).unwrap();
            let prec = l.lib.rec(i).unwrap().prec;
            (prec, Spec { mz, it, prec })
        })
        .collect();
    let mut out: Vec<Row> = qs
        .iter()
        .map(|q| {
            let Some(q) = q else { return Row::default() };
            let win = |i: usize| {
                (all[i].0 - q.prec).abs() <= ISOLATION
                    && (l.ids[i].is_some() || (all[i].0 - q.prec).abs() <= 1e-5 * q.prec)
            };
            let cols: Vec<usize> = (0..n as usize).filter(|&i| win(i)).collect();
            let cands: Vec<Candidate> = cols
                .iter()
                .map(|&i| Candidate {
                    mz: &all[i].1.mz,
                    it: &all[i].1.it,
                })
                .collect();
            let d = deconvolve(&q.spec.mz, &q.spec.it, &cands, FRAG_DA);
            let share: HashMap<u32, f64> = d.map_or(HashMap::new(), |d| {
                cols.iter().map(|&i| i as u32).zip(d.shares).collect()
            });
            identity_row(l, q, &|_, h| {
                (share.get(&h.index).copied().unwrap_or(0.0), h.shared)
            })
        })
        .collect();
    arms::add_q(&mut out);
    out
}

/// Ground truth of the filter on the true entry: share of the interferent, compound and noise fragments removed, and the queries skipped.
fn peaks_report(st: &super::Setup, ppm: f64, idx: &[usize]) -> Value {
    let (mut n, mut gone, mut skip, mut nq) = ([0usize; 3], [0usize; 3], 0usize, 0usize);
    for &k in idx {
        let q = &st.s.queries[k];
        let Some(id) = q.truth else { continue };
        let f = filter(&q.peaks, &ion_of(&st.s, id), ppm);
        nq += 1;
        skip += f.skip as usize;
        for i in (0..q.peaks.mz.len()).filter(|&i| q.peaks.mz[i] < q.peaks.prec - 1.6) {
            let s = q.peaks.src[i] as usize;
            n[s] += 1;
            gone[s] += !f.keep[i] as usize;
        }
    }
    let share = |a: usize, b: usize| json!((a as f64 / b.max(1) as f64 * 1e4).round() / 1e4 + 0.0);
    json!({
        "true_lost": share(gone[0], n[0]), "int_removed": share(gone[1], n[1]), "noise_removed": share(gone[2], n[2]),
        "skip": share(skip, nq),
    })
}

/// Adds `peaks_report` to `all` and to every group of a filter arm's report.
fn with_peaks(mut v: Value, st: &super::Setup, ppm: f64) -> Value {
    let qs = &st.s.queries;
    let all: Vec<usize> = (0..qs.len()).collect();
    v["all"]["peaks"] = peaks_report(st, ppm, &all);
    let names: Vec<String> = v["groups"].as_object().unwrap().keys().cloned().collect();
    for g in names {
        let idx: Vec<usize> = (0..qs.len())
            .filter(|&i| qs[i].group == g || qs[i].group.split_once('/').is_some_and(|x| x.0 == g))
            .collect();
        v["groups"][&g]["peaks"] = peaks_report(st, ppm, &idx);
    }
    v
}

/// One JSON object per arm on scenario `name`: the entropy baseline, the four filter arms (with the peak ground truth on `sub5` and `sub10`)
/// and the NNLS reference.
pub fn run_subf(name: &str, seed: u64, n_lib: usize, n: usize) -> Vec<Value> {
    let st = super::Setup::new(name, seed, n_lib, n);
    let l = arms::open(&st.bytes);
    let qs = st.queries();
    let mut out = vec![super::tag(
        &arms::baseline("entropy", &l, &qs),
        &st.s,
        "entropy",
        name,
        seed,
    )];
    for arm in ARMS {
        let rows = if arm == "nnls" {
            nnls_rows(&l, &qs)
        } else {
            filter_rows(arm, &st, &l, &qs)
        };
        let v = super::tag(&rows, &st.s, arm, name, seed);
        out.push(match arm {
            "sub5" => with_peaks(v, &st, 5.0),
            "sub10" => with_peaks(v, &st, 10.0),
            _ => v,
        });
    }
    out
}
