// Ported from Attributio plugins/rt_prediction.py.
//! Retention-time prediction from the 2D structure, to tell positional isomers apart: Morgan-style fingerprints (radius 2, 2048 bits) of the structure graph and a
//! random forest of regression trees (100 trees, bootstrap, all the bits tried at every split) trained on the standards of the campaign, so that it follows its
//! column and gradient. Without RDKit and scikit-learn the bits and the trees are not those of the original: the parity is a measured error bar (see the test),
//! not equality. A predicted RT supports or doubts a candidate; it never makes an identity.

use crate::chem::grafo::Mol;

pub const N_BITS: usize = 2048;
pub const N_TREES: usize = 100;
pub const MIN_TRAIN: usize = 10;
const RADIUS: usize = 2;

fn mix(a: u64, b: u64) -> u64 {
    let mut h = a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

fn el_hash(el: &str) -> u64 {
    el.bytes().fold(17u64, |h, b| mix(h, b as u64))
}

/// Sorted set bits of the fingerprint of a SMILES (the largest piece of a mixture); `None` when it does not parse.
pub fn morgan(smiles: &str) -> Option<Vec<u16>> {
    let piece = smiles.split('.').max_by_key(|p| p.len())?;
    let m = Mol::parse(piece).ok()?;
    let n = m.atoms.len();
    let mut nb: Vec<Vec<(usize, u64)>> = vec![Vec::new(); n];
    let on_ring = m.ring_bonds();
    let mut ring_atom = vec![false; n];
    for (k, b) in m.bonds.iter().enumerate() {
        let code = (b.order * 2.0) as u64;
        nb[b.a].push((b.b, code));
        nb[b.b].push((b.a, code));
        if on_ring[k] {
            ring_atom[b.a] = true;
            ring_atom[b.b] = true;
        }
    }
    let mut id: Vec<u64> = (0..n)
        .map(|i| {
            let a = &m.atoms[i];
            let mut h = el_hash(&a.el);
            for v in [
                nb[i].len() as u64,
                a.h as u64,
                (a.charge + 8) as u64,
                ring_atom[i] as u64,
                a.arom as u64,
            ] {
                h = mix(h, v);
            }
            h
        })
        .collect();
    let mut bits: Vec<u16> = id.iter().map(|h| (h % N_BITS as u64) as u16).collect();
    for _ in 0..RADIUS {
        let next: Vec<u64> = (0..n)
            .map(|i| {
                let mut around: Vec<u64> = nb[i].iter().map(|&(j, c)| mix(id[j], c)).collect();
                around.sort_unstable();
                around.iter().fold(mix(id[i], 0xA5), |h, v| mix(h, *v))
            })
            .collect();
        id = next;
        bits.extend(id.iter().map(|h| (h % N_BITS as u64) as u16));
    }
    bits.sort_unstable();
    bits.dedup();
    Some(bits)
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

enum Node {
    Leaf(f64),
    Split {
        bit: u16,
        off: Box<Node>,
        on: Box<Node>,
    },
}

fn mean(y: &[f64], ix: &[usize]) -> f64 {
    ix.iter().map(|&i| y[i]).sum::<f64>() / ix.len() as f64
}

/// A regression tree on binary features: the bit with the largest drop of the sum of squares splits the node; no limit of depth (as scikit-learn's default).
fn grow(x: &[Vec<u16>], y: &[f64], ix: Vec<usize>) -> Node {
    let m = mean(y, &ix);
    if ix.len() < 2 || ix.iter().all(|&i| (y[i] - m).abs() < 1e-12) {
        return Node::Leaf(m);
    }
    // per bit: count and sum of y over the samples that have it
    let mut cnt = vec![0usize; N_BITS];
    let mut sum = vec![0.0f64; N_BITS];
    for &i in &ix {
        for &b in &x[i] {
            cnt[b as usize] += 1;
            sum[b as usize] += y[i];
        }
    }
    let (n, total) = (ix.len() as f64, ix.iter().map(|&i| y[i]).sum::<f64>());
    // the drop of the sum of squares is  s1²/n1 + s0²/n0 - total²/n  (bigger is better)
    let mut best: Option<(f64, usize)> = None;
    for b in 0..N_BITS {
        let (n1, s1) = (cnt[b] as f64, sum[b]);
        let n0 = n - n1;
        if n1 == 0.0 || n0 == 0.0 {
            continue;
        }
        let gain = s1 * s1 / n1 + (total - s1).powi(2) / n0 - total * total / n;
        if best.is_none_or(|(g, _)| gain > g + 1e-12) {
            best = Some((gain, b));
        }
    }
    let Some((_, bit)) = best else {
        return Node::Leaf(m);
    };
    let (on, off): (Vec<usize>, Vec<usize>) = ix
        .into_iter()
        .partition(|&i| x[i].binary_search(&(bit as u16)).is_ok());
    Node::Split {
        bit: bit as u16,
        off: Box::new(grow(x, y, off)),
        on: Box::new(grow(x, y, on)),
    }
}

fn walk(t: &Node, bits: &[u16]) -> f64 {
    match t {
        Node::Leaf(v) => *v,
        Node::Split { bit, off, on } => {
            if bits.binary_search(bit).is_ok() {
                walk(on, bits)
            } else {
                walk(off, bits)
            }
        }
    }
}

/// The trained forest.
pub struct RtModel {
    trees: Vec<Node>,
}

impl RtModel {
    /// Trains on `(smiles, rt)` rows; `None` with fewer than `MIN_TRAIN` rows that parse. The seed fixes the bootstrap draws (42 like the original).
    pub fn train(rows: &[(String, f64)], seed: u64) -> Option<RtModel> {
        let (x, y): (Vec<Vec<u16>>, Vec<f64>) = rows
            .iter()
            .filter_map(|(s, rt)| morgan(s).map(|b| (b, *rt)))
            .unzip();
        if x.len() < MIN_TRAIN {
            return None;
        }
        let mut rng = Rng(seed.max(1).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let trees = (0..N_TREES)
            .map(|_| {
                let ix: Vec<usize> = (0..x.len())
                    .map(|_| (rng.next() % x.len() as u64) as usize)
                    .collect();
                grow(&x, &y, ix)
            })
            .collect();
        Some(RtModel { trees })
    }

    /// Mean of the trees; `None` when the SMILES does not parse.
    pub fn predict(&self, smiles: &str) -> Option<f64> {
        let bits = morgan(smiles)?;
        Some(self.trees.iter().map(|t| walk(t, &bits)).sum::<f64>() / self.trees.len() as f64)
    }
}
