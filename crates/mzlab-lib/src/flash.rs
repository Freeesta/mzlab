//! Flash-style entropy search over the fragment and neutral-loss indexes (Li & Fiehn, Nat. Methods 2023): the similarity of a query to every spectrum that
//! shares a peak, from the indexes only. Contributions are summed in flat per-spectrum accumulators (`FlashAcc`); only the spectra whose pairs are not one-to-one
//! (two library peaks for one query peak or the reverse) are collected and resolved pair by pair.

use crate::clean::{weighted, MZ_SCALE};
use crate::error::Result;
use crate::flash_resolve::score_pairs;
use crate::frag_tol::FragTol;
use crate::io::Source;
use crate::read::{Library, Light};
use crate::search::Query;
use crate::sortutil;
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

/// Natural logarithm to ~1e-13 relative error (range reduction to [√½, √2) and the atanh series), about three times faster than `f64::ln`;
/// subnormal, zero, negative or non-finite arguments use `f64::ln`.
#[inline]
fn fast_ln(x: f64) -> f64 {
    let bits = x.to_bits();
    let ex = ((bits >> 52) & 0x7ff) as i64;
    if ex == 0 || ex == 0x7ff || x < 0.0 {
        return x.ln();
    }
    let mut e = ex - 1023;
    let mut m = f64::from_bits((bits & 0x000F_FFFF_FFFF_FFFF) | (1023u64 << 52));
    if m > std::f64::consts::SQRT_2 {
        m *= 0.5;
        e += 1;
    }
    let t = (m - 1.0) / (m + 1.0);
    let t2 = t * t;
    let poly = 1.0
        + t2 * (1.0 / 3.0
            + t2 * (1.0 / 5.0
                + t2 * (1.0 / 7.0
                    + t2 * (1.0 / 9.0
                        + t2 * (1.0 / 11.0 + t2 * (1.0 / 13.0 + t2 * (1.0 / 15.0)))))));
    e as f64 * std::f64::consts::LN_2 + 2.0 * t * poly
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

    /// Scores every staged pair into the slots, bucket by bucket. When the search ends with this very flush (`last`, and no flush before), each
    /// bucket is also scanned right after it is scored, while its slots are hot, for the pairs of the spectra in conflict (`conf`).
    fn flush(&mut self, pa: &[f64], pc: &[f64], light: &Light, last: bool) {
        let single = last && self.flushes == 0;
        self.flushes += 1;
        let mut buckets = std::mem::take(&mut self.buckets);
        let mut dones = std::mem::take(&mut self.done);
        let mut tmp: Vec<(u32, u16, u32, f64)> = Vec::new();
        for (b, done) in buckets.iter().zip(dones.iter_mut()) {
            tmp.clear();
            for e in b.iter().skip(*done) {
                if let Some(p) = pair_of(e, pa, pc, light) {
                    self.add(p.0, p.1, p.2, p.3);
                    if single {
                        tmp.push(p);
                    }
                }
            }
            if single {
                let slots = &self.slots;
                self.conf.extend(tmp.iter().filter(|p| {
                    slots
                        .get(p.0 as usize)
                        .is_some_and(|s| s.cnt & CONFLICT != 0)
                }));
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

    /// Adds one pair; a repeated query or library peak (or a signature collision) marks the spectrum as in conflict.
    #[inline]
    fn add(&mut self, sp: u32, qi: u16, lib_mz: u32, term: f64) {
        let Some(s) = self.slots.get_mut(sp as usize) else {
            return;
        };
        let c = s.cnt;
        if c == 0 {
            self.touched.push(sp);
        }
        let bq = 1u64 << (qi & 63);
        let bl = 1u64 << ((lib_mz as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 58);
        let clash = (s.used_q & bq != 0) || (s.used_l & bl != 0);
        s.used_q |= bq;
        s.used_l |= bl;
        s.sum += term;
        let n = if c & COUNT == COUNT {
            COUNT
        } else {
            (c & COUNT) + 1
        };
        s.cnt = n | (c & CONFLICT) | if clash || n == COUNT { CONFLICT } else { 0 };
    }

    fn reset(&mut self) {
        for &sp in &self.touched {
            if let Some(s) = self.slots.get_mut(sp as usize) {
                *s = Slot::default();
            }
        }
        self.touched.clear();
        for (b, done) in self.buckets.iter_mut().zip(self.done.iter_mut()) {
            b.clear();
            *done = 0;
        }
        self.staged = 0;
        self.retained = 0;
        self.overflow = false;
        self.flushes = 0;
        self.conf.clear();
        self.conf_ready = false;
    }
}

/// `(spectrum, query peak, library peak in 1e-5 Da, contribution)` of a staged entry.
#[inline]
fn pair_of(e: &Ent, pa: &[f64], pc: &[f64], light: &Light) -> Option<(u32, u16, u32, f64)> {
    let sp = e.sp & !NL_BIT;
    let k = sp as usize;
    let (&sc, &ls, &p, &c, &lq) = (
        light.scale.get(k)?,
        light.ln_scale.get(k)?,
        pa.get(e.i as usize)?,
        pc.get(e.i as usize)?,
        ln_q().get(e.qi as usize)?,
    );
    let term = term_of(p, c, e.qi as f64 / 65535.0 * sc as f64, lq + ls);
    let mz = if e.sp & NL_BIT != 0 {
        light.precq.get(k)?.wrapping_sub(e.mz)
    } else {
        e.mz
    };
    Some((sp, e.i, mz, term))
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
                    if in_ranges(ranges, sp) {
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
            let flags = &acc.slots;
            let conflicted = |sp: u32| {
                flags
                    .get(sp as usize)
                    .is_some_and(|s| s.cnt & CONFLICT != 0)
            };
            let mut pairs: Vec<(u32, u16, u32, f64)> = Vec::new();
            if acc.conf_ready {
                out.extend(
                    score_pairs(std::mem::take(&mut acc.conf))
                        .into_iter()
                        .filter(|s| accept(s)),
                );
            } else if acc.overflow {
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
                            pairs.extend(pair_of(&e, &pa, &pc, light));
                        }
                    },
                    || {},
                )?;
                out.extend(score_pairs(pairs).into_iter().filter(|s| accept(s)));
            } else {
                // Bucket by bucket: each sort stays small and cache-resident.
                for b in &acc.buckets {
                    let pairs: Vec<_> = b
                        .iter()
                        .filter(|e| conflicted(e.sp & !NL_BIT))
                        .filter_map(|e| pair_of(e, &pa, &pc, light))
                        .collect();
                    out.extend(score_pairs(pairs).into_iter().filter(|s| accept(s)));
                }
            }
        }
        Ok(out)
    });
    acc.reset();
    let mut out = r?;
    match keep {
        Some(k) => sortutil::keep_first(&mut out, k, by_score),
        None => sortutil::sort_by(&mut out, by_score),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::fast_ln;

    #[test]
    fn fast_ln_matches_std() {
        let mut x = 1e-9f64;
        while x < 1e9 {
            let (a, b) = (fast_ln(x), x.ln());
            assert!((a - b).abs() <= 1e-12 * b.abs().max(1.0), "{x}: {a} vs {b}");
            x *= 1.0137;
        }
        assert!(fast_ln(0.0).is_infinite() && fast_ln(-1.0).is_nan());
    }
}
