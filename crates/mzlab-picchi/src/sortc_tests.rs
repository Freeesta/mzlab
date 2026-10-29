//! Tests of the sibling module (loaded with `#[path]`, so they see its private items).

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
        y.sort_by_key(|p| p.0);
        assert_eq!(x, y, "n = {n}");
        let (mut z, mut w) = (a.clone(), a.clone());
        by_key(&mut z, |p| p.1 % 7);
        w.sort_by_key(|p| p.1 % 7);
        assert_eq!(z, w);
        let mut s: Vec<String> = a.iter().map(|p| format!("{}-{}", p.0, p.1)).collect();
        let mut t = s.clone();
        any_by(&mut s, |p, q| p.len().cmp(&q.len()));
        t.sort_by_key(|p| p.len());
        assert_eq!(s, t);
    }
}

#[test]
fn any_by_is_stable_on_many_sequences_with_ties() {
    let mut seed = 5u64;
    let mut r = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 33) as u32
    };
    for round in 0..10_000u32 {
        let n = (r() % 40) as usize;
        let a: Vec<(u32, String)> = (0..n)
            .map(|i| (r() % (1 + round % 6), format!("v{i}")))
            .collect();
        let (mut x, mut y) = (a.clone(), a);
        any_by(&mut x, |p, q| p.0.cmp(&q.0));
        y.sort_by_key(|p| p.0);
        assert_eq!(x, y, "round {round}");
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
