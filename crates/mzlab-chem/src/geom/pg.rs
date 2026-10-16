//! Minimal undirected graph with the corner of the `petgraph` API that the geometry code uses (written for mzLab, no dependency).
//! Neighbours and incident edges come out in the order `petgraph::graph::UnGraph` gives them: for every node the edges where it is the
//! first end, newest first, then those where it is the second end, newest first. Edge references come out in insertion order.

use std::ops::{Index, IndexMut};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct NodeIndex(u32);

impl NodeIndex {
    pub fn new(i: usize) -> Self {
        NodeIndex(i as u32)
    }
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EdgeIndex(u32);

impl EdgeIndex {
    pub fn new(i: usize) -> Self {
        EdgeIndex(i as u32)
    }
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// One edge seen from a node (`EdgeRef` of petgraph): `source` is the node the walk started from, `target` the other end.
pub struct EdgeRef<'a, E> {
    id: EdgeIndex,
    source: NodeIndex,
    target: NodeIndex,
    weight: &'a E,
}

impl<'a, E> EdgeRef<'a, E> {
    pub fn id(&self) -> EdgeIndex {
        self.id
    }
    pub fn source(&self) -> NodeIndex {
        self.source
    }
    pub fn target(&self) -> NodeIndex {
        self.target
    }
    pub fn weight(&self) -> &'a E {
        self.weight
    }
}

struct EdgeData<E> {
    ends: [NodeIndex; 2],
    weight: E,
}

pub struct UnGraph<N, E> {
    nodes: Vec<N>,
    edges: Vec<EdgeData<E>>,
    /// Per node: edges where it is the first end, then those where it is the second end (insertion order).
    first: Vec<Vec<EdgeIndex>>,
    second: Vec<Vec<EdgeIndex>>,
}

impl<N, E> Default for UnGraph<N, E> {
    fn default() -> Self {
        Self::new_undirected()
    }
}

impl<N, E> UnGraph<N, E> {
    pub fn new_undirected() -> Self {
        UnGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
            first: Vec::new(),
            second: Vec::new(),
        }
    }

    pub fn add_node(&mut self, w: N) -> NodeIndex {
        self.nodes.push(w);
        self.first.push(Vec::new());
        self.second.push(Vec::new());
        NodeIndex::new(self.nodes.len() - 1)
    }

    pub fn add_edge(&mut self, a: NodeIndex, b: NodeIndex, w: E) -> EdgeIndex {
        let id = EdgeIndex::new(self.edges.len());
        self.edges.push(EdgeData {
            ends: [a, b],
            weight: w,
        });
        self.first[a.index()].push(id);
        self.second[b.index()].push(id);
        id
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn node_indices(&self) -> impl Iterator<Item = NodeIndex> {
        (0..self.nodes.len()).map(NodeIndex::new)
    }

    pub fn edge_indices(&self) -> impl Iterator<Item = EdgeIndex> {
        (0..self.edges.len()).map(EdgeIndex::new)
    }

    pub fn node_weight(&self, n: NodeIndex) -> Option<&N> {
        self.nodes.get(n.index())
    }

    pub fn node_weight_mut(&mut self, n: NodeIndex) -> Option<&mut N> {
        self.nodes.get_mut(n.index())
    }

    pub fn edge_weight(&self, e: EdgeIndex) -> Option<&E> {
        self.edges.get(e.index()).map(|d| &d.weight)
    }

    pub fn edge_endpoints(&self, e: EdgeIndex) -> Option<(NodeIndex, NodeIndex)> {
        self.edges.get(e.index()).map(|d| (d.ends[0], d.ends[1]))
    }

    /// Edges incident to `n` in petgraph order.
    fn incident(&self, n: NodeIndex) -> impl Iterator<Item = EdgeIndex> + '_ {
        let i = n.index();
        self.first[i]
            .iter()
            .rev()
            .chain(self.second[i].iter().rev())
            .copied()
    }

    pub fn neighbors(&self, n: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        self.incident(n).map(move |e| {
            let [a, b] = self.edges[e.index()].ends;
            if a == n {
                b
            } else {
                a
            }
        })
    }

    pub fn edges(&self, n: NodeIndex) -> impl Iterator<Item = EdgeRef<'_, E>> + '_ {
        self.incident(n).map(move |e| {
            let d = &self.edges[e.index()];
            let other = if d.ends[0] == n { d.ends[1] } else { d.ends[0] };
            EdgeRef {
                id: e,
                source: n,
                target: other,
                weight: &d.weight,
            }
        })
    }

    /// Every edge once, in insertion order, with `source`/`target` as added.
    pub fn edge_references(&self) -> impl Iterator<Item = EdgeRef<'_, E>> + '_ {
        self.edges.iter().enumerate().map(|(i, d)| EdgeRef {
            id: EdgeIndex::new(i),
            source: d.ends[0],
            target: d.ends[1],
            weight: &d.weight,
        })
    }

    pub fn find_edge(&self, a: NodeIndex, b: NodeIndex) -> Option<EdgeIndex> {
        self.incident(a).find(|e| {
            let [x, y] = self.edges[e.index()].ends;
            (x == a && y == b) || (x == b && y == a)
        })
    }
}

impl<N, E> Index<NodeIndex> for UnGraph<N, E> {
    type Output = N;
    fn index(&self, n: NodeIndex) -> &N {
        &self.nodes[n.index()]
    }
}

impl<N, E> IndexMut<NodeIndex> for UnGraph<N, E> {
    fn index_mut(&mut self, n: NodeIndex) -> &mut N {
        &mut self.nodes[n.index()]
    }
}

impl<N, E> Index<EdgeIndex> for UnGraph<N, E> {
    type Output = E;
    fn index(&self, e: EdgeIndex) -> &E {
        &self.edges[e.index()].weight
    }
}

/// Number of connected components.
pub fn connected_components<N, E>(g: &UnGraph<N, E>) -> usize {
    let n = g.node_count();
    let mut seen = vec![false; n];
    let mut count = 0;
    for s in 0..n {
        if seen[s] {
            continue;
        }
        count += 1;
        seen[s] = true;
        let mut stack = vec![NodeIndex::new(s)];
        while let Some(c) = stack.pop() {
            for nb in g.neighbors(c) {
                if !seen[nb.index()] {
                    seen[nb.index()] = true;
                    stack.push(nb);
                }
            }
        }
    }
    count
}
