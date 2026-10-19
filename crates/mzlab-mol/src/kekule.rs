//! Kekulization: choose which aromatic bonds become double so that every atom that needs a double bond gets exactly one.

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
