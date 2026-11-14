//! Synthetic MS/MS of the spectral study (`docs/agenti/studio-spettri.md`): CHNO(P,S,Cl) [M+H]+ precursors, fragmentation trees of neutral losses
//! with kinetic edges (A, E0), so the branching ratios move with the collision energy, and the noise of a real acquisition (2 ppm, 8 % lognormal,
//! weak noise peaks). Every random draw comes from `decoy::XorShift` with a fixed seed, masses from `mzlab_core::elements`, exp/ln/cos
//! from `libm` (the same bits on every target).

use mzlab_core::elements::{mass_of, ELECTRON};
use mzlab_lib::decoy::XorShift;

pub const ELEMS: [&str; 7] = ["C", "H", "N", "O", "P", "S", "Cl"];
/// Element counts in the order of `ELEMS`.
pub type F = [i32; 7];

/// Neutral losses of the fragmentation trees (closed-shell molecules, so an even-electron ion stays even-electron).
pub const LOSSES: [F; 18] = [
    [0, 2, 0, 1, 0, 0, 0], // H2O
    [1, 0, 0, 1, 0, 0, 0], // CO
    [1, 0, 0, 2, 0, 0, 0], // CO2
    [0, 3, 1, 0, 0, 0, 0], // NH3
    [1, 2, 0, 1, 0, 0, 0], // CH2O
    [2, 4, 0, 0, 0, 0, 0], // C2H4
    [1, 1, 1, 0, 0, 0, 0], // HCN
    [1, 4, 0, 1, 0, 0, 0], // CH3OH
    [2, 2, 0, 1, 0, 0, 0], // C2H2O
    [0, 1, 0, 0, 0, 0, 1], // HCl
    [0, 0, 0, 2, 0, 1, 0], // SO2
    [1, 4, 0, 0, 0, 0, 0], // CH4
    [3, 6, 0, 0, 0, 0, 0], // C3H6
    [2, 2, 0, 0, 0, 0, 0], // C2H2
    [1, 2, 0, 2, 0, 0, 0], // CH2O2
    [0, 2, 0, 0, 0, 1, 0], // H2S
    [4, 8, 0, 0, 0, 0, 0], // C4H8
    [2, 3, 1, 0, 0, 0, 0], // C2H3N
];

pub const H1: F = [0, 1, 0, 0, 0, 0, 0];

pub fn add(a: &F, b: &F) -> F {
    std::array::from_fn(|i| a[i] + b[i])
}

pub fn sub(a: &F, b: &F) -> F {
    std::array::from_fn(|i| a[i] - b[i])
}

pub fn mass(f: &F) -> f64 {
    let mut m = 0.0;
    for (i, e) in ELEMS.iter().enumerate() {
        m += f[i] as f64 * mass_of(e).unwrap();
    }
    m
}

/// m/z of a singly charged cation whose formula (protons included) is `f`.
pub fn ion_mz(f: &F) -> f64 {
    mass(f) - ELECTRON
}

/// Hill order (C, H, then alphabetical), the way the library metadata write it.
pub fn text(f: &F) -> String {
    let mut s = String::new();
    for i in [0, 1, 6, 2, 3, 4, 5] {
        match f[i] {
            0 => {}
            1 => s.push_str(ELEMS[i]),
            n => s.push_str(&format!("{}{n}", ELEMS[i])),
        }
    }
    s
}

/// Twice the rings-plus-double-bonds of `f` (an [M+H]+ ion gives an odd value).
pub fn rdbe2(f: &F) -> i32 {
    2 * f[0] - f[1] - f[6] + f[2] + f[4] + 2
}

/// A fragment ion is kept only with carbon, hydrogen, no negative count and RDBE >= -0.5.
pub fn valid_ion(f: &F) -> bool {
    f.iter().all(|&n| n >= 0) && f[0] >= 1 && f[1] >= 1 && rdbe2(f) >= -1
}

pub fn contains(a: &F, b: &F) -> bool {
    (0..7).all(|i| a[i] >= b[i])
}

/// The random numbers of the study: xorshift64* plus the few distributions it needs.
pub struct R(pub XorShift);

impl R {
    /// An independent stream per (seed, stream): the streams of two objects never depend on how many others were drawn.
    pub fn new(seed: u64, stream: u64) -> R {
        let mut z = seed ^ stream.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        R(XorShift::new(z ^ (z >> 31)))
    }
    pub fn unit(&mut self) -> f64 {
        self.0.unit()
    }
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
    /// Uniform integer in lo..=hi.
    pub fn int(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.0.next_u64() % (hi - lo + 1) as u64) as i32
    }
    pub fn pick<T: Copy>(&mut self, v: &[T]) -> T {
        v[(self.0.next_u64() % v.len() as u64) as usize]
    }
    /// Standard normal (Box-Muller).
    pub fn normal(&mut self) -> f64 {
        let u = self.unit().max(1e-300);
        let v = self.unit();
        libm::sqrt(-2.0 * libm::log(u)) * libm::cos(std::f64::consts::TAU * v)
    }
}

/// A neutral, closed-shell formula with RDBE >= 0 (Senior: even valence sum) whose [M+H]+ falls near `target`.
pub fn neutral_near(r: &mut R, target: f64) -> Option<F> {
    let n = r.int(0, 4) * (r.unit() < 0.7) as i32;
    let o = r.int(0, 8);
    let p = (r.unit() < 0.05) as i32;
    let s = if r.unit() < 0.1 { r.int(1, 2) } else { 0 };
    let cl = if r.unit() < 0.12 { r.int(1, 2) } else { 0 };
    let rho = r.range(0.05, 0.5);
    let het = [0, 0, n, o, p, s, cl];
    let fixed = mass(&het) + mass(&H1) * (3 + n + p - cl) as f64;
    let c = ((target - fixed) / (2.0 * mass(&H1) * (1.0 - rho) + mass(&[1, 0, 0, 0, 0, 0, 0])))
        .round() as i32;
    if c < 2 || o > c + 2 || n > c {
        return None;
    }
    let rd = (rho * c as f64).round() as i32;
    let h = 2 * c + 2 + n + p - cl - 2 * rd;
    let f = [c, h, n, o, p, s, cl];
    (h >= 1 && rdbe2(&f) >= 0).then_some(f)
}

/// A neutral formula whose [M+H]+ lies in lo..hi.
pub fn neutral_in(r: &mut R, lo: f64, hi: f64) -> F {
    loop {
        let t = r.range(lo, hi);
        if let Some(f) = neutral_near(r, t) {
            let mz = ion_mz(&add(&f, &H1));
            if mz >= lo && mz < hi {
                return f;
            }
        }
    }
}

/// One fragment ion; the edge from its parent carries the Arrhenius pair (A, E0).
#[derive(Clone, Debug)]
pub struct Node {
    pub f: F,
    pub parent: Option<usize>,
    pub a: f64,
    pub e0: f64,
}

#[derive(Clone, Debug)]
pub struct Tree {
    pub nodes: Vec<Node>,
}

pub fn edge(r: &mut R) -> (f64, f64) {
    (libm::pow(10.0, r.range(0.5, 3.0)), r.range(0.9, 2.6))
}

impl Tree {
    pub fn root(f: F) -> Tree {
        let n = Node {
            f,
            parent: None,
            a: 0.0,
            e0: 0.0,
        };
        Tree { nodes: vec![n] }
    }
    pub fn children(&self, i: usize) -> Vec<usize> {
        (0..self.nodes.len())
            .filter(|&k| self.nodes[k].parent == Some(i))
            .collect()
    }
    pub fn depth(&self, mut i: usize) -> usize {
        let mut d = 0;
        while let Some(p) = self.nodes[i].parent {
            d += 1;
            i = p;
        }
        d
    }
    /// Adds the child `f` of `parent` unless the tree already has that ion.
    pub fn push(&mut self, r: &mut R, parent: usize, f: F) -> Option<usize> {
        if self.nodes.iter().any(|n| n.f == f) {
            return None;
        }
        let (a, e0) = edge(r);
        self.nodes.push(Node {
            f,
            parent: Some(parent),
            a,
            e0,
        });
        Some(self.nodes.len() - 1)
    }
    /// A random child of `i` by one loss of `LOSSES` (up to 8 draws), at m/z >= 50.
    pub fn grow(&mut self, r: &mut R, i: usize) -> Option<usize> {
        for _ in 0..8 {
            let c = sub(&self.nodes[i].f, &r.pick(&LOSSES));
            if valid_ion(&c)
                && ion_mz(&c) >= 50.0
                && let Some(k) = self.push(r, i, c)
            {
                return Some(k);
            }
        }
        None
    }
}

/// Depth 2-5, branching 1-3, at most `cap` fragments; the root always branches, deeper ions with probability 0.6.
pub fn random_tree(r: &mut R, root: F, cap: usize) -> Tree {
    grow_tree(r, Tree::root(root), cap)
}

/// `random_tree` continued from the ions `t` already has.
pub fn grow_tree(r: &mut R, mut t: Tree, cap: usize) -> Tree {
    let max_d = r.int(2, 5) as usize;
    let mut k = 0;
    while k < t.nodes.len() {
        let d = t.depth(k);
        if d < max_d && (k == 0 || r.unit() < 0.6) {
            for _ in 0..r.int(1, 3) {
                if t.nodes.len() > cap {
                    break;
                }
                t.grow(r, k);
            }
        }
        k += 1;
    }
    t
}

/// Effective temperature of the ions at a collision energy (eV-like units): it sets k = A exp(-E0 / T).
pub fn temperature(ce: f64) -> f64 {
    0.15 + 0.012 * ce
}

/// Noise-free intensities of the tree at `ce`: an ion decays over unit time into its children in the ratio of their rates; what is left is seen.
pub fn intensities(t: &Tree, ce: f64) -> Vec<f64> {
    let temp = temperature(ce);
    let n = t.nodes.len();
    let mut pop = vec![0.0; n];
    let mut seen = vec![0.0; n];
    pop[0] = 1.0;
    // children always come after their parent, so one forward pass is enough
    for i in 0..n {
        let ch = t.children(i);
        let k: Vec<f64> = ch
            .iter()
            .map(|&c| t.nodes[c].a * libm::exp(-t.nodes[c].e0 / temp))
            .collect();
        let tot: f64 = k.iter().sum();
        let stay = if ch.is_empty() { 1.0 } else { libm::exp(-tot) };
        seen[i] = pop[i] * stay;
        for (j, &c) in ch.iter().enumerate() {
            pop[c] += pop[i] * (1.0 - stay) * k[j] / tot;
        }
    }
    seen
}

/// A centroided spectrum: precursor m/z and the peaks (any order).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Peaks {
    pub prec: f64,
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
    /// Origin of each peak (ground truth): 0 fragment of the compound, 1 fragment of an interferent, 2 noise.
    pub src: Vec<u8>,
}

/// Peaks at exact m/z: base peak 1 among the fragments (the precursor ion keeps its share but the cleaning removes it).
pub fn exact(t: &Tree, it: &[f64]) -> Peaks {
    let mx = it[1..].iter().copied().fold(1e-12, f64::max);
    Peaks {
        prec: ion_mz(&t.nodes[0].f),
        mz: t.nodes.iter().map(|n| ion_mz(&n.f)).collect(),
        it: it.iter().map(|v| v / mx).collect(),
        src: vec![0; it.len()],
    }
}

/// Measurement noise: m/z 2 ppm (sigma), intensity lognormal 8 %, 0-5 noise peaks at 0.5-2 % of the base peak below the precursor.
pub fn noisy(r: &mut R, p: &Peaks) -> Peaks {
    let mut q = Peaks {
        prec: p.prec,
        ..Default::default()
    };
    for k in 0..p.mz.len() {
        q.mz.push(p.mz[k] * (1.0 + 2e-6 * r.normal()));
        q.it.push(p.it[k] * libm::exp(0.08 * r.normal()));
        q.src.push(p.src[k]);
    }
    for _ in 0..r.int(0, 5) {
        q.mz.push(r.range(50.0, (p.prec - 2.0).max(51.0)));
        q.it.push(r.range(0.005, 0.02));
        q.src.push(2);
    }
    q
}

/// The tree measured at `ce` with noise.
pub fn measure(r: &mut R, t: &Tree, ce: f64) -> Peaks {
    noisy(r, &exact(t, &intensities(t, ce)))
}

/// Base peak of the fragments: the highest peak the cleaning keeps (below the precursor - 1.6 Da).
pub fn base(p: &Peaks) -> f64 {
    let it = p.mz.iter().zip(&p.it).filter(|(m, _)| **m < p.prec - 1.6);
    it.map(|(_, v)| *v).fold(1e-12, f64::max)
}

/// (1 - eta) Q + eta I, each normalised to its fragment base peak first; the precursor stays the one of Q.
pub fn mix(q: &Peaks, i: &Peaks, eta: f64) -> Peaks {
    let (mq, mi) = (base(q), base(i));
    let mut out = Peaks {
        prec: q.prec,
        ..Default::default()
    };
    for k in 0..q.mz.len() {
        out.mz.push(q.mz[k]);
        out.it.push((1.0 - eta) * q.it[k] / mq);
        out.src.push(q.src[k]);
    }
    for k in 0..i.mz.len() {
        out.mz.push(i.mz[k]);
        out.it.push(eta * i.it[k] / mi);
        out.src.push(if i.src[k] == 0 { 1 } else { i.src[k] });
    }
    out
}
