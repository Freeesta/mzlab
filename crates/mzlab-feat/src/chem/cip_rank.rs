//! The hierarchical digraph of the CIP rules: a tree grown from one atom, where ring closures and multiple bonds give duplicate atoms, and the
//! breadth-first comparison of two branches rule by rule (1a atomic number, 1b root distance of duplicates, 2 mass number, 3 seqcis before
//! seqtrans, 4a stereogenic before not, 4b like before unlike, 5 R before S). Each rule is used on the whole digraph before the next one; the
//! order of exploring a sphere is the ranking the earlier rules gave. The auxiliary descriptor of a centre met in the digraph is found with
//! the same tree hung from that centre; the duplicate of a double bond in a mancude ring has the mean atomic number of the atoms it could
//! copy in the other Kekulé forms. Not done: rule 4c, rules 4b and 5 on double bonds.

use std::cmp::Ordering;

/// Rules in the order they are used: 1a, 1b, 2, 3, 4a, 4b, 5.
pub(crate) const RULES: usize = 7;
/// Index of rule 4b; it and rule 5 after it compare whole branches instead of node values.
const LIKE: usize = 5;
/// Largest digraph grown for one label; past it the label is left out.
const NODE_CAP: usize = 40_000;
/// Atom index of an implicit hydrogen or of a lone pair.
pub(crate) const NO_ATOM: usize = usize::MAX;

/// The molecule as the rules see it.
pub(crate) struct Chem {
    pub z: Vec<u16>,
    /// Twice the mass number, plus one for natural abundance (which ranks above its lightest common isotope, below the next).
    pub mass: Vec<u16>,
    pub h: Vec<u8>,
    pub adj: Vec<Vec<(usize, u8)>>,
    /// Descriptors of the double bonds used by rule 3 (atoms low first; Z 2, E 1).
    pub db: Vec<(usize, usize, u8)>,
    /// The defined centres as written: neighbours in ref order, the hydrogen (1) or lone pair (0) after them, the parity.
    pub tetra: Vec<Option<Centre>>,
    /// For an atom whose one double bond is in a mancude ring: twelve times the mean atomic number of its possible partners, and their mass (0
    /// when they differ), given to the duplicate of that bond.
    pub manc: Vec<Option<(u16, u16)>>,
}

/// A defined tetrahedral centre of the record.
#[derive(Clone)]
pub(crate) struct Centre {
    pub refs: Vec<usize>,
    pub extra: Option<u16>,
    pub cw: bool,
}

/// R (2) or S (1) of a centre whose ligands, given as positions in `refs` + `extra`, are ranked highest first.
pub(crate) fn handed(order: &[usize], cw: bool) -> u8 {
    let mut perm = [order[3], order[0], order[1], order[2]];
    let mut swaps = 0;
    for i in 0..perm.len() {
        while perm[i] != i {
            let j = perm[i];
            perm.swap(i, j);
            swaps += 1;
        }
    }
    // Seen from the lowest ligand the other three turn clockwise: S.
    if cw != (swaps % 2 == 1) {
        1
    } else {
        2
    }
}

#[derive(Clone)]
struct Node {
    atom: usize,
    z: u16,
    /// Twelve times the atomic number used by rule 1a (a mean for a mancude duplicate).
    z12: u16,
    mass: u16,
    dup: bool,
    /// The node a duplicate copies (itself for any other node): rule 1b measures its distance from the root.
    orig: usize,
    /// A ring-closure duplicate (not one made by a multiple bond): the only kind rule 1b ranks.
    ring: bool,
    depth: u16,
    parent: usize,
}

/// A cached ranking: (from, rule, root) and the nodes onward, highest first.
type Sorted = (usize, usize, usize, Vec<usize>);

/// An edge walked from `.0` into `.1`: the tree seen from one root is the set of edges pointing away from it.
type Arc = (usize, usize);

/// One digraph, grown on demand from its root (node 0).
pub(crate) struct Digraph<'a> {
    c: &'a Chem,
    nodes: Vec<Node>,
    /// Per node: its children once grown.
    kids: Vec<Option<Vec<usize>>>,
    /// Per node entered: (from, rule, root) and the nodes onward in ranked order.
    sorted: Vec<Vec<Sorted>>,
    /// Per node: its auxiliary descriptor plus one, 0 while not known.
    aux: Vec<u32>,
    pub over: bool,
}

impl<'a> Digraph<'a> {
    pub fn new(c: &'a Chem, root: usize) -> Self {
        let n = Node {
            atom: root,
            z: c.z[root],
            z12: 12 * c.z[root],
            mass: c.mass[root],
            dup: false,
            orig: 0,
            ring: false,
            depth: 0,
            parent: NO_ATOM,
        };
        Digraph {
            c,
            nodes: vec![n],
            kids: vec![None],
            sorted: vec![Vec::new()],
            aux: vec![0],
            over: false,
        }
    }

    fn push(&mut self, parent: usize, atom: usize, dup: Option<usize>) -> usize {
        if self.nodes.len() >= NODE_CAP {
            self.over = true;
        }
        let i = self.nodes.len();
        let (z, mass) = match atom {
            NO_ATOM => (0, 0),
            a => (self.c.z[a], self.c.mass[a]),
        };
        self.nodes.push(Node {
            atom,
            z,
            z12: 12 * z,
            mass,
            dup: dup.is_some(),
            orig: dup.unwrap_or(i),
            ring: false,
            depth: self.nodes[parent].depth + 1,
            parent,
        });
        self.kids.push(None);
        self.sorted.push(Vec::new());
        self.aux.push(0);
        i
    }

    /// An implicit hydrogen (`z` 1) or a lone pair (`z` 0) under `parent`.
    fn leaf(&mut self, parent: usize, z: u16) -> usize {
        let i = self.push(parent, NO_ATOM, None);
        self.nodes[i].z = z;
        self.nodes[i].z12 = 12 * z;
        self.nodes[i].mass = if z == 1 { 3 } else { 0 };
        i
    }

    /// The node of `atom` on the path from the root to `i`, if it is there.
    fn on_path(&self, mut i: usize, atom: usize) -> Option<usize> {
        while i != NO_ATOM {
            let n = &self.nodes[i];
            if n.atom == atom && !n.dup {
                return Some(i);
            }
            i = n.parent;
        }
        None
    }

    /// The children of node `i` in the tree hung from the root: neighbours (a duplicate for one already on the path), one duplicate per
    /// extra bond order, implicit hydrogens, and the lone pair of a defined centre with one.
    fn children(&mut self, i: usize) -> Vec<usize> {
        if let Some(k) = &self.kids[i] {
            return k.clone();
        }
        let n = self.nodes[i].clone();
        let mut out = Vec::new();
        if !n.dup && n.atom != NO_ATOM && !self.over {
            let mut parent_seen = false;
            let c = self.c;
            for &(y, o) in &c.adj[n.atom] {
                let orig = if n.parent != NO_ATOM && self.nodes[n.parent].atom == y && !parent_seen
                {
                    parent_seen = true;
                    n.parent
                } else {
                    let k = match self.on_path(i, y) {
                        Some(a) => {
                            let k = self.push(i, y, Some(a));
                            self.nodes[k].ring = true;
                            k
                        }
                        None => self.push(i, y, None),
                    };
                    out.push(k);
                    self.nodes[k].orig
                };
                for _ in 1..o {
                    let k = self.push(i, y, Some(orig));
                    if let (2, Some((z12, mass))) = (o, self.c.manc[n.atom]) {
                        (self.nodes[k].z12, self.nodes[k].mass) = (z12, mass);
                    }
                    out.push(k);
                }
            }
            for _ in 0..self.c.h[n.atom] {
                out.push(self.leaf(i, 1));
            }
            if self.c.tetra[n.atom]
                .as_ref()
                .is_some_and(|c| c.extra == Some(0))
            {
                out.push(self.leaf(i, 0));
            }
        }
        self.kids[i] = Some(out.clone());
        out
    }

    /// The ligand nodes of the root for the neighbours in `refs`, then its hydrogen (`extra` 1) or lone pair (`extra` 0).
    pub fn ligands(&mut self, refs: &[usize], extra: Option<u16>) -> Vec<usize> {
        let all = self.children(0);
        let find = |g: &Self, atom: usize, z: u16| {
            all.iter().copied().find(|&k| {
                g.nodes[k].atom == atom && !g.nodes[k].dup && (atom != NO_ATOM || g.nodes[k].z == z)
            })
        };
        let mut out: Vec<usize> = refs.iter().filter_map(|&r| find(self, r, 0)).collect();
        if let Some(z) = extra {
            let k = find(self, NO_ATOM, z).unwrap_or_else(|| self.leaf(0, z));
            out.push(k);
        }
        out
    }

    /// The nodes reached from `node` when it was entered from `from`: its children and its parent, less `from`.
    fn onward(&mut self, (from, node): Arc) -> Vec<usize> {
        let mut k = self.children(node);
        k.retain(|&x| x != from);
        let p = self.nodes[node].parent;
        if p != NO_ATOM && p != from {
            k.push(p);
        }
        k
    }

    /// Number of edges between two nodes of the tree.
    fn distance(&self, mut a: usize, mut b: usize) -> u16 {
        let mut d = 0;
        while a != b {
            if self.nodes[a].depth >= self.nodes[b].depth {
                a = self.nodes[a].parent;
            } else {
                b = self.nodes[b].parent;
            }
            d += 1;
        }
        d
    }

    /// The value of the node an arc enters under `rule`, in the tree hung from `root`; higher ranks first, 0 is a phantom atom.
    fn key(&mut self, (from, i): Arc, rule: usize, root: usize) -> u32 {
        let n = &self.nodes[i];
        let real = !n.dup && n.atom != NO_ATOM && i != root;
        match rule {
            0 => u32::from(n.z12),
            1 if n.ring => (1 << 16) + u32::from(u16::MAX - self.distance(root, n.orig)),
            2 => u32::from(n.mass),
            3 if real && from != NO_ATOM && self.nodes[from].atom != NO_ATOM => {
                let p = self.nodes[from].atom;
                let k = (p.min(n.atom), p.max(n.atom));
                self.c
                    .db
                    .iter()
                    .find(|d| (d.0, d.1) == k)
                    .map_or(0, |d| u32::from(d.2))
            }
            4 if real => u32::from(self.c.tetra[n.atom].is_some()),
            _ => 0,
        }
    }

    /// Whether node `k` next to `i` is a duplicate made by a multiple bond (a ring-closure duplicate stands for a real neighbour).
    fn bond_dup(&self, i: usize, k: usize) -> bool {
        let o = self.nodes[k].orig;
        self.nodes[k].dup && (o == self.nodes[i].parent || self.nodes[o].parent == i)
    }

    /// The descriptor of the centre at node `i` as this digraph sees it (R 2, S 1, 0 when its ligands tie): the same tree hung from `i`,
    /// its branches ranked by rules 1a to 3.
    fn auxiliary(&mut self, i: usize) -> u32 {
        if self.aux[i] > 0 {
            return self.aux[i] - 1;
        }
        let c = self.c;
        let Some(cen) = &c.tetra[self.nodes[i].atom] else {
            return 0;
        };
        let lig: Vec<usize> = self
            .onward((NO_ATOM, i))
            .into_iter()
            .filter(|&k| !self.bond_dup(i, k))
            .collect();
        let mut order = lig;
        let tied = !self.sort_under(i, &mut order, 3, i);
        let pos = |g: &Self, k: usize| match g.nodes[k].atom {
            NO_ATOM => cen.refs.len(),
            a => cen.refs.iter().position(|&r| r == a).unwrap_or(usize::MAX),
        };
        let at: Vec<usize> = order.iter().map(|&o| pos(self, o)).collect();
        let bad = at.len() != 4 || at.iter().any(|&p| p > cen.refs.len());
        let v = if tied || bad || self.over {
            0
        } else {
            u32::from(handed(&at, cen.cw))
        };
        self.aux[i] = v + 1;
        v
    }

    /// The nodes reached through an arc, highest first by the rules up to `upto` in the tree hung from `root`, as arcs.
    fn ranked(&mut self, arc: Arc, upto: usize, root: usize) -> Vec<Arc> {
        if let Some(s) = self.sorted[arc.1]
            .iter()
            .find(|s| (s.0, s.1, s.2) == (arc.0, upto, root))
        {
            return s.3.iter().map(|&k| (arc.1, k)).collect();
        }
        let mut v = self.onward(arc);
        self.sort_under(arc.1, &mut v, upto, root);
        self.sorted[arc.1].push((arc.0, upto, root, v.clone()));
        v.into_iter().map(|k| (arc.1, k)).collect()
    }

    /// The auxiliary descriptors met in the branch through `a`, sphere by sphere in ranked order, as like (true) or unlike a reference: the
    /// descriptor of the first ranked centre (each one in turn when centres tied with it differ, keeping the best pattern). Within a run of
    /// tied nodes, like comes first.
    fn pattern(&mut self, a: Arc, root: usize) -> Vec<bool> {
        let runs = self.runs(a, root);
        let Some(first) = runs.first() else {
            return Vec::new();
        };
        let mut refs: Vec<u32> = Vec::new();
        for &v in first {
            if !refs.contains(&v) {
                refs.push(v);
            }
        }
        refs.into_iter()
            .map(|r| {
                runs.iter()
                    .flat_map(|run| {
                        let like = run.iter().filter(|&&v| v == r).count();
                        (0..run.len()).map(move |j| j < like)
                    })
                    .collect::<Vec<bool>>()
            })
            .max()
            .unwrap_or_default()
    }

    /// Rule 5 on a branch: its auxiliary descriptors sphere by sphere, R before S within a run of tied nodes.
    fn handedness(&mut self, a: Arc, root: usize) -> Vec<u32> {
        self.runs(a, root)
            .into_iter()
            .flat_map(|r| {
                // descriptors are 2 (R) or 1 (S): R first
                let rr = r.iter().filter(|&&v| v == 2).count();
                (0..r.len()).map(move |j| if j < rr { 2 } else { 1 })
            })
            .collect()
    }

    /// The auxiliary descriptors (not 0) of the branch through `a`, sphere by sphere, grouped in runs of nodes that rules 1a to 4a tie.
    fn runs(&mut self, a: Arc, root: usize) -> Vec<Vec<u32>> {
        let mut runs: Vec<Vec<u32>> = Vec::new();
        // Each node of a sphere carries the run of its parent: nodes under tied parents are ranked together.
        let mut front: Vec<(usize, Arc)> = vec![(0, a)];
        while !front.is_empty() && !self.over {
            for j in 1..front.len() {
                let mut k = j;
                while k > 0
                    && front[k].0 == front[k - 1].0
                    && self.cmp_upto(front[k].1, front[k - 1].1, LIKE - 1, root)
                        == Ordering::Greater
                {
                    front.swap(k, k - 1);
                    k -= 1;
                }
            }
            let mut next = Vec::new();
            let mut prev: Option<(usize, Arc)> = None;
            for (g, x) in front {
                let tied = prev.is_some_and(|(pg, p)| {
                    pg == g && self.cmp_upto(p, x, LIKE - 1, root) == Ordering::Equal
                });
                if !tied {
                    runs.push(Vec::new());
                }
                prev = Some((g, x));
                let n = &self.nodes[x.1];
                if !n.dup && n.atom != NO_ATOM && self.c.tetra[n.atom].is_some() {
                    let v = self.auxiliary(x.1);
                    if v != 0
                        && let Some(r) = runs.last_mut()
                    {
                        r.push(v);
                    }
                }
                let run = runs.len();
                next.extend(self.ranked(x, LIKE - 1, root).into_iter().map(|k| (run, k)));
            }
            front = next;
        }
        runs.retain(|r| !r.is_empty());
        runs
    }

    /// Breadth-first comparison of the branches through `a` and `b` under one rule (`Greater`: `a` ranks first).
    fn cmp_rule(&mut self, a: Arc, b: Arc, rule: usize, root: usize) -> Ordering {
        if rule == LIKE {
            return self.pattern(a, root).cmp(&self.pattern(b, root));
        }
        if rule == LIKE + 1 {
            return self.handedness(a, root).cmp(&self.handedness(b, root));
        }
        let first = self.key(a, rule, root).cmp(&self.key(b, rule, root));
        if first != Ordering::Equal {
            return first;
        }
        let mut front = vec![(a, b)];
        while !front.is_empty() && !self.over {
            let mut next = Vec::new();
            for (x, y) in front {
                let (kx, ky) = (self.ranked(x, rule, root), self.ranked(y, rule, root));
                for j in 0..kx.len().max(ky.len()) {
                    let vx = kx.get(j).map_or(0, |&n| self.key(n, rule, root));
                    let vy = ky.get(j).map_or(0, |&n| self.key(n, rule, root));
                    if vx != vy {
                        return vx.cmp(&vy);
                    }
                }
                next.extend(kx.into_iter().zip(ky));
            }
            front = next;
        }
        Ordering::Equal
    }

    /// Comparison by every rule up to `upto`, in order, in the tree hung from `root`.
    fn cmp_upto(&mut self, a: Arc, b: Arc, upto: usize, root: usize) -> Ordering {
        for rule in 0..=upto {
            let o = self.cmp_rule(a, b, rule, root);
            if o != Ordering::Equal {
                return o;
            }
        }
        Ordering::Equal
    }

    /// Sorts nodes reached from `p` highest first by the rules up to `upto` in the tree hung from `root`; false when two of them tie.
    pub fn sort_under(&mut self, p: usize, v: &mut [usize], upto: usize, root: usize) -> bool {
        for j in 1..v.len() {
            let mut k = j;
            while k > 0 && self.cmp_upto((p, v[k]), (p, v[k - 1]), upto, root) == Ordering::Greater
            {
                v.swap(k, k - 1);
                k -= 1;
            }
        }
        (1..v.len()).all(|j| self.cmp_upto((p, v[j - 1]), (p, v[j]), upto, root) != Ordering::Equal)
    }
}
