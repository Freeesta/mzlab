//! The orbit pruning of the canonical search: the union-find makes the same choice as the closure it replaced at every node (the old
//! search is kept here as the reference), and the work grows as a low power of the number of equal components (counted, not timed).

use super::*;

/// The search before the union-find: the orbit of `w` rebuilt from every automorphism found so far, at each sibling.
fn old_same_orbit(s: &Search, prefix: &[usize], w: usize, v: usize) -> bool {
    let mut seen = vec![w];
    let mut i = 0;
    while i < seen.len() {
        let x = seen[i];
        i += 1;
        for g in s.autos.iter().filter(|g| prefix.iter().all(|&p| g[p] == p)) {
            if g[x] == v {
                return true;
            }
            if !seen.contains(&g[x]) {
                seen.push(g[x]);
            }
        }
    }
    false
}

/// Walks the tree like `explore`, and at every sibling checks that the union-find and the old closure give the same answer.
fn walk(s: &mut Search, cls: Vec<usize>, prefix: &mut Vec<usize>, checks: &mut usize) {
    let n = cls.len();
    let mut size = vec![0usize; n];
    for &c in &cls {
        size[c] += 1;
    }
    let Some(target) = (0..n).find(|&c| size[c] > 1) else {
        s.leaf(cls);
        return;
    };
    let members: Vec<usize> = (0..n).filter(|&i| cls[i] == target).collect();
    let mut done: Vec<usize> = Vec::new();
    let mut orbits = Orbits::new(n);
    for v in members {
        if s.visited >= LEAF_LIMIT {
            return;
        }
        let old = done.iter().any(|&w| old_same_orbit(s, prefix, w, v));
        orbits.absorb(&s.autos, prefix, &s.work);
        let new = done.iter().any(|&w| orbits.same(w, v, &s.work));
        assert_eq!(old, new, "pruning of atom {v} under prefix {prefix:?}");
        *checks += 1;
        if old {
            continue;
        }
        let split: Vec<usize> = (0..n)
            .map(|i| cls[i] * 2 + usize::from(cls[i] == target && i != v))
            .collect();
        prefix.push(v);
        let next = s.refine(split);
        walk(s, next, prefix, checks);
        prefix.pop();
        done.push(v);
    }
}

fn search(m: &Mol) -> Search<'_> {
    let mut adj = vec![Vec::new(); m.atoms.len()];
    for b in &m.bonds {
        adj[b.a as usize].push((b.b as usize, b.o));
        adj[b.b as usize].push((b.a as usize, b.o));
    }
    Search {
        mol: m,
        adj,
        leaves: HashMap::new(),
        best: None,
        autos: Vec::new(),
        visited: 0,
        work: Cell::new(0),
    }
}

fn copies(unit: &str, k: usize) -> Mol {
    Mol::from_smiles(&vec![unit; k].join(".")).expect(unit)
}

#[test]
fn union_find_prunes_exactly_like_the_old_closure() {
    let units = [
        "c1ccccc1",
        "C12C3C4C1C5C2C3C45",
        "C1C2CC3CC1CC(C2)C3",
        "N[C@@H](C)C(=O)O",
        "C/C=C/C",
        "[Na+].[Cl-]",
        "[Na+].[Na+].[O-]C(=O)C(=O)[O-]",
        "O.CCO",
    ];
    let mut checks = 0;
    for unit in units {
        for k in 1..=6 {
            let m = copies(unit, k);
            let mut s = search(&m);
            let start = s.refine(s.initial());
            walk(&mut s, start, &mut Vec::new(), &mut checks);
            let want = s.best.map(|(_, p)| p).expect(unit);
            assert_eq!(m.canonical_order(), want, "{unit} x{k}");
        }
    }
    assert!(checks > 1000, "the walk compared {checks} pruning choices");
}

#[test]
fn work_grows_polynomially_with_equal_components() {
    // The old closure grew as about the sixth power of the copies (benzene: 10 copies 26 ms, 20 copies 2.2 s, 30 copies 32 s).
    // With the component automorphisms a doubling costs about 8 times the work units (k^3: a path per copy, each step touching every atom);
    // the bound is 2^4 per doubling, far under the old growth.
    for unit in [
        "c1ccccc1",
        "C12C3C4C1C5C2C3C45",
        "[Na+].[Cl-]",
        "[Na+].[Na+].[O-]C(=O)C(=O)[O-]",
    ] {
        let work: Vec<u64> = [10usize, 20, 40]
            .iter()
            .map(|&k| copies(unit, k).canonical_order_counted().1)
            .collect();
        for w in work.windows(2) {
            assert!(
                w[1] <= 16 * w[0],
                "{unit}: work {} then {} for twice the copies",
                w[0],
                w[1]
            );
        }
    }
}

/// A permutation of `0..n` from a seed (Fisher-Yates over a small LCG).
fn shuffled(n: usize, mut seed: u64) -> Vec<u32> {
    let mut p: Vec<u32> = (0..n as u32).collect();
    for i in (1..n).rev() {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        p.swap(i, (seed >> 33) as usize % (i + 1));
    }
    p
}

/// `k` pieces alternating between two writings of one constitution with different stereo.
fn alternating(a: &str, b: &str, k: usize) -> Mol {
    let parts: Vec<&str> = (0..k).map(|i| if i % 2 == 0 { a } else { b }).collect();
    Mol::from_smiles(&parts.join(".")).expect(a)
}

#[test]
fn part_cores_are_the_cores_of_only_piece() {
    for text in [
        "C[C@H](O)CC.C[C@@H](O)CC.OCC.C[C@@H](O)CC.C/C=C/C |&1:1,6,o1:14|",
        "[Na+].[Cl-].c1ccccc1.N[C@@H](C)C(=O)O",
        "C[C@H](O)[C@@H](O)C",
    ] {
        let m0 = Mol::from_smiles(text).expect(text);
        for seed in 0..4 {
            let m = m0.renumbered(&shuffled(m0.atoms.len(), seed)).unwrap();
            let s = search(&m);
            for (label, count) in [m.piece_labels(), s.units()] {
                let cores = m.part_cores(&label, count).expect(text);
                for (k, core) in cores.iter().enumerate() {
                    let one = m.only_piece(&label, k).unwrap();
                    let same = one
                        .renumbered(&(0..one.atoms.len() as u32).collect::<Vec<_>>())
                        .unwrap();
                    assert_eq!(core.core_json(), same.core_json(), "{text} part {k}");
                }
            }
        }
    }
}

#[test]
fn small_records_of_tied_pieces_keep_the_order_of_the_plain_search() {
    // under `SEARCH_SIZE` the order is the search's, as before the units; a racemate written as two pieces keeps its `can`
    for (a, b) in [
        ("N[C@@H](C)C(=O)O", "N[C@H](C)C(=O)O"),
        ("C/C=C/C", "C/C=C\\C"),
        ("C[C@H](O)[C@@H](O)C", "C[C@H](O)[C@H](O)C"),
    ] {
        for k in 2..=6 {
            let m = alternating(a, b, k);
            let mut s = search(&m);
            let start = s.refine(s.initial());
            s.explore(start, &mut Vec::new());
            let plain = s.best.map(|(_, p)| p).expect(a);
            assert_eq!(m.canonical_order(), plain, "{a} / {b} x{k}");
        }
    }
}

#[test]
fn work_is_linear_in_tied_pieces_with_different_stereo() {
    // the search alone took about C(k, k/2) leaves (16 pieces of an amino acid and its enantiomer: 1.7 s); shuffled atoms make every
    // piece a writing of its own, so each is searched; a doubling of the pieces stays under 3 times the work
    for (a, b) in [
        ("N[C@@H](C)C(=O)O", "N[C@H](C)C(=O)O"),
        ("C[C@H](O)[C@@H](O)C", "C[C@H](O)[C@H](O)C"),
    ] {
        let work: Vec<u64> = [50usize, 100, 200]
            .iter()
            .map(|&k| {
                let m = alternating(a, b, k);
                let m = m.renumbered(&shuffled(m.atoms.len(), 7)).unwrap();
                m.canonical_order_counted().1
            })
            .collect();
        for w in work.windows(2) {
            assert!(
                w[1] <= 3 * w[0],
                "{a} / {b}: work {} then {} for twice the pieces",
                w[0],
                w[1]
            );
        }
    }
}

#[test]
fn the_search_size_bound_is_fixed_at_1000_leaves() {
    // the bound decides which records keep the plain search's order, so moving it changes `can()`: a CAN_VERSION decision
    assert_eq!(SEARCH_SIZE, 1_000);
    // R and S copies of an amino acid: C(12, 6) = 924 interleavings keep the search, C(13, 6) = 1716 order the pieces alone
    let (a, b) = ("N[C@@H](C)C(=O)O", "N[C@H](C)C(=O)O");
    let contiguous = |m: &Mol| {
        let (piece, _) = m.piece_labels();
        let order = m.canonical_order();
        let mut by_new = vec![0usize; order.len()];
        for (old, &new) in order.iter().enumerate() {
            by_new[new as usize] = piece[old];
        }
        by_new.windows(2).filter(|w| w[0] != w[1]).count() + 1 == m.pieces().len()
    };
    let below = alternating(a, b, 12);
    let mut s = search(&below);
    let start = s.refine(s.initial());
    s.explore(start, &mut Vec::new());
    assert_eq!(below.canonical_order(), s.best.map(|(_, p)| p).unwrap());
    assert!(!contiguous(&below), "the search interleaves the pieces");
    assert!(
        contiguous(&alternating(a, b, 13)),
        "past the bound each piece is in one run"
    );
}
