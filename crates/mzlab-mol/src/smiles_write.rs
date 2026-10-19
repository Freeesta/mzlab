//! `mol/1` into SMILES: every atom in brackets with its hydrogens, Kekulé bond symbols, `@`/`@@` and `/`/`\` for the defined stereo
//! elements, enhanced-stereo groups as a CXSMILES tail. It is meant for round trips and for other tools to read; unknown elements
//! cannot be written (a SMILES says nothing about them) and the radicals are left out (the reader derives them).

use crate::codec::MolError;
use crate::model::{GroupKind, Mol, Parity, Status, Stereo};
use std::collections::BTreeMap;

fn fail<T>(msg: impl Into<String>) -> Result<T, MolError> {
    Err(MolError::Smiles(msg.into()))
}

struct Walk<'a> {
    m: &'a Mol,
    /// Neighbours with the order of the bond joining them, ascending.
    adj: Vec<Vec<(u32, u8)>>,
    parent: Vec<Option<usize>>,
    pos: Vec<usize>,
    children: Vec<Vec<usize>>,
    closures: Vec<Vec<usize>>,
    order: Vec<usize>,
    /// For a directional bond `(lo, hi)`: whether `hi` sits above `lo`.
    up: BTreeMap<(u32, u32), bool>,
}

impl Mol {
    /// SMILES (with a CXSMILES tail when there are stereo groups) and, for every written atom, its index in this record.
    pub fn to_smiles_with_order(&self) -> Result<(String, Vec<u32>), MolError> {
        let mut w = Walk::new(self);
        w.directions()?;
        let mut text = String::new();
        for s in 0..self.atoms.len() {
            if w.pos[s] == usize::MAX {
                w.visit(s, None);
                if !text.is_empty() {
                    text.push('.');
                }
                w.emit(s, &mut BTreeMap::new(), &mut text);
            }
        }
        text.push_str(&w.tail());
        Ok((text, w.order.iter().map(|&a| a as u32).collect()))
    }

    /// SMILES text of the record (see `to_smiles_with_order`).
    pub fn to_smiles(&self) -> Result<String, MolError> {
        self.to_smiles_with_order().map(|(s, _)| s)
    }
}

impl<'a> Walk<'a> {
    fn new(m: &'a Mol) -> Self {
        let n = m.atoms.len();
        let mut adj = vec![Vec::new(); n];
        for b in &m.bonds {
            adj[b.a as usize].push((b.b, b.o));
            adj[b.b as usize].push((b.a, b.o));
        }
        for v in &mut adj {
            v.sort_unstable();
        }
        Walk {
            m,
            adj,
            parent: vec![None; n],
            pos: vec![usize::MAX; n],
            children: vec![Vec::new(); n],
            closures: vec![Vec::new(); n],
            order: Vec::new(),
            up: BTreeMap::new(),
        }
    }

    /// Depth-first tree from `u`: parents, children in visiting order, and the ring-closure partners of every atom.
    fn visit(&mut self, u: usize, parent: Option<usize>) {
        self.parent[u] = parent;
        self.pos[u] = self.order.len();
        self.order.push(u);
        for k in 0..self.adj[u].len() {
            let v = self.adj[u][k].0 as usize;
            if self.pos[v] == usize::MAX {
                self.children[u].push(v);
                self.visit(v, Some(u));
            } else if Some(v) != parent && self.pos[v] < self.pos[u] {
                self.closures[u].push(v);
                self.closures[v].push(u);
            }
        }
    }

    /// Whether the neighbour `r` of `x` is above `x`, as far as the marks already given around `x` (other than toward `skip`) say.
    fn known(&self, x: u32, skip: u32, r: u32) -> Option<bool> {
        self.adj[x as usize]
            .iter()
            .filter(|&&(o, _)| o != skip)
            .find_map(|&(o, _)| {
                let up = self.mark(x, o)?;
                Some(up == (o == r))
            })
    }

    /// Whether `o` is above `x` when the bond carries a direction.
    fn mark(&self, x: u32, o: u32) -> Option<bool> {
        let hi_up = *self.up.get(&(x.min(o), x.max(o)))?;
        Some(hi_up == (o > x))
    }

    fn put(&mut self, x: u32, o: u32, up: bool) {
        self.up.insert((x.min(o), x.max(o)), up == (o > x));
    }

    /// Chooses `/` and `\` for the bonds beside every defined double bond; marks already given by a neighbouring double bond are kept.
    fn directions(&mut self) -> Result<(), MolError> {
        for s in &self.m.stereo {
            let Stereo::Db {
                a,
                b,
                refs: Some([ra, rb]),
                cfg: Some(cfg),
                status: Status::Defined,
            } = s
            else {
                continue;
            };
            let cis = *cfg == crate::model::Cfg::Cis;
            match (self.known(*a, *b, *ra), self.known(*b, *a, *rb)) {
                (None, None) => {
                    self.put(*a, *ra, true);
                    self.put(*b, *rb, cis);
                }
                (Some(p), None) => self.put(*b, *rb, p == cis),
                (None, Some(q)) => self.put(*a, *ra, q == cis),
                (Some(p), Some(q)) if (p == q) == cis => {}
                _ => {
                    return fail(format!(
                        "the double bonds around {a}={b} ask for incompatible / and \\ marks"
                    ))
                }
            }
        }
        Ok(())
    }

    /// Neighbours of `u` in the order the text lists them: predecessor, ring digits, branches and chain.
    fn written_neighbours(&self, u: usize) -> Vec<usize> {
        let mut ring = self.closures[u].clone();
        ring.sort_by_key(|&p| self.pos[p]);
        self.parent[u]
            .into_iter()
            .chain(ring)
            .chain(self.children[u].iter().copied())
            .collect()
    }

    /// `@`, `@@` or nothing for atom `u`.
    fn chirality(&self, u: usize) -> &'static str {
        let found = self.m.stereo.iter().find_map(|s| match s {
            Stereo::Tetra {
                c,
                par: Some(p),
                status: Status::Defined,
                ..
            } if *c as usize == u => Some(*p),
            _ => None,
        });
        let Some(par) = found else { return "" };
        let mut list: Vec<usize> = self.written_neighbours(u);
        if list.len() == 3 {
            list.insert(usize::from(self.parent[u].is_some()), usize::MAX);
        }
        let inversions = (0..list.len())
            .flat_map(|i| (i + 1..list.len()).map(move |j| (i, j)))
            .filter(|&(i, j)| list[i] > list[j])
            .count();
        if (par == Parity::Ccw) ^ (inversions % 2 == 1) {
            "@"
        } else {
            "@@"
        }
    }

    fn atom_text(&self, u: usize) -> String {
        let a = &self.m.atoms[u];
        let iso = if a.iso > 0 {
            a.iso.to_string()
        } else {
            String::new()
        };
        let h = match a.h {
            0 => String::new(),
            1 => "H".to_string(),
            n => format!("H{n}"),
        };
        let chg = match a.chg {
            0 => String::new(),
            1 => "+".to_string(),
            -1 => "-".to_string(),
            n if n > 0 => format!("+{n}"),
            n => format!("-{}", -n),
        };
        format!("[{iso}{}{}{h}{chg}]", a.el, self.chirality(u))
    }

    /// Symbol of the bond from `u` to `v` as it is written.
    fn bond_symbol(&self, u: usize, v: usize) -> &'static str {
        let order = self.adj[u]
            .iter()
            .find(|&&(o, _)| o as usize == v)
            .map_or(1, |&(_, o)| o);
        match (order, self.mark(u as u32, v as u32)) {
            (1, Some(true)) => "/",
            (1, Some(false)) => "\\",
            (2, _) => "=",
            (3, _) => "#",
            _ => "",
        }
    }

    fn ring_digit(n: usize) -> String {
        if n < 10 {
            n.to_string()
        } else {
            format!("%{n:02}")
        }
    }

    /// Writes the subtree at `u`. `open` maps a ring bond (lower position, higher position) to its digit while it is open.
    fn emit(&self, u: usize, open: &mut BTreeMap<(usize, usize), usize>, out: &mut String) {
        out.push_str(&self.atom_text(u));
        let mut ring = self.closures[u].clone();
        ring.sort_by_key(|&p| self.pos[p]);
        for p in ring {
            let key = (self.pos[u].min(self.pos[p]), self.pos[u].max(self.pos[p]));
            if let Some(n) = open.remove(&key) {
                out.push_str(&Self::ring_digit(n));
            } else {
                let n = (1..).find(|n| !open.values().any(|v| v == n)).unwrap_or(1);
                open.insert(key, n);
                out.push_str(self.bond_symbol(u, p));
                out.push_str(&Self::ring_digit(n));
            }
        }
        let last = self.children[u].len().saturating_sub(1);
        for (k, &c) in self.children[u].iter().enumerate() {
            if k < last {
                out.push('(');
            }
            out.push_str(self.bond_symbol(u, c));
            self.emit(c, open, out);
            if k < last {
                out.push(')');
            }
        }
    }

    /// The CXSMILES tail for the enhanced-stereo groups, atoms by their position in the text.
    fn tail(&self) -> String {
        let (mut ands, mut ors) = (0, 0);
        let mut parts = Vec::new();
        for g in &self.m.groups {
            let mut at: Vec<usize> = g.atoms.iter().map(|&a| self.pos[a as usize]).collect();
            at.sort_unstable();
            let list: Vec<String> = at.iter().map(usize::to_string).collect();
            let head = match g.kind {
                GroupKind::Abs => "a".to_string(),
                GroupKind::And => {
                    ands += 1;
                    format!("&{ands}")
                }
                GroupKind::Or => {
                    ors += 1;
                    format!("o{ors}")
                }
            };
            parts.push(format!("{head}:{}", list.join(",")));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(" |{}|", parts.join(","))
        }
    }
}
