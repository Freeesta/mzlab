//! Flash-style entropy search over the fragment and neutral-loss indexes (Li & Fiehn, Nat. Methods 2023): the similarity of a query to every spectrum that
//! shares a peak, from the indexes only. Contributions are summed in flat per-spectrum accumulators (`FlashAcc`); only the spectra whose pairs are not one-to-one
//! (two library peaks for one query peak or the reverse) are collected and resolved pair by pair.

use crate::clean::{weighted, MZ_SCALE};
use crate::error::Result;
use crate::fast_ln::fast_ln;
use crate::flash_resolve::score_pairs;
use crate::frag_tol::FragTol;
use crate::io::Source;
use crate::read::{Library, Light, LightRec};
use crate::search::Query;
use crate::topk;
use mzlab_picchi::sortc;
use std::ops::Range;

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

/// Best score first, then the lower index.
pub fn by_score(a: &FlashScore, b: &FlashScore) -> std::cmp::Ordering {
    b.score.total_cmp(&a.score).then(a.index.cmp(&b.index))
}

/// `ln(q / 65535)` for every index intensity `q`: the library side of the entropy term is `b = q / 65535 * scale`, so `ln b = LN_Q[q] + ln scale`.
fn ln_q() -> &'static [f64] {
    static T: std::sync::OnceLock<Vec<f64>> = std::sync::OnceLock::new();
    T.get_or_init(|| (0..=65535u32).map(|q| (q as f64 / 65535.0).ln()).collect())
}

/// Entropy term of one pair (as `scores::pair_term`) with `c = a ln a` and `ln_b` known: one logarithm instead of three.
#[inline]
fn term_of(a: f64, c: f64, b: f64, ln_b: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 {
        return 0.0;
    }
    let s = a + b;
    s / 2.0 - (c + b * ln_b - s * fast_ln(s / 2.0)) / (2.0 * std::f64::consts::LN_2)
}

const CONFLICT: u16 = 0x8000;
const COUNT: u16 = 0x7FFF;

/// Accumulator of one spectrum: the sum of the pair contributions, the pair count (with a conflict flag) and two 64-bit signatures of the query and
/// library peaks already used. One cache line holds two of them.
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct Slot {
    sum: f64,
    used_q: u64,
    used_l: u64,
    cnt: u16,
}

/// One staged candidate pair: spectrum (top bit = neutral-loss index), library value (1e-5 Da), query peak and index intensity.
#[derive(Clone, Copy)]
struct Ent {
    sp: u32,
    mz: u32,
    i: u16,
    qi: u16,
}

const NL_BIT: u32 = 1 << 31;
/// Spectra per bucket: the slots and per-spectrum tables of one bucket stay in the L2 cache while it is flushed.
const BUCKET_SHIFT: u32 = 12;
const FLUSH_AT: usize = 1 << 20;
/// Most scored entries kept for the conflicted spectra (12 bytes each) before falling back to a second walk.
const KEEP_MAX: usize = 1 << 23;

/// Flat accumulators of one search, reusable across queries and libraries (`prepare` grows them; they are all zero between searches).
/// Candidates are staged by spectrum bucket and scored bucket by bucket, so the random accesses stay inside a cache-sized window.
#[derive(Default)]
pub struct FlashAcc {
    slots: Vec<Slot>,
    touched: Vec<u32>,
    buckets: Vec<Vec<Ent>>,
    /// Entries of each bucket already scored (kept, for the conflicted spectra, until the end of the search).
    done: Vec<usize>,
    staged: usize,
    retained: usize,
    flushes: u32,
    /// Pairs of the spectra in conflict, collected by the only flush of a search (`conf_ready`).
    conf: Vec<(u32, u16, u32, f64)>,
    conf_ready: bool,
    /// Set when the kept entries outgrew `KEEP_MAX`: they were dropped and the conflicted spectra need a second walk.
    overflow: bool,
}

impl FlashAcc {
    pub fn new() -> Self {
        Self::default()
    }

    fn prepare(&mut self, n: usize) {
        if self.slots.len() < n {
            self.slots.resize(n, Slot::default());
        }
        let nb = (n >> BUCKET_SHIFT) + 1;
        if self.buckets.len() < nb {
            self.buckets.resize_with(nb, Vec::new);
            self.done.resize(nb, 0);
        }
    }

    #[inline]
    fn stage(&mut self, e: Ent) {
        if let Some(b) = self
            .buckets
            .get_mut(((e.sp & !NL_BIT) >> BUCKET_SHIFT) as usize)
        {
            b.push(e);
            self.staged += 1;
        }
    }
}

/// Spectrum and library peak (1e-5 Da) of a staged entry, `None` when the entry points outside the tables (corrupt data).
#[inline]
fn locate(e: &Ent, n_query: usize, light: &Light) -> Option<(u32, u32)> {
    let sp = e.sp & !NL_BIT;
    let r = light.recs.get(sp as usize)?;
    if e.i as usize >= n_query {
        return None;
    }
    let mz = if e.sp & NL_BIT != 0 {
        r.precq.wrapping_sub(e.mz)
    } else {
        e.mz
    };
    Some((sp, mz))
}

/// Contribution of a staged entry (as `scores::pair_term`); the caller has checked it with `locate`.
#[inline]
fn term_at(e: &Ent, pa: &[f64], pc: &[f64], lq: &[f64], r: &LightRec) -> f64 {
    let (a, c) = (pa[e.i as usize], pc[e.i as usize]);
    term_of(
        a,
        c,
        e.qi as f64 / 65535.0 * r.scale as f64,
        lq[e.qi as usize] + r.ln_scale,
    )
}

/// `(spectrum, query peak, library peak in 1e-5 Da, contribution)` of a staged entry.
#[inline]
fn pair_of(e: &Ent, pa: &[f64], pc: &[f64], light: &Light) -> Option<(u32, u16, u32, f64)> {
    let (sp, mz) = locate(e, pa.len(), light)?;
    let r = light.recs.get(sp as usize)?;
    Some((sp, e.i, mz, term_at(e, pa, pc, ln_q(), r)))
}

impl FlashAcc {
    /// Scores every staged pair into the slots, bucket by bucket. The conflicts of a bucket are found first, from the signatures alone; the entropy
    /// terms are then summed only for the spectra without a conflict. When the search ends with this very flush (`last`, and no flush before),
    /// the pairs of the spectra in conflict (`conf`) are scored in the same pass, while the slots are hot.
    fn flush(&mut self, pa: &[f64], pc: &[f64], light: &Light, last: bool) {
        let single = last && self.flushes == 0;
        self.flushes += 1;
        let mut buckets = std::mem::take(&mut self.buckets);
        let mut dones = std::mem::take(&mut self.done);
        let lq = ln_q();
        for (b, done) in buckets.iter().zip(dones.iter_mut()) {
            let fresh = b.get(*done..).unwrap_or_default();
            for e in fresh {
                if let Some((sp, mz)) = locate(e, pa.len(), light) {
                    self.mark(sp, e.i, mz);
                }
            }
            for e in fresh {
                let Some((sp, mz)) = locate(e, pa.len(), light) else {
                    continue;
                };
                let (Some(s), Some(r)) =
                    (self.slots.get_mut(sp as usize), light.recs.get(sp as usize))
                else {
                    continue;
                };
                if s.cnt & CONFLICT == 0 {
                    s.sum += term_at(e, pa, pc, lq, r);
                } else if single {
                    self.conf.push((sp, e.i, mz, term_at(e, pa, pc, lq, r)));
                }
            }
            *done = b.len();
        }
        self.conf_ready = single;
        self.retained += self.staged;
        self.staged = 0;
        if self.retained > KEEP_MAX {
            self.overflow = true;
            self.retained = 0;
            for (b, done) in buckets.iter_mut().zip(dones.iter_mut()) {
                b.clear();
                *done = 0;
            }
        }
        self.buckets = buckets;
        self.done = dones;
    }
}

mod sel;
use sel::Sel;

mod flush;

/// Entropy similarity of the query against every spectrum in `ranges` that shares at least one pair, computed from the indexes only.
/// Sorted by score, best first; spectra without a pair are not listed.
pub fn flash_scores<S: Source>(
    lib: &Library<S>,
    q: &Query,
    mode: FlashMode,
    frag_da: f64,
    ranges: &[Range<u32>],
) -> Result<Vec<FlashScore>> {
    flash_scores_with(lib, q, mode, frag_da, ranges)
}

/// `flash_scores` with the fragment tolerance as `FragTol`: in ppm the window of a query peak is that many ppm of its own m/z
/// (the shifted window too, as in `modified_cosine`); a plain Da value gives the same bits as `flash_scores`.
pub fn flash_scores_with<S: Source>(
    lib: &Library<S>,
    q: &Query,
    mode: FlashMode,
    frag: impl Into<FragTol>,
    ranges: &[Range<u32>],
) -> Result<Vec<FlashScore>> {
    flash_top(
        lib,
        q,
        mode,
        frag,
        ranges,
        &mut FlashAcc::new(),
        None,
        |_| true,
    )
}

/// Query-side tables of a search: entropy-weighted intensities and `a ln a`.
fn query_tables(q: &Query) -> (Vec<f64>, Vec<f64>) {
    let pa = weighted(&q.spec.it);
    let pc = pa
        .iter()
        .map(|a| if *a > 0.0 { a * a.ln() } else { 0.0 })
        .collect();
    (pa, pc)
}

/// Walks the windows of every query peak; `f(after_window, neutral_loss, spectrum, query peak, library value in 1e-5 Da, index intensity)` for each
/// candidate entry in `ranges`. `after_window` runs once per window (a chance to flush staged pairs).
fn walk<S: Source>(
    lib: &Library<S>,
    q: &Query,
    mode: FlashMode,
    frag: FragTol,
    ranges: &[Range<u32>],
    mut f: impl FnMut(bool, u32, u16, u32, u16),
    mut after_window: impl FnMut(),
) -> Result<()> {
    let window = |x: f64, tol_q: f64| -> Option<(u32, u32)> {
        let lo = (x - tol_q).ceil().max(0.0);
        let hi = (x + tol_q).floor();
        (hi >= 0.0 && lo <= hi && hi < u32::MAX as f64).then_some((lo as u32, hi as u32))
    };
    let sel = Sel::new(ranges, lib.n_spectra());
    for (i, &mz) in q.spec.mz.iter().enumerate() {
        let tol_q = frag.at(mz) * MZ_SCALE;
        for nl in [false, true] {
            if (nl && mode == FlashMode::Open) || (!nl && mode == FlashMode::NeutralLoss) {
                continue;
            }
            let x = if nl {
                (q.prec - mz) * MZ_SCALE
            } else {
                mz * MZ_SCALE
            };
            if let Some((lo, hi)) = window(x, tol_q) {
                lib.for_each_entry(nl, lo, hi, |v, sp, qi| {
                    if sel.has(sp) {
                        f(nl, sp, i as u16, v, qi);
                    }
                })?;
                after_window();
            }
        }
    }
    Ok(())
}

/// The search behind `flash_scores_with`, with the accumulators of the caller, only the `keep` best (all, sorted, when `None`) and a filter applied before
/// the selection. Equal to filtering and truncating the full result.
#[allow(clippy::too_many_arguments)]
pub fn flash_top<S: Source>(
    lib: &Library<S>,
    q: &Query,
    mode: FlashMode,
    frag: impl Into<FragTol>,
    ranges: &[Range<u32>],
    acc: &mut FlashAcc,
    keep: Option<usize>,
    mut accept: impl FnMut(&FlashScore) -> bool,
) -> Result<Vec<FlashScore>> {
    let frag = frag.into();
    let light = lib.light()?;
    let (pa, pc) = query_tables(q);
    acc.prepare(lib.n_spectra() as usize);
    let cell = std::cell::RefCell::new(&mut *acc);
    let swept = walk(
        lib,
        q,
        mode,
        frag,
        ranges,
        |nl, sp, i, v, qi| {
            cell.borrow_mut().stage(Ent {
                sp: if nl { sp | NL_BIT } else { sp },
                mz: v,
                i,
                qi,
            })
        },
        || {
            let mut a = cell.borrow_mut();
            if a.staged >= FLUSH_AT {
                a.flush(&pa, &pc, light, false);
            }
        },
    );
    let r = swept.and_then(|()| {
        acc.flush(&pa, &pc, light, true);
        let mut out: Vec<FlashScore> = Vec::with_capacity(acc.touched.len().min(1 << 16));
        let mut conflicts = false;
        for &sp in &acc.touched {
            let Some(slot) = acc.slots.get(sp as usize) else {
                continue;
            };
            if slot.cnt & CONFLICT != 0 {
                conflicts = true;
                continue;
            }
            let s = FlashScore {
                index: sp,
                score: slot.sum.clamp(0.0, 1.0),
                pairs: slot.cnt as u32,
            };
            if accept(&s) {
                out.push(s);
            }
        }
        if conflicts {
            let pairs = conflict_scores(lib, q, mode, frag, ranges, acc, (&pa, &pc), light)?;
            out.extend(pairs.into_iter().filter(|s| accept(s)));
        }
        Ok(out)
    });
    acc.reset();
    let mut out = r?;
    match keep {
        Some(k) => topk::keep_first(&mut out, k, by_score),
        None => sortc::by(&mut out, by_score),
    }
    Ok(out)
}

/// One-to-one scores of the spectra flagged in conflict: from the pairs collected by the only flush, else from the retained entries, else from a second walk.
#[allow(clippy::too_many_arguments)]
fn conflict_scores<S: Source>(
    lib: &Library<S>,
    q: &Query,
    mode: FlashMode,
    frag: FragTol,
    ranges: &[Range<u32>],
    acc: &mut FlashAcc,
    (pa, pc): (&[f64], &[f64]),
    light: &Light,
) -> Result<Vec<FlashScore>> {
    let flags = &acc.slots;
    let conflicted = |sp: u32| {
        flags
            .get(sp as usize)
            .is_some_and(|s| s.cnt & CONFLICT != 0)
    };
    if acc.conf_ready {
        return Ok(score_pairs(std::mem::take(&mut acc.conf)));
    }
    if acc.overflow {
        let mut pairs: Vec<(u32, u16, u32, f64)> = Vec::new();
        walk(
            lib,
            q,
            mode,
            frag,
            ranges,
            |nl, sp, i, v, qi| {
                if conflicted(sp) {
                    let e = Ent {
                        sp: if nl { sp | NL_BIT } else { sp },
                        mz: v,
                        i,
                        qi,
                    };
                    pairs.extend(pair_of(&e, pa, pc, light));
                }
            },
            || {},
        )?;
        return Ok(score_pairs(pairs));
    }
    // Bucket by bucket: each sort stays small and cache-resident.
    let mut out = Vec::new();
    for b in &acc.buckets {
        let pairs: Vec<_> = b
            .iter()
            .filter(|e| conflicted(e.sp & !NL_BIT))
            .filter_map(|e| pair_of(e, pa, pc, light))
            .collect();
        out.extend(score_pairs(pairs));
    }
    Ok(out)
}
