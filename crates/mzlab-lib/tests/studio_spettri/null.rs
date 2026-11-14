//! Arm A of the spectral study, the calibrated null: the entropy ranking of the engine with other decoys and the +1-corrected q. Every decoy kind
//! builds its own `.mzlib` (targets followed by one decoy each, `DECOY:` accession as `decoy::make_decoy` writes it), so the search and the counts
//! are the engine's; only the decoy peaks differ. The kinds are test code: `decoy::DecoyKind` stays as it is.

use super::arms::{self, identity_row, spectrum, Lib, Row};
use super::r#gen::{ion_mz, valid_ion, F, R};
use super::scen::Scen;
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::decoy::{decoy_seed, lib_fdr, make_decoy, make_decoys, DecoyKind, DEFAULT_SEED};
use mzlab_lib::search::Query;
use mzlab_lib::spec::Spectrum;
use serde_json::Value;

/// `fragments` (today's default), `precursor` (the other engine kind), `swap_near` (peaks of the nearest entry by precursor, which may be an
/// isomer inside the search window), `swap_out` (nearest entry more than 20 ppm away), `subformula` (random plausible sub-formulas of the root),
/// `sead` (the consecutive neutral losses of the spectrum in another order).
pub const KINDS: [&str; 6] = [
    "fragments",
    "precursor",
    "swap_near",
    "swap_out",
    "subformula",
    "sead",
];

/// Peaks of a decoy closer than this to a target peak count as landing on it (the fragment tolerance).
const NEAR: f64 = arms::FRAG_DA;

fn below(p: &[(f64, f64)], cut: f64) -> Vec<(f64, f64)> {
    p.iter().copied().filter(|x| x.0 < cut).collect()
}

/// The decoy of `t` carrying the peaks `p` (metadata, accession and precursor as `make_decoy` writes them).
fn with_peaks(t: &Spectrum, p: Vec<(f64, f64)>) -> Spectrum {
    let mut d = make_decoy(t, DecoyKind::Fragments, decoy_seed(t, DEFAULT_SEED));
    (d.mz, d.it) = p.into_iter().unzip();
    d
}

/// Peaks of another entry placed at the precursor of `t`: the nearest entry by precursor (`min_ppm` 0) or the nearest farther than `min_ppm`.
/// Donor peaks at or above either precursor - 1.6 Da are dropped (the cleaning would drop them); a donor with no peak left is skipped.
fn swap(ts: &[Spectrum], order: &[usize], pos: usize, min_ppm: f64) -> Spectrum {
    let t = &ts[order[pos]];
    let far = |j: usize| (ts[j].prec - t.prec).abs() > min_ppm * 1e-6 * t.prec;
    let (mut lo, mut hi) = (pos, pos + 1);
    loop {
        // the closer of the two neighbours first; ties go down
        let down = lo > 0
            && (hi >= order.len()
                || t.prec - ts[order[lo - 1]].prec <= ts[order[hi]].prec - t.prec);
        let j = if down {
            lo -= 1;
            order[lo]
        } else if hi < order.len() {
            hi += 1;
            order[hi - 1]
        } else {
            return make_decoy(t, DecoyKind::Fragments, decoy_seed(t, DEFAULT_SEED));
        };
        let d = &ts[j];
        let p: Vec<(f64, f64)> = d.mz.iter().copied().zip(d.it.iter().copied()).collect();
        let p = below(&below(&p, d.prec - 1.6), t.prec - 1.6);
        if far(j) && !p.is_empty() {
            return with_peaks(t, p);
        }
    }
}

/// As many random sub-formulas of the root ion as the target has peaks: every element count uniform in 0..=n, a valid ion (C, H >= 1, RDBE >=
/// -0.5) between 50 Da and the precursor - 2, never on a target peak nor twice; after 64 draws a peak takes a uniform m/z, as `Fragments` does.
fn subformula(t: &Spectrum, root: &F) -> Spectrum {
    let mut r = R::new(decoy_seed(t, DEFAULT_SEED), 0xA);
    let top = t.prec - 2.0;
    let mut mz: Vec<f64> = Vec::new();
    for _ in &t.mz {
        let mut m = r.range(50.0, top.max(51.0));
        for _ in 0..64 {
            let f: F = std::array::from_fn(|e| r.int(0, root[e]));
            let x = ion_mz(&f);
            let free = |v: &[f64]| v.iter().all(|y| (y - x).abs() > NEAR);
            if valid_ion(&f) && (50.0..top).contains(&x) && free(&t.mz) && free(&mz) {
                m = x;
                break;
            }
        }
        mz.push(m);
    }
    with_peaks(t, mz.into_iter().zip(t.it.iter().copied()).collect())
}

/// Neutral-loss-order swap: peaks from the highest m/z down give consecutive losses from the precursor; they are shuffled (Fisher-Yates) and
/// summed again, each rank keeping its intensity. The total loss stays, so the lowest fragment stays where it was; of 16 shuffles the one with
/// the fewest other peaks on a target peak is kept.
fn sead(t: &Spectrum) -> Spectrum {
    let mut p: Vec<(f64, f64)> = t.mz.iter().copied().zip(t.it.iter().copied()).collect();
    p = below(&p, t.prec - 1.6);
    p.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut prev = t.prec;
    let loss: Vec<f64> = p
        .iter()
        .map(|x| std::mem::replace(&mut prev, x.0) - x.0)
        .collect();
    let mut r = R::new(decoy_seed(t, DEFAULT_SEED), 0xB);
    let mut best: Option<(usize, Vec<f64>)> = None;
    for _ in 0..16 {
        let mut l = loss.clone();
        for i in (1..l.len()).rev() {
            l.swap(i, (r.0.next_u64() % (i as u64 + 1)) as usize);
        }
        let mut mz = Vec::with_capacity(l.len());
        let mut m = t.prec;
        for d in &l {
            m -= d;
            mz.push(m);
        }
        let hits = mz[..mz.len().saturating_sub(1)]
            .iter()
            .filter(|x| p.iter().any(|y| (y.0 - **x).abs() <= NEAR))
            .count();
        if best.as_ref().is_none_or(|b| hits < b.0) {
            best = Some((hits, mz));
        }
    }
    let mz = best.map_or(Vec::new(), |b| b.1);
    with_peaks(t, mz.into_iter().zip(p.iter().map(|x| x.1)).collect())
}

/// The `.mzlib` of `s` with the decoys of `kind`.
pub fn lib_bytes(s: &Scen, kind: &str) -> Vec<u8> {
    let ts: Vec<Spectrum> = (0..s.lib.len()).map(|i| spectrum(i, &s.lib[i])).collect();
    let mut order: Vec<usize> = (0..ts.len()).collect();
    order.sort_by(|&a, &b| ts[a].prec.total_cmp(&ts[b].prec).then(a.cmp(&b)));
    let mut at = vec![0; ts.len()];
    for (p, &i) in order.iter().enumerate() {
        at[i] = p;
    }
    let decoys: Vec<Spectrum> = match kind {
        "fragments" => make_decoys(&ts, DecoyKind::Fragments, DEFAULT_SEED),
        "precursor" => make_decoys(&ts, DecoyKind::Precursor, DEFAULT_SEED),
        "swap_near" => (0..ts.len())
            .map(|i| swap(&ts, &order, at[i], 0.0))
            .collect(),
        "swap_out" => (0..ts.len())
            .map(|i| swap(&ts, &order, at[i], 20.0))
            .collect(),
        "subformula" => (0..ts.len())
            .map(|i| subformula(&ts[i], &s.lib[i].tree.nodes[0].f))
            .collect(),
        "sead" => ts.iter().map(sead).collect(),
        _ => panic!("unknown decoy kind {kind}"),
    };
    let mut all = ts;
    all.extend(decoys);
    build_in_memory(&all, BuildOpts::default()).unwrap().0
}

/// q with the +1 correction: (1 + #decoy >= s) / max(1, #target >= s), capped at 1, then the running minimum from the lowest score as in
/// `decoy::lib_fdr`. A query without a target gets q = 1.
pub fn q_plus1(t: &[f32], d: &[f32]) -> Vec<f32> {
    let mut ts: Vec<f32> = t.iter().copied().filter(|x| x.is_finite()).collect();
    let mut ds: Vec<f32> = d.iter().copied().filter(|x| x.is_finite()).collect();
    ts.sort_by(|a, b| a.total_cmp(b));
    ds.sort_by(|a, b| a.total_cmp(b));
    let mut q_of: Vec<(f32, f32)> = Vec::new();
    let mut run = 1.0f32;
    let mut i = 0;
    while i < ts.len() {
        let s = ts[i];
        let nt = ts.len() - i;
        let nd = ds.len() - ds.partition_point(|x| *x < s);
        run = run.min(((1 + nd) as f32 / nt.max(1) as f32).min(1.0));
        q_of.push((s, run));
        while i < ts.len() && ts[i] == s {
            i += 1;
        }
    }
    t.iter()
        .map(|&s| {
            let k = q_of.partition_point(|x| x.0 < s);
            if s.is_finite() {
                q_of.get(k).map_or(1.0, |x| x.1)
            } else {
                1.0
            }
        })
        .collect()
}

/// Entropy rows of the queries on a library, without q.
pub fn entropy_rows(l: &Lib, qs: &[Option<Query>]) -> Vec<Row> {
    qs.iter()
        .map(|q| {
            q.as_ref().map_or(Row::default(), |q| {
                identity_row(l, q, &|_, h| (h.entropy, h.shared))
            })
        })
        .collect()
}

/// The rows with q standard (`decoy::lib_fdr`) and with q +1, in that order.
pub fn with_q(rows: &[Row]) -> [Vec<Row>; 2] {
    let t: Vec<f32> = rows
        .iter()
        .map(|r| r.ranked.first().map_or(f32::NAN, |x| x.1))
        .collect();
    let d: Vec<f32> = rows.iter().map(|r| r.decoy).collect();
    let mut a = rows.to_vec();
    for (r, f) in a.iter_mut().zip(lib_fdr(&t, &d, DecoyKind::Fragments)) {
        r.q = f.q_value;
    }
    let mut b = rows.to_vec();
    for (r, q) in b.iter_mut().zip(q_plus1(&t, &d)) {
        r.q = q;
    }
    [a, b]
}

/// One JSON object per arm `<kind>` and `<kind>+1` on scenario `name`.
pub fn run_null(name: &str, seed: u64, n_lib: usize, n: usize) -> Vec<Value> {
    let st = super::Setup::new(name, seed, n_lib, n);
    let qs = st.queries();
    let mut out = Vec::new();
    for kind in KINDS {
        let bytes = lib_bytes(&st.s, kind);
        let l = arms::open(&bytes);
        let [a, b] = with_q(&entropy_rows(&l, &qs));
        out.push(super::tag(&a, &st.s, kind, name, seed));
        out.push(super::tag(&b, &st.s, &format!("{kind}+1"), name, seed));
    }
    out
}
