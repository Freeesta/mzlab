use super::*;

fn rng(seed: &mut u64) -> u32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 33) as u32
}

#[test]
fn sortm_matches_the_std_stable_sort_on_every_length() {
    let mut seed = 7u64;
    for n in [0usize, 1, 2, 3, 15, 16, 17, 31, 32, 33, 100, 257, 1000] {
        for spread in [1u32, 4, 50, 100_000] {
            let v: Vec<(u32, u32)> = (0..n as u32)
                .map(|i| (rng(&mut seed) % spread, i))
                .collect();
            let mut want = v.clone();
            want.sort_by_key(|p| p.0);
            let mut got = v.clone();
            got.sortm_by_key(|p| p.0);
            assert_eq!(got, want, "stable by key, n={n} spread={spread}");
            let mut want = v.clone();
            want.sort();
            let mut got = v.clone();
            got.sortm();
            assert_eq!(got, want, "ord, n={n} spread={spread}");
            let mut want = v.clone();
            want.sort_by(|a, b| a.0.cmp(&b.0).reverse());
            let mut got = v.clone();
            got.sortm_by(|a, b| a.0.cmp(&b.0).reverse());
            assert_eq!(got, want, "descending, n={n} spread={spread}");
        }
    }
}

#[test]
fn sortm_moves_items_that_are_not_copy() {
    let mut v: Vec<String> = ["pear", "fig", "apple", "kiwi", "date"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    v.sortm_by_key(|s| s.len());
    assert_eq!(v, ["fig", "pear", "kiwi", "date", "apple"]);
}

#[test]
fn sorted_vec_behaves_like_a_btree_map() {
    let mut seed = 3u64;
    let mut a: SortedVec<u32, u32> = SortedVec::new();
    let mut b = std::collections::BTreeMap::new();
    for _ in 0..2000 {
        let (k, v) = (rng(&mut seed) % 60, rng(&mut seed));
        match rng(&mut seed) % 3 {
            0 => assert_eq!(a.insert(k, v), b.insert(k, v)),
            1 => assert_eq!(a.remove(&k), b.remove(&k)),
            _ => assert_eq!(a.get(&k), b.get(&k)),
        }
        assert_eq!(a.len(), b.len());
        assert_eq!(a.contains(&k), b.contains_key(&k));
    }
    let x: Vec<(u32, u32)> = a.iter().copied().collect();
    let y: Vec<(u32, u32)> = b.into_iter().collect();
    assert_eq!(x, y);
}
