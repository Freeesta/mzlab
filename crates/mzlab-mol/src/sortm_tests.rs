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

#[test]
fn sorted_vec_keys_can_be_tuples_and_lists_and_a_set_reports_new_keys() {
    let mut t: SortedVec<(u8, u32, u32), ()> = SortedVec::new();
    assert!(t.add((1, 5, 2)));
    assert!(t.add((0, 9, 0)));
    assert!(!t.add((1, 5, 2)));
    let keys: Vec<_> = t.iter().map(|e| e.0).collect();
    assert_eq!(keys, [(0, 9, 0), (1, 5, 2)]);

    let mut l: SortedVec<Vec<usize>, ()> = SortedVec::new();
    for k in [vec![3, 1], vec![1, 2, 3], vec![1, 2], vec![3, 1]] {
        l.add(k);
    }
    assert_eq!(l.len(), 3);
    assert!(l.contains(&vec![1, 2]) && !l.contains(&vec![2]));
    let order: Vec<_> = l.iter().map(|e| e.0.clone()).collect();
    assert_eq!(order, [vec![1, 2], vec![1, 2, 3], vec![3, 1]]);
}

#[test]
fn collecting_into_a_sorted_set_sorts_and_drops_repeats() {
    let s: SortedVec<u32, ()> = [5u32, 1, 5, 3, 1].into_iter().collect();
    assert_eq!(s.iter().map(|e| e.0).collect::<Vec<_>>(), [1, 3, 5]);
}

#[test]
fn first_repeat_is_the_first_item_equal_to_an_earlier_one() {
    let mut seed = 5u64;
    for n in [0usize, 1, 2, 5, 40, 300] {
        for spread in [3u32, 30, 100_000] {
            let v: Vec<u32> = (0..n).map(|_| rng(&mut seed) % spread).collect();
            let want = (0..n).find(|&i| v[..i].contains(&v[i]));
            assert_eq!(first_repeat(&v), want, "n={n} spread={spread}");
        }
    }
}
