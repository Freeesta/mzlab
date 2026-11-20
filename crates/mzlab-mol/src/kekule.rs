//! Kekulization: choose which aromatic bonds become double so that every atom that needs a double bond gets exactly one.

use crate::valence::needs_double;

/// Upper bound on the backtracking steps; a ring system that needs more is reported as impossible rather than hanging.
const MAX_STEPS: u32 = 2_000_000;

struct Search<'a> {
    adj: &'a [Vec<usize>],
    needy: &'a [bool],
    mate: Vec<Option<usize>>,
    steps: u32,
}

impl Search<'_> {
    fn free_options(&self, a: usize) -> usize {
        self.adj[a]
            .iter()
            .filter(|&&b| self.needy[b] && self.mate[b].is_none())
            .count()
    }

    /// The unmatched needy atom with the fewest free partners (the most constrained first); None when all are matched.
    fn pick(&self) -> Option<usize> {
        (0..self.needy.len())
            .filter(|&a| self.needy[a] && self.mate[a].is_none())
            .min_by_key(|&a| self.free_options(a))
    }

    fn solve(&mut self) -> bool {
        let Some(a) = self.pick() else { return true };
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return false;
        }
        let options: Vec<usize> = self.adj[a]
            .iter()
            .copied()
            .filter(|&b| self.needy[b] && self.mate[b].is_none())
            .collect();
        for b in options {
            self.mate[a] = Some(b);
            self.mate[b] = Some(a);
            if self.solve() {
                return true;
            }
            self.mate[a] = None;
            self.mate[b] = None;
        }
        false
    }
}

/// Aromatic bonds `(a, b)` between `n` atoms; `needy[i]` marks atoms that need a double bond. Returns the bonds to make double,
/// or None when no assignment exists.
pub(crate) fn kekulize(
    n: usize,
    bonds: &[(usize, usize)],
    needy: &[bool],
) -> Option<Vec<(usize, usize)>> {
    let mut adj = vec![Vec::new(); n];
    for &(a, b) in bonds {
        adj[a].push(b);
        adj[b].push(a);
    }
    let mut s = Search {
        adj: &adj,
        needy,
        mate: vec![None; n],
        steps: 0,
    };
    if !s.solve() {
        return None;
    }
    let doubles = (0..n)
        .filter_map(|a| s.mate[a].filter(|&b| a < b).map(|b| (a, b)))
        .collect();
    Some(doubles)
}

/// What the aromatic step needs to know about an atom: its symbol, charge and the hydrogens already counted.
pub(crate) struct Facts<'a> {
    pub el: &'a str,
    pub chg: i32,
    pub h: u32,
}

/// Turns the aromatic bonds (`orders[i] == 0`, between `ends[i]`) into single or double ones, so that every aromatic atom that needs a
/// double bond gets exactly one. None when no assignment exists; else the atoms that were given a hydrogen the input does not state
/// when another atom could carry it as well (empty when nothing was chosen), so that the caller can report the choice.
pub(crate) fn assign(
    ends: &[(usize, usize)],
    orders: &mut [u8],
    atoms: &[Facts],
) -> Option<Vec<usize>> {
    let aromatic: Vec<usize> = (0..orders.len()).filter(|&i| orders[i] == 0).collect();
    if aromatic.is_empty() {
        return Some(Vec::new());
    }
    let n = atoms.len();
    let mut total = vec![0u32; n];
    let mut in_ring = vec![false; n];
    for (i, &(f, t)) in ends.iter().enumerate() {
        let o = u32::from(orders[i].max(1));
        total[f] += o;
        total[t] += o;
        if orders[i] == 0 {
            in_ring[f] = true;
            in_ring[t] = true;
        }
    }
    let needs = |i: usize, extra_h: u32| {
        in_ring[i] && needs_double(atoms[i].el, atoms[i].chg, total[i] + atoms[i].h + extra_h)
    };
    let needy: Vec<bool> = (0..n).map(|i| needs(i, 0)).collect();
    let pairs: Vec<(usize, usize)> = aromatic.iter().map(|&i| ends[i]).collect();
    let stated = kekulize(n, &pairs, &needy).map(|d| (d, Vec::new()));
    let (doubles, given) = stated.or_else(|| {
        // a file that does not say which ring atom carries the H (pyrrole-type N): try one, then two, of the atoms that an H would satisfy
        // heteroatoms first: the H of indole or carbazole is on the N, never on a ring-fusion carbon that would leave the ring
        let mut flexible: Vec<usize> = (0..n).filter(|&i| needy[i] && !needs(i, 1)).collect();
        flexible.sort_by_key(|&i| atoms[i].el == "C");
        let with_h = |skip: &[usize]| {
            let mut v = needy.clone();
            skip.iter().for_each(|&i| v[i] = false);
            kekulize(n, &pairs, &v).map(|d| (d, skip.to_vec()))
        };
        let carbons = |v: &[usize]| v.iter().filter(|&&i| atoms[i].el == "C").count();
        let mut one = flexible.iter().filter_map(|&i| with_h(&[i]));
        let mut two = flexible.iter().enumerate().flat_map(|(k, &i)| {
            let with_h = &with_h;
            flexible[k + 1..]
                .iter()
                .filter_map(move |&j| with_h(&[i, j]))
        });
        // the first that works is taken; it is a choice (reported) only when another atom of the same kind works too
        let (doubles, given) = one.next().or_else(|| two.next())?;
        let same = |(_, other): (_, Vec<usize>)| {
            other.len() == given.len() && carbons(&other) == carbons(&given)
        };
        let choice = if given.len() == 1 {
            one.any(same)
        } else {
            two.any(same)
        };
        Some((doubles, if choice { given } else { Vec::new() }))
    })?;
    for &i in &aromatic {
        let (f, t) = ends[i];
        orders[i] = if doubles.contains(&(f.min(t), f.max(t))) {
            2
        } else {
            1
        };
    }
    Some(given)
}
