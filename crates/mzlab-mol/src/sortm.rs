//! The one sort of this crate. `slice::sort*` is instantiated once per element type and closure (about 3 kB of `.wasm` each, and every
//! `.wasm` that links `mzlab-mol` carries them), so the sorting itself is ONE non-generic stable merge sort of indices; a call site only
//! adds its comparison closure and a swap-only permutation. Stable: where the old call was `sort_unstable*`, equal elements are identical
//! (plain numbers, tuples, lists) or the keys are unique, so the result is the same bytes.
//! `SortedVec` is the small ordered set/map the BTree types were used for: binary search, iteration in key order.

use std::cmp::Ordering;

/// Runs of this length are insertion-sorted; up to this length a whole sort needs no allocation (the index list lives on the stack).
const RUN: usize = 16;

/// Stable sort of the identity permutation `v` by `cmp` on item indices: insertion-sorted runs of `RUN`, then merges of doubling width
/// between `v` and `buf` (`buf` is only touched when `v` is longer than `RUN`). The result is in `v`.
fn sort_idx(v: &mut [u32], buf: &mut [u32], cmp: &mut dyn FnMut(usize, usize) -> Ordering) {
    let n = v.len();
    let mut s = 0;
    while s < n {
        let e = (s + RUN).min(n);
        for i in s + 1..e {
            let x = v[i];
            let mut j = i;
            while j > s && cmp(v[j - 1] as usize, x as usize) == Ordering::Greater {
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
    buf.copy_from_slice(v);
    let (mut src, mut dst): (&mut [u32], &mut [u32]) = (v, buf);
    let mut width = RUN;
    let mut in_v = true;
    while width < n {
        let mut lo = 0;
        while lo < n {
            let mid = (lo + width).min(n);
            let hi = (lo + 2 * width).min(n);
            let (mut i, mut j) = (lo, mid);
            for slot in dst[lo..hi].iter_mut() {
                // the right run goes first only when strictly smaller: equal items keep their order
                let right =
                    i >= mid || (j < hi && cmp(src[j] as usize, src[i] as usize) == Ordering::Less);
                if right {
                    *slot = src[j];
                    j += 1;
                } else {
                    *slot = src[i];
                    i += 1;
                }
            }
            lo = hi;
        }
        std::mem::swap(&mut src, &mut dst);
        in_v = !in_v;
        width *= 2;
    }
    if !in_v {
        // after an odd number of passes the sorted order is in the second buffer
        dst.copy_from_slice(src);
    }
}

/// The stable order of `n` items as a permutation (`result[i]` is the index of the item that goes to position `i`): on the stack up to `RUN`
/// items, in a `Vec` above. Not generic: ONE copy of the sort per `.wasm`, each call site only adds its comparison closure.
enum Perm {
    Small([u32; RUN], usize),
    Big(Vec<u32>),
}

impl Perm {
    fn as_mut_slice(&mut self) -> &mut [u32] {
        match self {
            Perm::Small(a, n) => &mut a[..*n],
            Perm::Big(v) => v,
        }
    }
}

fn order(n: usize, cmp: &mut dyn FnMut(usize, usize) -> Ordering) -> Perm {
    if n <= RUN {
        let mut a = [0u32; RUN];
        for (i, x) in a[..n].iter_mut().enumerate() {
            *x = i as u32;
        }
        sort_idx(&mut a[..n], &mut [], cmp);
        Perm::Small(a, n)
    } else {
        let mut v: Vec<u32> = (0..n as u32).collect();
        let mut buf = v.clone();
        sort_idx(&mut v, &mut buf, cmp);
        Perm::Big(v)
    }
}

/// Moves `v` so that `v_new[i] = v_old[idx[i]]`, cycle by cycle with swaps only (no `Clone`). `idx` is used up.
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

/// The sorts of a slice as methods, so a call site changes one word (`sort_by_key` -> `sortm_by_key`).
pub(crate) trait SortM<T> {
    fn sortm_by<F: FnMut(&T, &T) -> Ordering>(&mut self, cmp: F);
    fn sortm_by_key<K: Ord, F: FnMut(&T) -> K>(&mut self, key: F);
    fn sortm(&mut self)
    where
        T: Ord;
}

impl<T> SortM<T> for [T] {
    fn sortm_by<F: FnMut(&T, &T) -> Ordering>(&mut self, mut cmp: F) {
        if self.len() < 2 {
            return;
        }
        let mut idx = order(self.len(), &mut |a, b| cmp(&self[a], &self[b]));
        permute(self, idx.as_mut_slice());
    }
    fn sortm_by_key<K: Ord, F: FnMut(&T) -> K>(&mut self, mut key: F) {
        self.sortm_by(|a, b| key(a).cmp(&key(b)));
    }
    fn sortm(&mut self)
    where
        T: Ord,
    {
        self.sortm_by(|a, b| a.cmp(b));
    }
}

/// Index of the first item that repeats an earlier one (`None` when all differ), in O(n log n): the stable sort puts equal keys side by side
/// in their original order, so the later item of the earliest pair is the answer. For untrusted lists, where a sorted insert would be O(n^2).
pub(crate) fn first_repeat<K: Ord>(keys: &[K]) -> Option<usize> {
    let mut idx: Vec<usize> = (0..keys.len()).collect();
    idx.sortm_by(|&a, &b| keys[a].cmp(&keys[b]));
    idx.windows(2)
        .filter(|w| keys[w[0]] == keys[w[1]])
        .map(|w| w[1])
        .min()
}

/// Entries kept sorted by key: a map, or a set with `()` values. Iteration is in key order, like the BTree types it replaces.
#[derive(Clone, Debug)]
pub(crate) struct SortedVec<K, V>(Vec<(K, V)>);

impl<K: Ord, V> SortedVec<K, V> {
    pub(crate) fn new() -> Self {
        SortedVec(Vec::new())
    }

    fn find<Q: Ord + ?Sized>(&self, k: &Q) -> Result<usize, usize>
    where
        K: std::borrow::Borrow<Q>,
    {
        self.0.binary_search_by(|e| e.0.borrow().cmp(k))
    }

    pub(crate) fn get(&self, k: &K) -> Option<&V> {
        self.find(k).ok().map(|i| &self.0[i].1)
    }

    pub(crate) fn contains(&self, k: &K) -> bool {
        self.find(k).is_ok()
    }

    /// Adds or replaces; the old value comes back.
    pub(crate) fn insert(&mut self, k: K, v: V) -> Option<V> {
        match self.find(&k) {
            Ok(i) => Some(std::mem::replace(&mut self.0[i].1, v)),
            Err(i) => {
                self.0.insert(i, (k, v));
                None
            }
        }
    }

    pub(crate) fn remove(&mut self, k: &K) -> Option<V> {
        self.find(k).ok().map(|i| self.0.remove(i).1)
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, (K, V)> {
        self.0.iter()
    }
}

impl<K: Ord> SortedVec<K, ()> {
    /// Set insert: `true` when the key was not there.
    pub(crate) fn add(&mut self, k: K) -> bool {
        self.insert(k, ()).is_none()
    }
}

impl<K: Ord> FromIterator<K> for SortedVec<K, ()> {
    fn from_iter<I: IntoIterator<Item = K>>(it: I) -> Self {
        let mut v: Vec<K> = it.into_iter().collect();
        v.sortm();
        v.dedup();
        SortedVec(v.into_iter().map(|k| (k, ())).collect())
    }
}

#[cfg(test)]
#[path = "sortm_tests.rs"]
mod tests;
