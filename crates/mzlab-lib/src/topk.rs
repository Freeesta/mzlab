//! Top-k selection for the Flash search: the `k` best hits in order without sorting all of them (the stable sort itself is `mzlab_picchi::sortc`).

use mzlab_picchi::sortc;
use std::cmp::Ordering;

/// Quickselect: moves the `k` smallest items by `cmp` (unordered) to the front of `v`.
fn select_first<T: Copy, F: FnMut(&T, &T) -> Ordering>(v: &mut [T], k: usize, cmp: &mut F) {
    let kk = k as isize - 1;
    let (mut lo, mut hi) = (0isize, v.len() as isize - 1);
    while lo < hi {
        let p = v[(lo + (hi - lo) / 2) as usize];
        let (mut i, mut j) = (lo, hi);
        while i <= j {
            while cmp(&v[i as usize], &p) == Ordering::Less {
                i += 1;
            }
            while cmp(&p, &v[j as usize]) == Ordering::Less {
                j -= 1;
            }
            if i <= j {
                v.swap(i as usize, j as usize);
                i += 1;
                j -= 1;
            }
        }
        if kk <= j {
            hi = j;
        } else if kk >= i {
            lo = i;
        } else {
            break;
        }
    }
}

/// Keeps in `v` the `k` smallest items by `cmp`, in order (quickselect, then a sort of the survivors); `cmp` must be a total order.
pub fn keep_first<T: Copy, F: FnMut(&T, &T) -> Ordering>(v: &mut Vec<T>, k: usize, mut cmp: F) {
    if k == 0 {
        v.clear();
        return;
    }
    if v.len() > k {
        select_first(v, k, &mut cmp);
        v.truncate(k);
    }
    sortc::by(v, cmp);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_first_matches_a_full_sort() {
        let mut x = 12345u64;
        for n in [0usize, 1, 2, 5, 40, 300] {
            let v: Vec<(u32, u32)> = (0..n as u32)
                .map(|i| {
                    x = x
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    ((x >> 40) as u32 % 7, i)
                })
                .collect();
            let mut full = v.clone();
            sortc::by(&mut full, |a, b| a.cmp(b));
            for k in [0usize, 1, 3, n / 2, n, n + 4] {
                let mut got = v.clone();
                keep_first(&mut got, k, |a, b| a.cmp(b));
                assert_eq!(got, full[..k.min(n)], "n={n} k={k}");
            }
        }
    }
}
