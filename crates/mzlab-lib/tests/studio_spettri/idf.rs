//! Arm B of the spectral study: coincidence p-value and information weights on the identity window of the engine. A background set B of target
//! entries around the query precursor gives, per query peak f, N(f) = entries of B with a peak within the fragment tolerance; the scores use the
//! chance of a coincidence (N + 0.5) / (|B| + 1). Test code only: the engine's scores stay as they are.

use super::arms::{self, identity_row, Lib, Row, FRAG_DA, PREC_PPM};
use super::r#gen::R;
use mzlab_lib::scores::entropy_from_pairs;
use mzlab_lib::search::{Hit, Query};
use serde_json::{json, Value};

/// `binom`: -log10 of the binomial tail P(X >= k) of the k matched peaks among the n of the query, each peak with the mean chance of B without
/// the candidate; `idf`: entropy with every intensity (query and candidate) times I = -log2((N + 0.5) / (|B| + 1)); `surp`: the sum of
/// -log2(chance) over the matched query peaks. `win`: B is the 10 ppm identity window; `wide`: the targets within 10 Da.
pub const ARMS: [&str; 6] = [
    "binom_win",
    "binom_wide",
    "idf_win",
    "idf_wide",
    "surp_win",
    "surp_wide",
];
/// Half width of the `wide` background, in Da.
pub const WIDE_DA: f64 = 10.0;

/// The background: targets of the library by precursor, with their stored (cleaned) peaks.
pub struct Bg {
    prec: Vec<f64>,
    mz: Vec<Vec<f64>>,
    ids: Vec<usize>,
}

/// The entries kept by the 50 % subsample of the background: one fixed bit per entry id.
pub fn half(id: usize) -> bool {
    R::new(0x5eed, id as u64).0.next_u64() & 1 == 0
}

impl Bg {
    pub fn new(l: &Lib, keep: &dyn Fn(usize) -> bool) -> Bg {
        let mut v: Vec<(f64, Vec<f64>, usize)> = Vec::new();
        for i in 0..l.lib.n_spectra() {
            if let Some(id) = l.ids[i as usize].filter(|&id| keep(id)) {
                v.push((l.lib.rec(i).unwrap().prec, l.lib.peaks(i).unwrap().0, id));
            }
        }
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)));
        Bg {
            prec: v.iter().map(|x| x.0).collect(),
            mz: v.iter().map(|x| x.1.clone()).collect(),
            ids: v.iter().map(|x| x.2).collect(),
        }
    }

    /// Positions of B for a query precursor (`wide` or the identity window).
    fn window(&self, prec: f64, wide: bool) -> std::ops::Range<usize> {
        let w = if wide {
            WIDE_DA
        } else {
            PREC_PPM * 1e-6 * prec
        };
        self.prec.partition_point(|p| *p < prec - w)..self.prec.partition_point(|p| *p <= prec + w)
    }
}

fn has(mz: &[f64], m: f64) -> bool {
    let k = mz.partition_point(|x| *x < m - FRAG_DA);
    k < mz.len() && mz[k] <= m + FRAG_DA
}

/// B of one query: its size, the entry ids in it and, for any m/z, N(m).
struct Set<'a> {
    bg: &'a Bg,
    r: std::ops::Range<usize>,
}

impl Set<'_> {
    fn size(&self) -> usize {
        self.r.len()
    }
    fn n(&self, m: f64) -> usize {
        self.r.clone().filter(|&k| has(&self.bg.mz[k], m)).count()
    }
    fn holds(&self, id: usize) -> bool {
        self.r.clone().any(|k| self.bg.ids[k] == id)
    }
}

/// -log2 of the smoothed chance (N + 0.5) / (|B| + 1).
fn info(n: usize, size: usize) -> f64 {
    -libm::log2((n as f64 + 0.5) / (size as f64 + 1.0))
}

/// -log10 P(X >= k), X ~ Binomial(n, p), summed in log space.
fn binom_tail(n: usize, k: usize, p: f64) -> f64 {
    let p = p.clamp(1e-12, 1.0 - 1e-12);
    let lgam = |x: usize| libm::lgamma(x as f64 + 1.0);
    let terms: Vec<f64> = (k..=n)
        .map(|j| {
            lgam(n) - lgam(j) - lgam(n - j)
                + j as f64 * libm::log(p)
                + (n - j) as f64 * libm::log(1.0 - p)
        })
        .collect();
    let top = terms.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if terms.is_empty() {
        return 0.0;
    }
    let s: f64 = terms.iter().map(|t| libm::exp(t - top)).sum();
    -(top + libm::log(s)) / std::f64::consts::LN_10
}

/// Score of one candidate `h` (entry `id`, `None` for a decoy) under `arm`; `nq` holds N of every query peak.
fn score(arm: &str, set: &Set, nq: &[usize], q: &Query, h: &Hit, id: Option<usize>) -> f64 {
    let size = set.size();
    // the candidate's own peaks are not a coincidence: leave it out of B for the chance
    let own = id.is_some_and(|id| set.holds(id));
    let chance = |i: usize| {
        let mine = own && has(&h.spec.mz, q.spec.mz[i]);
        let n = nq[i] - mine as usize;
        (n as f64 + 0.5) / ((size - own as usize) as f64 + 1.0)
    };
    match &arm[..4] {
        "bino" => {
            let p = (0..nq.len()).map(chance).sum::<f64>() / nq.len().max(1) as f64;
            binom_tail(nq.len(), h.pairs.len(), p)
        }
        "surp" => h.pairs.iter().map(|&(i, _)| -libm::log2(chance(i))).sum(),
        _ => {
            let a: Vec<f64> = q
                .spec
                .it
                .iter()
                .zip(nq)
                .map(|(v, &n)| v * info(n, size))
                .collect();
            let b: Vec<f64> = h
                .spec
                .mz
                .iter()
                .zip(&h.spec.it)
                .map(|(&m, v)| v * info(set.n(m), size))
                .collect();
            entropy_from_pairs(&a, &b, &h.pairs)
        }
    }
}

/// The rows of `arm` over the queries with background `bg` (q of `decoy::lib_fdr` within the batch, as the baselines).
pub fn rows(arm: &str, l: &Lib, bg: &Bg, qs: &[Option<Query>]) -> Vec<Row> {
    let mut out: Vec<Row> = qs
        .iter()
        .map(|q| {
            let Some(q) = q else { return Row::default() };
            let set = Set {
                bg,
                r: bg.window(q.prec, arm.ends_with("wide")),
            };
            let nq: Vec<usize> = q.spec.mz.iter().map(|&m| set.n(m)).collect();
            identity_row(l, q, &|q, h| {
                let id = l.ids[h.index as usize];
                (score(arm, &set, &nq, q, h, id), h.shared)
            })
        })
        .collect();
    arms::add_q(&mut out);
    out
}

/// Share of queries whose top entry differs between two sets of rows.
fn top_changed(a: &[Row], b: &[Row]) -> f64 {
    let top = |r: &Row| r.ranked.first().map(|x| x.0);
    a.iter().zip(b).filter(|(x, y)| top(x) != top(y)).count() as f64 / a.len().max(1) as f64
}

/// One JSON object per arm on scenario `name`: the entropy baseline, each arm of `ARMS` with the full background and with the 50 % one
/// (`<arm>@50`, with `top_changed` against the full background).
pub fn run_idf(name: &str, seed: u64, n_lib: usize, n: usize) -> Vec<Value> {
    let st = super::Setup::new(name, seed, n_lib, n);
    let l = arms::open(&st.bytes);
    let qs = st.queries();
    let (full, part) = (Bg::new(&l, &|_| true), Bg::new(&l, &half));
    let mut out = vec![super::tag(
        &arms::baseline("entropy", &l, &qs),
        &st.s,
        "entropy",
        name,
        seed,
    )];
    for arm in ARMS {
        let a = rows(arm, &l, &full, &qs);
        let b = rows(arm, &l, &part, &qs);
        out.push(super::tag(&a, &st.s, arm, name, seed));
        let mut v = super::tag(&b, &st.s, &format!("{arm}@50"), name, seed);
        v["all"]["top_changed"] = json!((top_changed(&a, &b) * 1e4).round() / 1e4 + 0.0);
        out.push(v);
    }
    out
}
