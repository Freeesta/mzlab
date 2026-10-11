//! Searching a library: identity (precursor window, every candidate scored on its peaks) and Flash-style open / neutral-loss / hybrid searches
//! over the fragment and neutral-loss indexes (Li & Fiehn, Nat. Methods 2023), used for the analogues.

use crate::clean::{clean, weighted, CleanOpts};
use crate::error::Result;
use crate::io::Source;
use crate::read::Library;
use crate::scores::{common_losses, modified_cosine, pair_scores, pair_term, ModCosine, Spec};
use crate::sortutil;
use std::ops::Range;

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
    let w = o.prec_tol.window(q.prec);
    let mut hits: Vec<Hit> = Vec::new();
    for r in lib.ranges(q.pol, q.prec - w, q.prec + w)? {
        for i in r {
            let rec = lib.rec(i)?;
            let s = spec_of(lib, i, rec.prec)?;
            let sc = pair_scores(&q.spec, &s, o.frag_da);
            let d = rec.prec - q.prec;
            hits.push(Hit {
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
                pairs: sc.pairs,
                spec: s,
            });
        }
    }
    sortutil::sort_any_by(&mut hits, |a, b| {
        b.entropy
            .total_cmp(&a.entropy)
            .then(b.cosine.total_cmp(&a.cosine))
            .then(a.index.cmp(&b.index))
    });
    hits.truncate(o.top.max(1));
    for h in hits.iter_mut() {
        h.nl_shared = common_losses(&q.spec, &h.spec, o.frag_da);
    }
    Ok(hits)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashMode {
    /// Fragments equal within the tolerance.
    Open,
    /// Neutral losses (precursor − m/z) equal within the tolerance.
    NeutralLoss,
    /// Either: a peak pairs unshifted or shifted by the precursor difference; every peak pairs once.
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlashScore {
    pub index: u32,
    pub score: f64,
    pub pairs: u32,
}

fn in_ranges(r: &[Range<u32>], i: u32) -> bool {
    r.iter().any(|x| x.contains(&i))
}

/// Entropy similarity of the query against every spectrum in `ranges` that shares at least one pair, computed from the indexes only.
/// Sorted by score, best first; spectra without a pair are not listed.
pub fn flash_scores<S: Source>(
    lib: &Library<S>,
    q: &Query,
    mode: FlashMode,
    frag_da: f64,
    ranges: &[Range<u32>],
) -> Result<Vec<FlashScore>> {
    let light = lib.light()?;
    let pa = weighted(&q.spec.it);
    let tol_q = frag_da * crate::clean::MZ_SCALE;
    // (spectrum, query peak, library peak in 1e-5 Da units, contribution)
    let mut pairs: Vec<(u32, u16, u32, f64)> = Vec::new();
    let window = |x: f64| -> Option<(u32, u32)> {
        let lo = (x - tol_q).ceil().max(0.0);
        let hi = (x + tol_q).floor();
        (hi >= 0.0 && lo <= hi && hi < u32::MAX as f64).then_some((lo as u32, hi as u32))
    };
    for (i, &mz) in q.spec.mz.iter().enumerate() {
        if mode != FlashMode::NeutralLoss {
            if let Some((lo, hi)) = window(mz * crate::clean::MZ_SCALE) {
                lib.for_each_entry(false, lo, hi, |mq, sp, qi| {
                    if in_ranges(ranges, sp) {
                        let p = qi as f64 / 65535.0 * light.scale[sp as usize] as f64;
                        pairs.push((sp, i as u16, mq, pair_term(pa[i], p)));
                    }
                })?;
            }
        }
        if mode != FlashMode::Open {
            if let Some((lo, hi)) = window((q.prec - mz) * crate::clean::MZ_SCALE) {
                lib.for_each_entry(true, lo, hi, |nq, sp, qi| {
                    if in_ranges(ranges, sp) {
                        let p = qi as f64 / 65535.0 * light.scale[sp as usize] as f64;
                        let mq = light.precq[sp as usize].wrapping_sub(nq);
                        pairs.push((sp, i as u16, mq, pair_term(pa[i], p)));
                    }
                })?;
            }
        }
    }
    sortutil::sort_by(&mut pairs, |a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));
    pairs.dedup_by(|b, a| (a.0, a.1, a.2) == (b.0, b.1, b.2));
    let mut out: Vec<FlashScore> = Vec::new();
    let mut k = 0;
    while k < pairs.len() {
        let sp = pairs[k].0;
        let mut e = k;
        while e < pairs.len() && pairs[e].0 == sp {
            e += 1;
        }
        let g = &pairs[k..e];
        let clean_group = g.windows(2).all(|w| w[0].1 != w[1].1) && {
            let mut js: Vec<u32> = g.iter().map(|x| x.2).collect();
            sortutil::sort_by(&mut js, |a, b| a.cmp(b));
            js.windows(2).all(|w| w[0] != w[1])
        };
        let (score, n) = if clean_group {
            (g.iter().map(|x| x.3).sum::<f64>(), g.len())
        } else {
            let mut order: Vec<usize> = (0..g.len()).collect();
            sortutil::sort_by(&mut order, |&a, &b| {
                g[b].3.total_cmp(&g[a].3).then(a.cmp(&b))
            });
            let (mut ui, mut uj) = (Vec::new(), Vec::new());
            let (mut s, mut n) = (0.0, 0);
            for x in order {
                if !ui.contains(&g[x].1) && !uj.contains(&g[x].2) {
                    ui.push(g[x].1);
                    uj.push(g[x].2);
                    s += g[x].3;
                    n += 1;
                }
            }
            (s, n)
        };
        out.push(FlashScore {
            index: sp,
            score: score.clamp(0.0, 1.0),
            pairs: n as u32,
        });
        k = e;
    }
    sortutil::sort_by(&mut out, |a, b| {
        b.score.total_cmp(&a.score).then(a.index.cmp(&b.index))
    });
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct AnalogOpts {
    pub frag_da: f64,
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
    let pol = if o.pol != 0 { o.pol } else { q.pol };
    let ranges = lib.ranges(pol, q.prec - o.max_delta, q.prec + o.max_delta)?;
    let light = lib.light()?;
    let mut sc = flash_scores(lib, q, FlashMode::Hybrid, o.frag_da, &ranges)?;
    let pq = q.prec;
    sc.retain(|s| {
        s.pairs >= o.min_hybrid_pairs
            && (light.precq[s.index as usize] as f64 / crate::clean::MZ_SCALE - pq).abs()
                >= o.min_delta
    });
    sc.truncate(o.candidates);
    let mut out = Vec::new();
    for s in sc {
        let rec = lib.rec(s.index)?;
        let spec = spec_of(lib, s.index, rec.prec)?;
        let mc = modified_cosine(&q.spec, &spec, o.frag_da);
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
    sortutil::sort_any_by(&mut out, |a, b| {
        b.mc.cosine
            .total_cmp(&a.mc.cosine)
            .then(a.index.cmp(&b.index))
    });
    out.truncate(o.top.max(1));
    Ok(out)
}
