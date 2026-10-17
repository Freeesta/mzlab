//! The molecular graph of a SMILES as `hr/molgraph.py` sees it: its connected substructures after at most four cuts, and the SMILES of one of them (`fragtree.mask_smiles`).

use super::*;
use crate::sortc::SortC;

/// The heavy-atom graph of `molgraph.Mol`.
pub struct Graph {
    pub(crate) atoms: Vec<String>,
    pub(crate) h: Vec<i64>,
    pub(crate) bonds: Vec<(usize, usize, f32)>,
    pub(crate) adj: Vec<Vec<(usize, usize)>>,
}

impl Graph {
    pub fn parse(smiles: &str) -> Option<Graph> {
        let m = grafo::Mol::parse(smiles).ok()?;
        let bonds: Vec<(usize, usize, f32)> = m.bonds.iter().map(|b| (b.a, b.b, b.order)).collect();
        let mut adj = vec![Vec::new(); m.atoms.len()];
        for (k, &(a, b, _)) in bonds.iter().enumerate() {
            adj[a].push((b, k));
            adj[b].push((a, k));
        }
        Some(Graph {
            atoms: m.atoms.iter().map(|a| a.el.clone()).collect(),
            h: m.atoms.iter().map(|a| a.h as i64).collect(),
            bonds,
            adj,
        })
    }
}

impl Graph {
    /// `Mol.rings`: the smallest ring through every bond (BFS), duplicates removed, atoms sorted.
    pub(crate) fn rings(&self) -> Vec<Vec<usize>> {
        let (mut found, mut seen) = (Vec::new(), HashSet::new());
        for (k, &(a, b, _)) in self.bonds.iter().enumerate() {
            let mut prev: HashMap<usize, Option<usize>> = HashMap::from([(a, None)]);
            let (mut q, mut hit) = (vec![a], false);
            while !q.is_empty() && !hit {
                let mut nq = Vec::new();
                'outer: for &u in &q {
                    for &(v, kk) in &self.adj[u] {
                        if kk == k || prev.contains_key(&v) {
                            continue;
                        }
                        prev.insert(v, Some(u));
                        if v == b {
                            hit = true;
                            break 'outer;
                        }
                        nq.push(v);
                    }
                }
                q = nq;
            }
            if hit {
                let (mut ring, mut u) = (Vec::new(), Some(b));
                while let Some(x) = u {
                    ring.push(x);
                    u = prev[&x];
                }
                ring.sortc();
                if seen.insert(ring.clone()) {
                    found.push(ring);
                }
            }
        }
        found
    }
}

fn components(g: &Graph, mask: u64, cut: u128) -> Vec<u64> {
    let (mut seen, mut out) = (0u64, Vec::new());
    for s in 0..g.atoms.len() {
        if (mask >> s) & 1 == 0 || (seen >> s) & 1 == 1 {
            continue;
        }
        let (mut c, mut st) = (1u64 << s, vec![s]);
        while let Some(u) = st.pop() {
            for &(v, k) in &g.adj[u] {
                if (mask >> v) & 1 == 1 && (c >> v) & 1 == 0 && (cut >> k) & 1 == 0 {
                    c |= 1 << v;
                    st.push(v);
                }
            }
        }
        seen |= c;
        out.push(c);
    }
    out
}

/// `molgraph.SubStructures`: masks in the order they were found, the fewest cuts of each and their neutral formulas.
pub struct Subs {
    pub(crate) g: Graph,
    pub(crate) masks: Vec<u64>,
    cuts: Vec<i64>,
    pub(crate) els: Vec<&'static str>,
    pub(crate) f: Vec<Vec<i64>>,
}

impl Subs {
    /// `candidates(ion)`: substructures with the heavy atoms of the ion and |H + 1 - H_ion| <= 3, weighted exp(-3 (cuts - min) - |dH|).
    pub(crate) fn candidates(&self, ion: &[i64]) -> (Vec<usize>, Vec<f64>) {
        if ion.len() != self.els.len() {
            return (Vec::new(), Vec::new());
        }
        let ih = self.els.iter().position(|e| *e == "H").unwrap_or(0);
        let ok = |r: &Vec<i64>| {
            (0..self.els.len()).all(|i| i == ih || r[i] == ion[i])
                && (r[ih] + 1 - ion[ih]).abs() <= 3
        };
        let idx: Vec<usize> = (0..self.f.len()).filter(|&i| ok(&self.f[i])).collect();
        if idx.is_empty() {
            return (idx, Vec::new());
        }
        let cmin = idx.iter().map(|&i| self.cuts[i]).min().unwrap_or(0);
        let w: Vec<f64> = idx
            .iter()
            .map(|&i| {
                (-3.0 * (self.cuts[i] - cmin) as f64 - (self.f[i][ih] + 1 - ion[ih]).abs() as f64)
                    .exp()
            })
            .collect();
        let tot = w.iter().fold(0.0, |a, b| a + b);
        (idx, w.iter().map(|x| x / tot).collect())
    }

    /// `_top_smiles`: up to three different substructures, heaviest first.
    pub(crate) fn top_smiles(&self, idx: &[usize], w: &[f64]) -> Vec<(String, f64)> {
        let mut o: Vec<usize> = (0..w.len()).collect();
        o.sortc_by(|&a, &b| w[b].total_cmp(&w[a]));
        let mut out: Vec<(String, f64)> = Vec::new();
        for j in o {
            let s = mask_smiles(&self.g, self.masks[idx[j]]);
            if out.iter().any(|x| x.0 == s) {
                continue;
            }
            out.push((s, round(w[j], 3)));
            if out.len() == MAX_SMILES {
                break;
            }
        }
        out
    }
}

/// The connected pieces after up to four cuts, with the number of cuts that first made each one (in the order Python finds them); None above 400 000 states.
fn enumerate(g: &Graph, n: usize) -> Option<Vec<(u64, i64)>> {
    let inside: Vec<u64> = g
        .bonds
        .iter()
        .map(|&(a, b, _)| (1u64 << a) | (1u64 << b))
        .collect();
    let full = if n == 64 { u64::MAX } else { (1u64 << n) - 1 };
    let mut best: Vec<(u64, i64)> = vec![(full, 0)];
    let mut known: HashSet<u64> = HashSet::from([full]);
    let mut seen: HashSet<(u64, u128)> = HashSet::from([(full, 0)]);
    let mut frontier = vec![(full, 0u128)];
    for depth in 1..=4 {
        let mut nxt = Vec::new();
        for &(mask, cut) in &frontier {
            for (k, &(a, b, _)) in g.bonds.iter().enumerate() {
                if (cut >> k) & 1 == 1 || (mask >> a) & 1 == 0 || (mask >> b) & 1 == 0 {
                    continue;
                }
                let c2 = cut | (1u128 << k);
                for comp in components(g, mask, c2) {
                    let internal = (0..g.bonds.len())
                        .filter(|&kk| (c2 >> kk) & 1 == 1 && comp & inside[kk] == inside[kk])
                        .fold(0u128, |a, kk| a | (1u128 << kk));
                    if !seen.insert((comp, internal)) {
                        continue;
                    }
                    nxt.push((comp, internal));
                    if known.insert(comp) {
                        best.push((comp, depth));
                    }
                    if seen.len() > 400_000 {
                        return None;
                    }
                }
            }
        }
        frontier = nxt;
    }
    Some(best)
}

/// `substructures(mol, 4, 400000)`; None where the Python raises (more than 64 atoms, too many states, an element it does not know).
pub fn substructures(g: Graph) -> Option<Subs> {
    let n = g.atoms.len();
    if n == 0 || n > 64 || g.bonds.len() > 128 {
        return None;
    }
    let best = enumerate(&g, n)?;
    let mut counts: Vec<&str> = g.atoms.iter().map(String::as_str).collect();
    if g.h.iter().sum::<i64>() > 0 {
        counts.push("H");
    }
    let els = element_order(counts)?;
    let ih = els.iter().position(|e| *e == "H")?;
    let f = best
        .iter()
        .map(|&(m, _)| {
            let mut row = vec![0i64; els.len()];
            for (i, a) in g.atoms.iter().enumerate() {
                if (m >> i) & 1 == 1 {
                    if let Some(p) = els.iter().position(|e| e == a) {
                        row[p] += 1;
                    }
                }
            }
            row[ih] = (0..n).filter(|&i| (m >> i) & 1 == 1).map(|i| g.h[i]).sum();
            row
        })
        .collect();
    let (masks, cuts) = best.into_iter().unzip();
    Some(Subs {
        g,
        masks,
        cuts,
        els,
        f,
    })
}

/// The aromatic atoms of the mask (whole aromatic rings inside it) and the order written for every bond inside it.
pub(super) fn orders(g: &Graph, ins: &[bool]) -> (Vec<bool>, Vec<Option<f32>>) {
    let bond: Vec<bool> = g.bonds.iter().map(|&(a, b, _)| ins[a] && ins[b]).collect();
    let mut arom = vec![false; g.atoms.len()];
    for ring in g.rings() {
        if ring.iter().all(|&a| ins[a]) {
            let inr = |x: usize| ring.contains(&x);
            let ks =
                (0..g.bonds.len()).filter(|&k| bond[k] && inr(g.bonds[k].0) && inr(g.bonds[k].1));
            if ks.filter(|&k| g.bonds[k].2 == 1.5).count() >= ring.len() {
                ring.iter().for_each(|&a| arom[a] = true);
            }
        }
    }
    let mut matched = vec![false; g.atoms.len()];
    let order = g
        .bonds
        .iter()
        .enumerate()
        .map(|(k, &(a, b, o))| {
            if !bond[k] {
                None
            } else if o == 1.5 && arom[a] && arom[b] {
                Some(1.5)
            } else if o == 1.5 && !matched[a] && !matched[b] {
                matched[a] = true;
                matched[b] = true;
                Some(2.0)
            } else if o == 1.5 {
                Some(1.0)
            } else {
                Some(o)
            }
        })
        .collect();
    (arom, order)
}

/// The writer of `mask_smiles`: the DFS tree, the ring closures and the text.
pub(super) struct Writer<'a> {
    pub(super) g: &'a Graph,
    pub(super) arom: Vec<bool>,
    pub(super) order: Vec<Option<f32>>,
    pub(super) adj: Vec<Vec<(usize, usize)>>,
    pub(super) seen: Vec<bool>,
    pub(super) kids: Vec<Vec<(usize, usize)>>,
    pub(super) rings: Vec<Vec<(usize, usize, bool)>>,
    pub(super) closed: HashSet<usize>,
    pub(super) nring: usize,
}

impl Writer<'_> {
    fn sym(&self, k: usize) -> &'static str {
        let (a, b, _) = self.g.bonds[k];
        match self.order[k] {
            Some(2.0) => "=",
            Some(3.0) => "#",
            Some(o) if o == 1.0 && self.arom[a] && self.arom[b] => "-",
            _ => "",
        }
    }

    fn tok(&self, i: usize) -> String {
        let el = &self.g.atoms[i];
        let h = self.g.h[i];
        let e = if self.arom[i] {
            el.to_lowercase()
        } else {
            el.clone()
        };
        let hs = match h {
            1 => "H".to_string(),
            x if x > 1 => format!("H{x}"),
            _ => String::new(),
        };
        format!("[{e}{hs}]")
    }
}

impl Writer<'_> {
    fn dfs(&mut self, i: usize, frm: Option<usize>) {
        self.seen[i] = true;
        for q in 0..self.adj[i].len() {
            let (j, k) = self.adj[i][q];
            if Some(k) == frm {
                continue;
            }
            if !self.seen[j] {
                self.kids[i].push((j, k));
                self.dfs(j, Some(k));
            } else if !self.closed.contains(&k) && !self.kids[j].iter().any(|x| x.1 == k) {
                self.closed.insert(k);
                self.nring += 1;
                self.rings[j].push((self.nring, k, true));
                self.rings[i].push((self.nring, k, false));
            }
        }
    }
}

fn gen(w: &Writer, i: usize) -> String {
    let mut s = w.tok(i);
    for &(n, k, opening) in &w.rings[i] {
        if opening {
            s += w.sym(k);
        }
        s += &if n < 10 {
            n.to_string()
        } else {
            format!("%{n}")
        };
    }
    let parts: Vec<String> = w.kids[i]
        .iter()
        .map(|&(j, k)| w.sym(k).to_string() + &gen(w, j))
        .collect();
    if let Some((last, rest)) = parts.split_last() {
        for p in rest {
            s += &format!("({p})");
        }
        s += last;
    }
    s
}

/// `mask_smiles(mol, mask)`: the SMILES of a substructure, every atom in brackets with its hydrogens.
pub fn mask_smiles(g: &Graph, mask: u64) -> String {
    let n = g.atoms.len();
    let ins: Vec<bool> = (0..n).map(|i| (mask >> i) & 1 == 1).collect();
    let (arom, order) = orders(g, &ins);
    let mut adj = vec![Vec::new(); n];
    for (k, &(a, b, _)) in g.bonds.iter().enumerate() {
        if ins[a] && ins[b] {
            adj[a].push((b, k));
            adj[b].push((a, k));
        }
    }
    let mut w = Writer {
        g,
        arom,
        order,
        adj,
        seen: vec![false; n],
        kids: vec![Vec::new(); n],
        rings: vec![Vec::new(); n],
        closed: HashSet::new(),
        nring: 0,
    };
    let mut out = Vec::new();
    for i in (0..n).filter(|&i| ins[i]) {
        if !w.seen[i] {
            w.dfs(i, None);
            out.push(gen(&w, i));
        }
    }
    out.join(".")
}
