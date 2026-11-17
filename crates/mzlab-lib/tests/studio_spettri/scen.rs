//! Scenarios E0-E6 of the spectral study: a base library of random compounds at CE 35 plus the entries and queries each scenario needs, with the
//! ground truth of every query (`docs/agenti/studio-spettri.md`). The same seed gives the same library and the same queries.

use super::r#gen::*;

/// A library entry: neutral formula, fragmentation tree and its spectrum at CE 35.
#[derive(Clone, Debug)]
pub struct Entry {
    pub neutral: F,
    pub tree: Tree,
    pub peaks: Peaks,
}

/// A query with its truth: the library entry it should find (`None` for a trap) and its subgroup.
#[derive(Clone, Debug)]
pub struct Q {
    pub peaks: Peaks,
    pub truth: Option<usize>,
    pub group: String,
    /// Ion formula of the compound measured (the interferent is not part of it).
    pub ion: F,
    /// E5: the two mass shifts of the analogue (Da), else empty.
    pub deltas: Vec<f64>,
}

pub struct Scen {
    pub lib: Vec<Entry>,
    pub queries: Vec<Q>,
    /// Entries of the base library (the first `n_base` of `lib`).
    pub n_base: usize,
}

pub struct Ctx {
    seed: u64,
    k: u64,
    scaffolds: Vec<F>,
    pub lib: Vec<Entry>,
}

const CAP: usize = 25;
pub const SCENARIOS: [&str; 7] = ["e0", "e1", "e2", "e3", "e4", "e5", "e6"];

impl Ctx {
    /// The base library: `n_lib` compounds with [M+H]+ 120-700, 30 % of them built on one of 40 shared scaffold ions.
    pub fn new(seed: u64, n_lib: usize) -> Ctx {
        let mut r = R::new(seed, u64::MAX);
        let scaffolds = (0..40)
            .map(|_| add(&neutral_in(&mut r, 80.0, 200.0), &H1))
            .collect();
        let mut c = Ctx {
            seed,
            k: 0,
            scaffolds,
            lib: Vec::new(),
        };
        for i in 0..n_lib {
            // one stream per entry: a subsample keeps the same entries
            let mut r = R::new(seed, i as u64);
            let (neutral, tree) = c.compound(&mut r, 120.0, 700.0);
            let peaks = measure(&mut r, &tree, 35.0);
            c.lib.push(Entry {
                neutral,
                tree,
                peaks,
            });
        }
        c
    }
    pub fn rng(&mut self) -> R {
        self.k += 1;
        R::new(self.seed, (1 << 40) | self.k)
    }
    /// A random compound in lo..hi (m/z of [M+H]+) with at least 2 fragments.
    pub fn compound(&self, r: &mut R, lo: f64, hi: f64) -> (F, Tree) {
        loop {
            let (neutral, t) = if r.unit() < 0.3 {
                match self.on_scaffold(r, lo, hi) {
                    Some(x) => x,
                    None => continue,
                }
            } else {
                let n = neutral_in(r, lo, hi);
                (n, random_tree(r, add(&n, &H1), CAP))
            };
            if t.nodes.len() >= 3 {
                return (neutral, t);
            }
        }
    }
    fn on_scaffold(&self, r: &mut R, lo: f64, hi: f64) -> Option<(F, Tree)> {
        let s = r.pick(&self.scaffolds);
        let ms = ion_mz(&s);
        if hi - ms < 60.0 {
            return None;
        }
        let h = mass(&H1);
        let x = neutral_in(r, (lo - ms).max(30.0) + h, hi - ms + h);
        let ion = add(&s, &x);
        let neutral = sub(&ion, &H1);
        let mz = ion_mz(&ion);
        if mz < lo || mz >= hi || rdbe2(&neutral) < 0 {
            return None;
        }
        let mut t = Tree::root(ion);
        t.push(r, 0, s);
        Some((neutral, grow_tree(r, t, CAP)))
    }
    /// A new compound with the formula `neutral` and its own tree.
    pub fn isomer(&self, r: &mut R, neutral: &F) -> Tree {
        loop {
            let t = random_tree(r, add(neutral, &H1), CAP);
            if t.nodes.len() >= 3 {
                return t;
            }
        }
    }
    pub fn push(&mut self, neutral: F, tree: Tree, peaks: Peaks) -> usize {
        self.lib.push(Entry {
            neutral,
            tree,
            peaks,
        });
        self.lib.len() - 1
    }
}

pub fn q(peaks: Peaks, truth: Option<usize>, group: String, ion: F) -> Q {
    Q {
        peaks,
        truth,
        group,
        ion,
        deltas: Vec::new(),
    }
}

/// The base entry that query `k` of `n` replicates.
pub fn pick(n_base: usize, k: usize, n: usize) -> usize {
    (k * n_base / n.max(1) + k % 7) % n_base
}

pub fn e0(c: &mut Ctx, n_base: usize, n: usize) -> Vec<Q> {
    (0..n)
        .map(|k| {
            let id = pick(n_base, k, n);
            let mut r = c.rng();
            let e = &c.lib[id];
            q(
                measure(&mut r, &e.tree, 35.0),
                Some(id),
                "e0".into(),
                e.tree.nodes[0].f,
            )
        })
        .collect()
}

/// A small tree: 2-3 fragments of a [M+H]+ below 200.
fn small_tree(r: &mut R, ion: F) -> Option<Tree> {
    let mut t = Tree::root(ion);
    let want = r.int(2, 3) as usize;
    for _ in 0..20 {
        if t.nodes.len() > want {
            break;
        }
        let p = r.int(0, t.nodes.len() as i32 - 1) as usize;
        t.grow(r, p);
    }
    (t.nodes.len() >= 3).then_some(t)
}

/// Library spectrum of a small compound: its fragments plus 5 weak peaks at 1-2 % (minor ions the query does not show).
fn small_entry(r: &mut R, t: &Tree) -> Peaks {
    let mut p = measure(r, t, 35.0);
    for _ in 0..5 {
        p.mz.push(r.range(50.0, p.prec - 2.0));
        p.it.push(r.range(0.01, 0.02) * base(&p));
        p.src.push(2);
    }
    p
}

/// E1: a compound < 200 Da with 2-3 fragments and two isobaric (same formula) competitors in the library; a trap leaves the compound out.
pub fn small(c: &mut Ctx, trap: bool, group: &str) -> Q {
    let mut r = c.rng();
    let (neutral, t) = loop {
        let n = neutral_in(&mut r, 120.0, 200.0);
        if let Some(t) = small_tree(&mut r, add(&n, &H1)) {
            break (n, t);
        }
    };
    // competitors: an isomer sharing all fragments but one (with its own kinetics) and an unrelated isomer
    let other = |r: &mut R| loop {
        if let Some(ti) = small_tree(r, add(&neutral, &H1)) {
            break ti;
        }
    };
    let near = match variant(&mut r, &t, &[t.nodes.len() - 1], &t) {
        Some(mut v) => {
            v.nodes
                .iter_mut()
                .skip(1)
                .for_each(|n| (n.a, n.e0) = edge(&mut r));
            v
        }
        None => other(&mut r),
    };
    let far = other(&mut r);
    for ti in [near, far] {
        let p = small_entry(&mut r, &ti);
        c.push(neutral, ti, p);
    }
    let truth = (!trap).then(|| {
        let p = small_entry(&mut r, &t);
        c.push(neutral, t.clone(), p)
    });
    q(measure(&mut r, &t, 35.0), truth, group.into(), t.nodes[0].f)
}

/// E2 spectrum of one isomer: base peak 90 % of the intensity, the two diagnostic ions 3 % each, the other shared fragments 4 % in their kinetic ratio.
fn design(t: &Tree, b: usize, shared: &[usize], diag: &[usize], it: &[f64]) -> Peaks {
    let mut v = vec![0.0; t.nodes.len()];
    let tot: f64 = shared.iter().map(|&s| it[s]).sum::<f64>().max(1e-12);
    v[b] = 0.90;
    for &s in shared {
        v[s] = 0.04 * it[s] / tot;
    }
    for &d in diag {
        v[d] = 0.03;
    }
    exact(t, &v)
}

/// The tree with the ions `diag` replaced by other sub-formulas of the root (one loss from another ion), new to the tree and to `avoid`
/// (`None` after 200 draws). The parent links are not updated: only the formulas and the kinetics of the edges are read afterwards.
fn variant(r: &mut R, t: &Tree, diag: &[usize], avoid: &Tree) -> Option<Tree> {
    let mut v = t.clone();
    for &d in diag {
        let mut found = false;
        for _ in 0..200 {
            let p = r.int(0, t.nodes.len() as i32 - 1) as usize;
            let f = sub(&t.nodes[p].f, &r.pick(&LOSSES));
            if !diag.contains(&p)
                && valid_ion(&f)
                && ion_mz(&f) >= 50.0
                && v.nodes.iter().chain(&avoid.nodes).all(|n| n.f != f)
            {
                v.nodes[d].f = f;
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }
    Some(v)
}

/// E2: positional isomers A and B in the library (shared base peak, 2 diagnostic ions each); the query is A, or a third isomer for a trap.
pub fn isomers(c: &mut Ctx, trap: bool, group: &str) -> Q {
    let mut r = c.rng();
    let (neutral, t, it, b, others, diag, tb, tc) = loop {
        let n = neutral_in(&mut r, 150.0, 500.0);
        let t = random_tree(&mut r, add(&n, &H1), CAP);
        if t.nodes.len() < 5 {
            continue;
        }
        let it = intensities(&t, 35.0);
        let k = t.nodes.len();
        let b = (1..k)
            .max_by(|&x, &y| it[x].total_cmp(&it[y]).then(y.cmp(&x)))
            .unwrap();
        let mut others: Vec<usize> = (1..k).filter(|&x| x != b).collect();
        let d1 = others.remove(r.int(0, others.len() as i32 - 1) as usize);
        let d2 = others.remove(r.int(0, others.len() as i32 - 1) as usize);
        let diag = [d1, d2];
        let Some(tb) = variant(&mut r, &t, &diag, &t) else {
            continue;
        };
        let Some(tc) = variant(&mut r, &tb, &diag, &t) else {
            continue;
        };
        break (n, t, it, b, others, diag, tb, tc);
    };
    let pa = noisy(&mut r, &design(&t, b, &others, &diag, &it));
    let pb = noisy(&mut r, &design(&tb, b, &others, &diag, &it));
    let ia = c.push(neutral, t.clone(), pa);
    c.push(neutral, tb.clone(), pb);
    let tq = if trap { tc } else { t };
    let p = noisy(&mut r, &design(&tq, b, &others, &diag, &it));
    q(p, (!trap).then_some(ia), group.into(), tq.nodes[0].f)
}

/// A compound whose [M+H]+ lies 0.02-0.5 Da from `prec` (co-isolated, outside the 10 ppm window).
fn isobaric(c: &Ctx, r: &mut R, prec: f64) -> Tree {
    loop {
        let s = if r.unit() < 0.5 { 1.0 } else { -1.0 };
        let t = prec + s * r.range(0.02, 0.5);
        if let Some(f) = neutral_near(r, t) {
            let d = (ion_mz(&add(&f, &H1)) - prec).abs();
            if (0.02..=0.5).contains(&d) {
                return c.isomer(r, &f);
            }
        }
    }
}

pub const ETAS: [f64; 6] = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6];

/// E3: (1 - eta) Q + eta I with an isobaric or an isomeric interferent absent from the library; a trap replaces Q by an isomer of entry `id`.
pub fn chimera(c: &mut Ctx, id: usize, eta: f64, isomer: bool, trap: bool, group: &str) -> Q {
    let mut r = c.rng();
    let neutral = c.lib[id].neutral;
    let (tq, truth) = if trap {
        (c.isomer(&mut r, &neutral), None)
    } else {
        (c.lib[id].tree.clone(), Some(id))
    };
    let prec = ion_mz(&tq.nodes[0].f);
    let ti = if isomer {
        c.isomer(&mut r, &neutral)
    } else {
        isobaric(c, &mut r, prec)
    };
    let p = mix(
        &measure(&mut r, &tq, 35.0),
        &measure(&mut r, &ti, 35.0),
        eta,
    );
    q(p, truth, group.into(), tq.nodes[0].f)
}

pub const CES: [f64; 4] = [10.0, 20.0, 50.0, 65.0];

pub fn e4(c: &mut Ctx, n_base: usize, n: usize) -> Vec<Q> {
    (0..n)
        .map(|k| {
            let id = pick(n_base, k, n);
            let ce = CES[k % 4];
            let mut r = c.rng();
            let t = &c.lib[id].tree;
            q(
                measure(&mut r, t, ce),
                Some(id),
                format!("ce={ce}"),
                t.nodes[0].f,
            )
        })
        .collect()
}

/// E6: per style of E0-E3, half the queries in the library and half traps (same formula as a library entry, another tree, absent).
pub fn e6(c: &mut Ctx, n_base: usize, n: usize) -> Vec<Q> {
    let mut out = Vec::new();
    for k in 0..n {
        let (style, trap) = (k % 4, (k / 4) % 2 == 1);
        let g = format!(
            "{}/{}",
            ["e0", "e1", "e2", "e3"][style],
            if trap { "trap" } else { "true" }
        );
        let id = pick(n_base, k, n);
        out.push(match style {
            0 if trap => {
                let mut r = c.rng();
                let t = c.isomer(&mut r, &c.lib[id].neutral);
                q(measure(&mut r, &t, 35.0), None, g, t.nodes[0].f)
            }
            0 => {
                let mut r = c.rng();
                let t = &c.lib[id].tree;
                q(measure(&mut r, t, 35.0), Some(id), g, t.nodes[0].f)
            }
            1 => small(c, trap, &g),
            2 => isomers(c, trap, &g),
            _ => chimera(c, id, 0.3, false, trap, &g),
        });
    }
    out
}

/// The library and the queries of scenario `name` (`SCENARIOS`).
pub fn build(name: &str, seed: u64, n_lib: usize, n: usize) -> Scen {
    let mut c = Ctx::new(seed, n_lib);
    let queries = match name {
        "e0" => e0(&mut c, n_lib, n),
        "e1" => (0..n).map(|_| small(&mut c, false, "e1")).collect(),
        "e2" => (0..n).map(|_| isomers(&mut c, false, "e2")).collect(),
        "e3" => (0..n)
            .map(|k| {
                let (eta, iso) = (ETAS[k % 6], (k / 6) % 2 == 1);
                let g = format!("eta={eta:.1}/{}", if iso { "isomer" } else { "isobaric" });
                let id = pick(n_lib, k, n);
                chimera(&mut c, id, eta, iso, false, &g)
            })
            .collect(),
        "e4" => e4(&mut c, n_lib, n),
        "e5" => super::analogue::e5(&mut c, n_lib, n),
        "e6" => e6(&mut c, n_lib, n),
        _ => panic!("unknown scenario {name}"),
    };
    Scen {
        lib: c.lib,
        queries,
        n_base: n_lib,
    }
}
