//! A decoy structure for the validation of the fragmentation rules: the same atoms (so the same formula) with the bonds rewired by swaps that keep
//! the order and the number of bonds of every atom, with a fixed seed. Used by `mzlab-cli msn-rules`; a decoy is not a molecule anyone drew.

use super::grafo::{Bond, Mol};

/// SplitMix64: a few lines, the same sequence on every platform.
struct Seme(u64);

impl Seme {
    fn prossimo(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn sotto(&mut self, n: usize) -> usize {
        (self.prossimo() % n as u64) as usize
    }
}

impl Mol {
    /// The same atoms, hydrogens and charges with the bonds rewired (`(a,b),(c,d)` become `(a,d),(c,b)` between bonds of equal order): the formula is
    /// the same, the groups are not. Deterministic for a `seme`.
    pub fn rimescola(&self, seme: u64) -> Mol {
        let mut s = Seme(seme);
        let mut bonds: Vec<Bond> = self.bonds.clone();
        let n = bonds.len();
        if n >= 2 {
            for _ in 0..(10 * n) {
                let (i, j) = (s.sotto(n), s.sotto(n));
                let capovolto = s.sotto(2) == 1;
                scambia(&mut bonds, i, j, capovolto);
            }
        }
        let mut adj = vec![Vec::new(); self.atoms.len()];
        for (k, b) in bonds.iter().enumerate() {
            adj[b.a].push((b.b, k));
            adj[b.b].push((b.a, k));
        }
        Mol {
            atoms: self.atoms.clone(),
            bonds,
            adj,
        }
    }
}

/// One swap of the ends of bonds `i` and `j`, only when it makes no loop and no second bond between two atoms.
fn scambia(bonds: &mut [Bond], i: usize, j: usize, capovolto: bool) {
    if i == j || bonds[i].order != bonds[j].order {
        return;
    }
    let (a, b) = (bonds[i].a, bonds[i].b);
    let (c, d) = if capovolto {
        (bonds[j].b, bonds[j].a)
    } else {
        (bonds[j].a, bonds[j].b)
    };
    let esiste = |x: usize, y: usize| {
        bonds
            .iter()
            .any(|k| (k.a == x && k.b == y) || (k.a == y && k.b == x))
    };
    if a == d || c == b || esiste(a, d) || esiste(c, b) {
        return;
    }
    bonds[i].b = d;
    bonds[j].a = c;
    bonds[j].b = b;
}
