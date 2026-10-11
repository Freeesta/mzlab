//! Small stable sorts. `slice::sort*` of the standard library is instantiated once per element type and closure and costs ~6-10 KB of `.wasm` each;
//! the library sorts a dozen different things, so it uses these two instead (the page loads the `.wasm` only when the libraries are opened).

use std::cmp::Ordering;

/// Stable bottom-up merge sort for plain data.
pub fn sort_by<T: Copy, F: FnMut(&T, &T) -> Ordering>(v: &mut [T], mut cmp: F) {
    let n = v.len();
    if n < 2 {
        return;
    }
    // insertion sort of runs of 16, then merge doubling widths between `v` and a buffer
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
    let mut width = RUN;
    let mut from_v = true;
    while width < n {
        {
            let (src, dst): (&[T], &mut [T]) = if from_v {
                (&*v, &mut buf[..])
            } else {
                (&buf[..], &mut *v)
            };
            let mut lo = 0;
            while lo < n {
                let mid = (lo + width).min(n);
                let hi = (lo + 2 * width).min(n);
                let (mut i, mut j, mut k) = (lo, mid, lo);
                while i < mid && j < hi {
                    if cmp(&src[j], &src[i]) == Ordering::Less {
                        dst[k] = src[j];
                        j += 1;
                    } else {
                        dst[k] = src[i];
                        i += 1;
                    }
                    k += 1;
                }
                while i < mid {
                    dst[k] = src[i];
                    i += 1;
                    k += 1;
                }
                while j < hi {
                    dst[k] = src[j];
                    j += 1;
                    k += 1;
                }
                lo = hi;
            }
        }
        from_v = !from_v;
        width *= 2;
    }
    if !from_v {
        v.copy_from_slice(&buf);
    }
}

/// Stable sort of any items (the hits): the order is found on indices, then the items are moved.
pub fn sort_any_by<T, F: FnMut(&T, &T) -> Ordering>(v: &mut Vec<T>, mut cmp: F) {
    let mut idx: Vec<u32> = (0..v.len() as u32).collect();
    sort_by(&mut idx, |&a, &b| cmp(&v[a as usize], &v[b as usize]));
    let mut slots: Vec<Option<T>> = v.drain(..).map(Some).collect();
    v.extend(idx.iter().filter_map(|&i| slots[i as usize].take()));
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
            let mut x = a.clone();
            let mut y = a.clone();
            sort_by(&mut x, |p, q| p.0.cmp(&q.0));
            y.sort_by(|p, q| p.0.cmp(&q.0));
            assert_eq!(x, y, "n = {n}");
            let mut z: Vec<String> = a.iter().map(|p| format!("{}-{}", p.0, p.1)).collect();
            let mut w = z.clone();
            sort_any_by(&mut z, |p, q| {
                p.split('-')
                    .next()
                    .unwrap()
                    .len()
                    .cmp(&q.split('-').next().unwrap().len())
                    .then(p.cmp(q))
            });
            w.sort_by(|p, q| {
                p.split('-')
                    .next()
                    .unwrap()
                    .len()
                    .cmp(&q.split('-').next().unwrap().len())
                    .then(p.cmp(q))
            });
            assert_eq!(z, w);
        }
    }
}
