//! Times `Mol::can` on records made of k equal pieces that carry different stereo (enantiomers, defined and unknown, cis and trans)
//! and checks that shuffled atom orders give the same string; prints one tab-separated row per record: unit, k, milliseconds, the
//! number of distinct `can` over the shuffles (1 = canonical).
//! Run: `cargo run --release -p mzlab-mol --example canon_bench [-- K...]`.

use std::time::Instant;

use mzlab_mol::Mol;

/// Pairs of writings of one constitution with different stereo; the record alternates them.
const PAIRS: &[(&str, &str)] = &[
    ("N[C@@H](C)C(=O)O", "N[C@H](C)C(=O)O"),
    ("N[C@@H](C)C(=O)O", "NC(C)C(=O)O"),
    ("C/C=C/C", "C/C=C\\C"),
    ("C[C@H](O)[C@@H](O)C", "C[C@H](O)[C@H](O)C"),
];

/// A permutation of 0..n from a small linear congruential generator (no dependency, the same on every run).
fn shuffle(n: usize, seed: u64) -> Vec<u32> {
    let mut p: Vec<u32> = (0..n as u32).collect();
    let mut x = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    for i in (1..n).rev() {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = ((x >> 33) % (i as u64 + 1)) as usize;
        p.swap(i, j);
    }
    p
}

fn main() {
    let ks: Vec<usize> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let ks = if ks.is_empty() {
        vec![2, 4, 8, 16, 32, 64]
    } else {
        ks
    };
    println!("unit\tk\tms\tdistinct_can");
    for &(a, b) in PAIRS {
        for &k in &ks {
            let parts: Vec<&str> = (0..k).map(|i| if i % 2 == 0 { a } else { b }).collect();
            let m = Mol::from_smiles(&parts.join(".")).expect("a synthetic SMILES");
            let t = Instant::now();
            let want = m.can();
            let ms = t.elapsed().as_secs_f64() * 1e3;
            let mut seen = vec![want];
            for seed in 1..=5 {
                let s = m
                    .renumbered(&shuffle(m.atoms.len(), seed))
                    .expect("a permutation");
                let c = s.can();
                if !seen.contains(&c) {
                    seen.push(c);
                }
            }
            println!("{a}|{b}\t{k}\t{ms:.1}\t{}", seen.len());
        }
    }
}
