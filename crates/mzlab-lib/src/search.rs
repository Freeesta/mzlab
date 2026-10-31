//! Searching a library: identity (precursor window, every candidate scored on its peaks) and Flash-style open / neutral-loss / hybrid searches
//! over the fragment and neutral-loss indexes (Li & Fiehn, Nat. Methods 2023), used for the analogues.

use crate::clean::{clean, CleanOpts};
use crate::error::Result;
pub use crate::flash::{flash_scores, flash_scores_with, FlashMode, FlashScore};
use crate::flash::{flash_top, FlashAcc};
use crate::frag_tol::FragTol;
use crate::io::Source;
use crate::prep::Scorer;
use crate::read::Library;
use crate::scores::{
    common_losses_with, modified_cosine, pair_scores, MatchScratch, ModCosine, Spec,
};
use mzlab_picchi::sortc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tol {
    Ppm(f64),
    Da(f64),
}

impl Tol {
    pub fn window(&self, at: f64) -> f64 {
        match *self {
            Tol::Ppm(p) => at * p * 1e-6,
            Tol::Da(d) => d,
        }
    }
}

/// A query spectrum cleaned like the library spectra (precursor region, noise, centroiding), intensities with maximum 1.
#[derive(Debug, Clone)]
pub struct Query {
    pub prec: f64,
    pub pol: i8,
    pub spec: Spec,
}

impl Query {
    /// `None` when nothing is left after the cleaning (or there is no peak).
    pub fn new(prec: f64, pol: i8, mz: &[f64], it: &[f64]) -> Option<Query> {
        let (m, i) = clean(prec, mz, it, &CleanOpts::default());
        let mx = i.iter().cloned().fold(0.0, f64::max);
        if m.is_empty() || mx <= 0.0 {
            return None;
        }
        Some(Query {
            prec,
            pol,
            spec: Spec {
                mz: m,
                it: i.iter().map(|v| v / mx).collect(),
                prec,
            },
        })
    }
}

fn spec_of<S: Source>(lib: &Library<S>, i: u32, prec: f64) -> Result<Spec> {
    let (mz, it) = lib.peaks(i)?;
    Ok(Spec { mz, it, prec })
}

#[derive(Debug, Clone)]
pub struct IdentityOpts {
    pub prec_tol: Tol,
    pub frag_da: f64,
    pub pol: i8,
    pub top: usize,
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub index: u32,
    pub prec: f64,
    pub pol: i8,
    pub licence: u8,
    pub entropy: f64,
    pub cosine: f64,
    pub forward: f64,
    pub reverse: f64,
    pub shared: usize,
    pub explained: f64,
    pub nl_shared: usize,
    /// Library precursor minus query precursor: in ppm (of the query) for `Tol::Ppm`, in Da for `Tol::Da`.
    pub dprec: f64,
    pub pairs: Vec<(usize, usize)>,
    pub spec: Spec,
}

/// Every library spectrum with the query's polarity (or none known) and a precursor within the tolerance, best entropy first.
pub fn identity<S: Source>(lib: &Library<S>, q: &Query, o: &IdentityOpts) -> Result<Vec<Hit>> {
    identity_with(lib, q, o, o.frag_da)
}

/// `identity` with the fragment tolerance given as `FragTol` (ppm of the query peak or Da); `o.frag_da` is not read.
/// `dprec` and the neutral-loss count use the same tolerance (losses: the window of the precursor m/z).
pub fn identity_with<S: Source>(
    lib: &Library<S>,
    q: &Query,
    o: &IdentityOpts,
    frag: impl Into<FragTol>,
) -> Result<Vec<Hit>> {
    let frag = frag.into();
    let w = o.prec_tol.window(q.prec);
    let top = o.top.max(1);
    // Scores only: spectra and pairs are loaded for the winners, so a wide window over a big library stays small.
    let mut scorer = Scorer::new(&q.spec);
    let mut light: Vec<Hit> = Vec::new();
    let mut s = Spec::default();
    let order = |a: &Hit, b: &Hit| {
        b.entropy
            .total_cmp(&a.entropy)
            .then(b.cosine.total_cmp(&a.cosine))
            .then(a.index.cmp(&b.index))
    };
    for r in lib.ranges(q.pol, q.prec - w, q.prec + w)? {
        for i in r {
            let rec = lib.rec(i)?;
            lib.peaks_into(i, &mut s.mz, &mut s.it)?;
            s.prec = rec.prec;
            let sc = scorer.score(&q.spec, &s, frag);
            let d = rec.prec - q.prec;
            light.push(Hit {
                index: i,
                prec: rec.prec,
                pol: rec.pol,
                licence: rec.licence,
                entropy: sc.entropy,
                cosine: sc.cosine,
                forward: sc.forward,
                reverse: sc.reverse,
                shared: sc.shared,
                explained: sc.explained,
                nl_shared: 0,
                dprec: if matches!(o.prec_tol, Tol::Ppm(_)) {
                    d / q.prec * 1e6
                } else {
                    d
                },
                pairs: Vec::new(),
                spec: Spec::default(),
            });
            // amortised pruning keeps at most 2 * top candidates alive
            if light.len() >= top.saturating_mul(2).saturating_add(64) {
                sortc::any_by(&mut light, order);
                light.truncate(top);
            }
        }
    }
    sortc::any_by(&mut light, order);
    light.truncate(top);
    let mut scratch = MatchScratch::default();
    for h in light.iter_mut() {
        h.spec = spec_of(lib, h.index, h.prec)?;
        h.pairs = pair_scores(&q.spec, &h.spec, frag).pairs;
        h.nl_shared = common_losses_with(&q.spec, &h.spec, frag.at(q.prec), &mut scratch);
    }
    Ok(light)
}

#[derive(Debug, Clone)]
pub struct AnalogOpts {
    pub frag_da: f64,
    /// When > 0 the fragment window is this many ppm of each query peak and `frag_da` is ignored.
    pub frag_ppm: f64,
    pub pol: i8,
    pub max_delta: f64,
    pub min_delta: f64,
    /// Candidates kept after the Flash hybrid search (they get the exact modified cosine).
    pub candidates: usize,
    pub min_hybrid_pairs: u32,
    pub min_matched: usize,
    pub top: usize,
}

impl Default for AnalogOpts {
    fn default() -> Self {
        AnalogOpts {
            frag_da: 0.01,
            frag_ppm: 0.0,
            pol: 0,
            max_delta: 200.0,
            min_delta: 0.5,
            candidates: 300,
            min_hybrid_pairs: 3,
            min_matched: 4,
            top: 10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnalogHit {
    pub index: u32,
    pub prec: f64,
    pub pol: i8,
    pub licence: u8,
    pub hybrid: f64,
    pub coinc: u32,
    pub mc: ModCosine,
    pub spec: Spec,
}

/// Analogues: Flash hybrid entropy search in the precursor window picks the candidates, the modified cosine (≥ `min_matched` peaks) ranks them.
pub fn analogs<S: Source>(lib: &Library<S>, q: &Query, o: &AnalogOpts) -> Result<Vec<AnalogHit>> {
    analogs_with(lib, q, o, &mut FlashAcc::new())
}

/// `analogs` with the accumulators of the caller (shared by the libraries of a set and across queries).
pub fn analogs_with<S: Source>(
    lib: &Library<S>,
    q: &Query,
    o: &AnalogOpts,
    acc: &mut FlashAcc,
) -> Result<Vec<AnalogHit>> {
    let pol = if o.pol != 0 { o.pol } else { q.pol };
    let ranges = lib.ranges(pol, q.prec - o.max_delta, q.prec + o.max_delta)?;
    let light = lib.light()?;
    let frag = FragTol::from_params(o.frag_da, o.frag_ppm);
    let pq = q.prec;
    let sc = flash_top(
        lib,
        q,
        FlashMode::Hybrid,
        frag,
        &ranges,
        acc,
        Some(o.candidates),
        |s| {
            s.pairs >= o.min_hybrid_pairs
                && (light.recs[s.index as usize].precq as f64 / crate::clean::MZ_SCALE - pq).abs()
                    >= o.min_delta
        },
    )?;
    let mut out = Vec::new();
    for s in sc {
        let rec = lib.rec(s.index)?;
        let spec = spec_of(lib, s.index, rec.prec)?;
        let mc = modified_cosine(&q.spec, &spec, frag);
        if mc.matched < o.min_matched {
            continue;
        }
        out.push(AnalogHit {
            index: s.index,
            prec: rec.prec,
            pol: rec.pol,
            licence: rec.licence,
            hybrid: s.score,
            coinc: s.pairs,
            mc,
            spec,
        });
    }
    sortc::any_by(&mut out, |a, b| {
        b.mc.cosine
            .total_cmp(&a.mc.cosine)
            .then(a.index.cmp(&b.index))
    });
    out.truncate(o.top.max(1));
    Ok(out)
}
