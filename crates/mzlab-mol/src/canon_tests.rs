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
