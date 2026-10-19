//! Stable LSD radix sort on a 64-bit key, for the peaks of a bucket of the `.mzidx` builder (`mzidx::ordina`); constant digits cost no pass,
//! and within a 1-unit m/z bucket most of the high bits of the key are constant. Equal keys keep their input order, like `slice::sort_by`.

use std::cmp::Ordering;

use crate::error::{Error, Result};
use crate::mzidx::{by_mz, Num, Peak};

/// Slices shorter than this go to the comparison sort of the caller (the histograms would cost more than they save).
pub(crate) const MIN: usize = 160;
type Tie<'a, P> = &'a dyn Fn(&P, &P) -> Ordering;
/// A run of equal keys longer than this is finished with the stable comparison sort instead of insertion.
const RUN: usize = 24;

/// Orders `s` ascending by `key`, looking only at its digits (bytes) from `from` up, 0..=4; `tmp` is the scratch (grown to `s.len()`).
/// `tie`, when given, orders the elements whose keys are equal (the key is then a prefix of the real order).
#[allow(clippy::indexing_slicing)] // every index is a byte (`< 256`) into an array of 256, or a digit `< 8`
pub(crate) fn radix<P: Copy + Default>(
    s: &mut [P],
    tmp: &mut Vec<P>,
    key: impl Fn(&P) -> u64,
    from: usize,
    tie: Option<Tie<P>>,
) -> Result<()> {
    let n = s.len();
    let mut hist = [[0u32; 256]; 8];
    let first = s.first().map_or(0, &key);
    for p in s.iter() {
        let k = key(p);
        for (d, h) in hist[from..].iter_mut().enumerate() {
            h[(k >> (8 * (d + from))) as usize & 255] += 1;
        }
    }
    tmp.clear();
    tmp.try_reserve(n).map_err(|_| Error::memory_budget())?;
    tmp.resize(n, P::default());
    // the sorted data is in `s` when `in_s`, else in `tmp`
    let mut in_s = true;
    for (d, h) in hist.iter().enumerate().skip(from) {
        // one value in every element: the digit is the same everywhere
        if h[(first >> (8 * d)) as usize & 255] as usize == n {
            continue;
        }
        let mut at = [0usize; 256];
        let mut acc = 0;
        for (a, &c) in at.iter_mut().zip(h.iter()) {
            *a = acc;
            acc += c as usize;
        }
        let (src, dst): (&[P], &mut [P]) = if in_s {
            (&*s, &mut tmp[..])
        } else {
            (&tmp[..], &mut *s)
        };
        for p in src {
            let b = (key(p) >> (8 * d)) as usize & 255;
            dst[at[b]] = *p;
            at[b] += 1;
        }
        in_s = !in_s;
    }
    if !in_s {
        s.copy_from_slice(tmp);
    }
    if let Some(tie) = tie {
        finish_runs(s, &key, tie);
    }
    Ok(())
}

/// Orders each run of equal keys with `tie`, keeping the order of the elements `tie` calls equal.
#[allow(clippy::disallowed_methods)] // runs longer than `RUN` only: the stable std sort
#[allow(clippy::indexing_slicing)] // `a < z <= s.len()` and `j >= 1` where read
fn finish_runs<P: Copy>(s: &mut [P], key: &impl Fn(&P) -> u64, tie: Tie<P>) {
    let mut a = 0;
    while a < s.len() {
        let k = key(&s[a]);
        let mut z = a + 1;
        while z < s.len() && key(&s[z]) == k {
            z += 1;
        }
        if z - a > RUN {
            s[a..z].sort_by(tie);
        } else {
            for i in a + 1..z {
                let mut j = i;
                while j > a && tie(&s[j - 1], &s[j]) == Ordering::Greater {
                    s.swap(j - 1, j);
                    j -= 1;
                }
            }
        }
        a = z;
    }
}

/// Orders the peaks of a bucket by m/z (and by scan when `per_scansione`; equal ones keep their order): the radix sort on the 32-bit order bits
/// of an `f32` m/z (and the scan below them), or on the high half of the order bits of an `f64` and then the runs of equal halves by comparison.
#[allow(clippy::disallowed_methods)] // hot path: per-peak sort, std is ~2x faster than sortc (docs/studi/OTTIMIZZAZIONE_CORE.md §4.1)
pub(crate) fn ordina<T: Num>(
    s: &mut [Peak<T>],
    tmp: &mut Vec<Peak<T>>,
    per_scansione: bool,
) -> Result<()> {
    let cmp = |x: &Peak<T>, y: &Peak<T>| {
        if per_scansione {
            by_mz(x, y).then(x.pos.cmp(&y.pos))
        } else {
            by_mz(x, y)
        }
    };
    if s.len() < MIN {
        s.sort_by(cmp);
        return Ok(());
    }
    if T::F32 {
        // the scan goes below the order bits; without `per_scansione` the digits of the scan are not looked at
        radix(s, tmp, key32::<T>, usize::from(!per_scansione) * 4, None)
    } else {
        radix(s, tmp, key64::<T>, 4, Some(&cmp))
    }
}

/// Bits that order like the value, NaN last, `-0.0` as `0.0` (as `by_mz`): the 32 of an `f32` m/z, the scan below them.
#[inline(always)]
fn key32<T: Num>(p: &Peak<T>) -> u64 {
    let x = p.mz.to64();
    let k = match (x as f32 + 0.0).to_bits() {
        _ if x.is_nan() => u32::MAX,
        b if b >> 31 == 1 => !b,
        b => b | 1 << 31,
    };
    (u64::from(k) << 32) | u64::from(p.pos)
}

/// The same for an `f64` m/z, keeping the high half (the runs of equal halves are ordered by comparison).
#[inline(always)]
fn key64<T: Num>(p: &Peak<T>) -> u64 {
    let x = p.mz.to64();
    let k = match (x + 0.0).to_bits() {
        _ if x.is_nan() => u64::MAX,
        b if b >> 63 == 1 => !b,
        b => b | 1 << 63,
    };
    k & !0xFFFF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Default, Debug, PartialEq)]
    struct P {
        k: u64,
        id: u32,
    }

    /// The radix sort (with and without a tie-break on the low half) gives what the stable comparison sort gives, ties included.
    #[test]
    fn radix_is_the_stable_sort() {
        let mut x = 12345u64;
        let mut next = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for n in [0usize, 1, 5, 200, 3000] {
            // few distinct high halves and low halves, so that equal keys and runs longer than `RUN` happen
            let v: Vec<P> = (0..n)
                .map(|id| P {
                    k: ((next() % 7) << 40) | ((next() % 5) << 32) | (next() % 9),
                    id: id as u32,
                })
                .collect();
            let (mut a, mut b, mut tmp) = (v.clone(), v.clone(), Vec::new());
            radix(&mut a, &mut tmp, |p| p.k, 0, None).unwrap();
            b.sort_by_key(|p| p.k);
            assert_eq!(a, b);
            let (mut a, mut b) = (v.clone(), v.clone());
            radix(
                &mut a,
                &mut tmp,
                |p| p.k & !0xFFFF_FFFF,
                4,
                Some(&|x: &P, y: &P| x.k.cmp(&y.k)),
            )
            .unwrap();
            b.sort_by_key(|p| p.k);
            assert_eq!(a, b);
        }
    }
}
