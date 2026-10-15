//! Compact stable sorts. `slice::sort*` of the standard library is instantiated once per element type and closure and costs 5-10 KB of `.wasm` each;
//! the detection of a file sorts a dozen different things and the `.wasm` of mzFinder has a limit of 400 KB, so the modules that run in it use these instead.
//! All of them are stable; the places that used `sort_unstable*` sort plain data whose equal elements are identical, so the order is the same.

use std::cmp::Ordering;

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

/// Stable sort of any items: the order is found on indices, then the items are moved.
pub fn any_by<T, F: FnMut(&T, &T) -> Ordering>(v: &mut Vec<T>, mut cmp: F) {
    let mut idx: Vec<u32> = (0..v.len() as u32).collect();
    by(&mut idx, |&a, &b| cmp(&v[a as usize], &v[b as usize]));
    let mut slots: Vec<Option<T>> = v.drain(..).map(Some).collect();
    v.extend(idx.iter().filter_map(|&i| slots[i as usize].take()));
}

/// Stable sort of any items by a key.
pub fn any_by_key<T, K: Ord, F: FnMut(&T) -> K>(v: &mut Vec<T>, mut key: F) {
    any_by(v, |a, b| key(a).cmp(&key(b)));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_like_the_standard_stable_sort() {
        let mut seed = 7u64;
        let mut r = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 33) as u32
        };
        for n in [0usize, 1, 2, 15, 16, 17, 31, 32, 33, 100, 1000, 5000] {
            let a: Vec<(u32, u32)> = (0..n).map(|i| (r() % 20, i as u32)).collect();
            let (mut x, mut y) = (a.clone(), a.clone());
            by(&mut x, |p, q| p.0.cmp(&q.0));
            y.sort_by(|p, q| p.0.cmp(&q.0));
            assert_eq!(x, y, "n = {n}");
            let (mut z, mut w) = (a.clone(), a.clone());
            by_key(&mut z, |p| p.1 % 7);
            w.sort_by_key(|p| p.1 % 7);
            assert_eq!(z, w);
            let mut s: Vec<String> = a.iter().map(|p| format!("{}-{}", p.0, p.1)).collect();
            let mut t = s.clone();
            any_by(&mut s, |p, q| p.len().cmp(&q.len()));
            t.sort_by(|p, q| p.len().cmp(&q.len()));
            assert_eq!(s, t);
        }
    }

    #[test]
    fn slice_sorts_like_the_standard_stable_sort() {
        let mut seed = 99u64;
        let mut r = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 33) as u32
        };
        for round in 0..1000 {
            let n = (r() % 70) as usize;
            let a: Vec<(u32, usize)> = (0..n).map(|i| (r() % (3 + round % 9), i)).collect();
            let (mut x, mut y) = (a.clone(), a.clone());
            slice_by(&mut x, |p, q| p.0.cmp(&q.0));
            y.sort_by_key(|p| p.0);
            assert_eq!(x, y, "round {round}");
            let (mut z, mut w) = (a.clone(), a.clone());
            slice_by_key(&mut z, |p| std::cmp::Reverse(p.0));
            w.sort_by_key(|p| std::cmp::Reverse(p.0));
            assert_eq!(z, w);
            let mut s: Vec<String> = a.iter().map(|p| format!("{}", p.0)).collect();
            let mut u = s.clone();
            slice_ord(&mut s);
            u.sort();
            assert_eq!(s, u);
        }
    }
}
