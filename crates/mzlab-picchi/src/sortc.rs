//! Compact stable sorts. `slice::sort*` of the standard library is instantiated once per element type and closure and costs 5-10 KB of `.wasm` each;
//! the detection of a file sorts a dozen different things and the `.wasm` of mzFinder has a limit of 400 KB, so the modules that run in it use these instead.
//! All of them are stable; the places that used `sort_unstable*` sort plain data whose equal elements are identical, so the order is the same.

use std::cmp::Ordering;

/// Ascending order of two floats with NaN after every number. Unlike `total_cmp`, -0 and +0 are equal, so data without NaN sort exactly as
/// with `partial_cmp`; unlike `partial_cmp(..).unwrap_or(Equal)` it is a total order, so one NaN cannot leave the numbers unsorted.
pub fn asc_nan_last(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b)
        .unwrap_or_else(|| a.is_nan().cmp(&b.is_nan()))
}

/// Descending order of two floats, NaN still after every number (see `asc_nan_last`).
pub fn desc_nan_last(a: f64, b: f64) -> Ordering {
    b.partial_cmp(&a)
        .unwrap_or_else(|| a.is_nan().cmp(&b.is_nan()))
}

/// Stable bottom-up merge sort for plain data: insertion sort of runs of 16, then merges of doubling width between `v` and a buffer.
pub fn by<T: Copy, F: FnMut(&T, &T) -> Ordering>(v: &mut [T], mut cmp: F) {
    let n = v.len();
    const RUN: usize = 16;
    let mut s = 0;
    while s < n {
        let e = (s + RUN).min(n);
        for i in s + 1..e {
            let x = v[i];
            let mut j = i;
            while j > s && cmp(&v[j - 1], &x) == Ordering::Greater {
                v[j] = v[j - 1];
                j -= 1;
            }
            v[j] = x;
        }
        s = e;
    }
    if n <= RUN {
        return;
    }
    let mut buf: Vec<T> = v.to_vec();
    let (mut width, mut from_v) = (RUN, true);
    while width < n {
        let (src, dst): (&[T], &mut [T]) = if from_v {
            (&*v, &mut buf[..])
        } else {
            (&buf[..], &mut *v)
        };
        let mut lo = 0;
        while lo < n {
            let mid = (lo + width).min(n);
            let hi = (lo + 2 * width).min(n);
            let (mut i, mut j) = (lo, mid);
            for slot in dst[lo..hi].iter_mut() {
                let right = i >= mid || (j < hi && cmp(&src[j], &src[i]) == Ordering::Less);
                *slot = if right { src[j] } else { src[i] };
                if right {
                    j += 1;
                } else {
                    i += 1;
                }
            }
            lo = hi;
        }
        from_v = !from_v;
        width *= 2;
    }
    if !from_v {
        v.copy_from_slice(&buf);
    }
}

/// Stable sort by a key.
pub fn by_key<T: Copy, K: Ord, F: FnMut(&T) -> K>(v: &mut [T], mut key: F) {
    by(v, |a, b| key(a).cmp(&key(b)));
}

/// Stable sort of data that orders itself.
pub fn ord<T: Copy + Ord>(v: &mut [T]) {
    by(v, |a, b| a.cmp(b));
}

/// Stable sort of any items: the order is found on indices, then the items are moved in place.
pub fn any_by<T, F: FnMut(&T, &T) -> Ordering>(v: &mut Vec<T>, cmp: F) {
    slice_by(v.as_mut_slice(), cmp);
}

/// Stable sort of any items by a key.
pub fn any_by_key<T, K: Ord, F: FnMut(&T) -> K>(v: &mut Vec<T>, mut key: F) {
    any_by(v, |a, b| key(a).cmp(&key(b)));
}

/// Median of `v` (even length: the mean of the middle pair) found by selection instead of a sort, the same value a sort gives; reorders `v`.
/// `None` when `v` holds a NaN (the order of a sort then depends on its algorithm: the caller keeps its sort for that case).
pub fn select_median(v: &mut [f64]) -> Option<f64> {
    let n = v.len();
    if n == 0 || v.iter().any(|x| x.is_nan()) {
        return None;
    }
    let k = n / 2;
    let (mut l, mut r) = (0isize, n as isize - 1);
    // Hoare partition: equal elements stop both scans, so runs of equal values (zeros) stay linear
    while l < r {
        let pivot = v[((l + r) / 2) as usize];
        let (mut i, mut j) = (l, r);
        while i <= j {
            while v[i as usize] < pivot {
                i += 1;
            }
            while v[j as usize] > pivot {
                j -= 1;
            }
            if i <= j {
                v.swap(i as usize, j as usize);
                i += 1;
                j -= 1;
            }
        }
        if (k as isize) <= j {
            r = j;
        } else if (k as isize) >= i {
            l = i;
        } else {
            break;
        }
    }
    if n % 2 == 1 {
        Some(v[k])
    } else {
        let low = v[..k]
            .iter()
            .fold(f64::NEG_INFINITY, |m, &x| if x > m { x } else { m });
        Some((low + v[k]) / 2.0)
    }
}

#[cfg(test)]
mod select_tests {
    use super::*;

    #[test]
    fn select_median_is_the_median_of_the_sorted_values() {
        let mut seed = 11u64;
        let mut r = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 33) as u32
        };
        for n in [1usize, 2, 3, 4, 7, 8, 15, 16, 17, 100, 1001, 2000] {
            for spread in [1u32, 3, 50, 100_000] {
                let v: Vec<f64> = (0..n).map(|_| f64::from(r() % spread) * 0.5).collect();
                let mut sorted = v.clone();
                by(&mut sorted, |a, b| a.total_cmp(b));
                let want = if n % 2 == 1 {
                    sorted[n / 2]
                } else {
                    (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
                };
                let mut w = v.clone();
                assert_eq!(select_median(&mut w).unwrap().to_bits(), want.to_bits());
            }
        }
        assert!(select_median(&mut []).is_none());
        assert!(select_median(&mut [1.0, f64::NAN]).is_none());
        assert_eq!(
            select_median(&mut [f64::NEG_INFINITY, f64::NEG_INFINITY]),
            Some(f64::NEG_INFINITY)
        );
    }
}

/// The stable order of `n` items as a permutation: `result[i]` is the index of the item that goes to position `i`.
/// Not generic on purpose: there is ONE copy of the sort in the `.wasm`; each call site only adds its comparison closure.
pub fn order(n: usize, cmp: &mut dyn FnMut(usize, usize) -> Ordering) -> Vec<u32> {
    let mut idx: Vec<u32> = (0..n as u32).collect();
    by(&mut idx, |&a, &b| cmp(a as usize, b as usize));
    idx
}

/// Moves `v` so that `v_new[i] = v_old[idx[i]]` (cycle by cycle, swaps only: no `Clone`/`Copy`). `idx` is used up.
fn permute<T>(v: &mut [T], idx: &mut [u32]) {
    for i in 0..v.len() {
        let mut cur = i;
        while idx[cur] as usize != i {
            let next = idx[cur] as usize;
            v.swap(cur, next);
            idx[cur] = cur as u32;
            cur = next;
        }
        idx[cur] = cur as u32;
    }
}

/// Stable sort of a slice of any items (cold paths; hot paths keep `slice::sort*`, see `clippy.toml`).
pub fn slice_by<T, F: FnMut(&T, &T) -> Ordering>(v: &mut [T], mut cmp: F) {
    let mut idx = order(v.len(), &mut |a, b| cmp(&v[a], &v[b]));
    permute(v, &mut idx);
}

/// Stable sort of a slice by a key.
pub fn slice_by_key<T, K: Ord, F: FnMut(&T) -> K>(v: &mut [T], mut key: F) {
    slice_by(v, |a, b| key(a).cmp(&key(b)));
}

/// Stable sort of a slice of items that order themselves.
pub fn slice_ord<T: Ord>(v: &mut [T]) {
    slice_by(v, |a, b| a.cmp(b));
}

/// The cold-path sorts as methods of a slice (`v.sortc_by(cmp)` where std has `v.sort_by(cmp)`), so a call site changes one word.
pub trait SortC<T> {
    fn sortc_by<F: FnMut(&T, &T) -> Ordering>(&mut self, cmp: F);
    fn sortc_by_key<K: Ord, F: FnMut(&T) -> K>(&mut self, key: F);
    fn sortc(&mut self)
    where
        T: Ord;
}

impl<T> SortC<T> for [T] {
    fn sortc_by<F: FnMut(&T, &T) -> Ordering>(&mut self, cmp: F) {
        slice_by(self, cmp);
    }
    fn sortc_by_key<K: Ord, F: FnMut(&T) -> K>(&mut self, key: F) {
        slice_by_key(self, key);
    }
    fn sortc(&mut self)
    where
        T: Ord,
    {
        slice_ord(self);
    }
}

#[cfg(test)]
#[path = "sortc_tests.rs"]
mod tests;
