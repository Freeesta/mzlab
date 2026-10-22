//! The drawing of `msngrafo`: one column per MS level, the rows ordered by the barycentre of the neighbours in the other columns (one pass
//! from the root, one back), starting from the m/z from high to low. `pos` is that; the tree view of `vista` (`albero`) adds a pass of
//! adjacent swaps that removes arc crossings. The page only draws these positions.

use super::Grafo;

/// What the drawing needs of a graph: the MS level and m/z of each node and its arcs (from, to), so it works on a `Grafo` and on a read JSON.
pub(super) struct Rete<'a> {
    pub livello: &'a [u8],
    pub mz: &'a [f64],
    pub archi: &'a [(usize, usize)],
}

impl Rete<'_> {
    fn col(&self, i: usize) -> usize {
        self.livello[i] as usize - 1
    }
}

/// Row of each node over the height of its column, 0..1.
fn alto(cols: &[Vec<usize>], n: usize) -> Vec<f64> {
    let mut h = vec![0.0; n];
    for c in cols {
        let d = (c.len().max(2) - 1) as f64;
        for (r, &i) in c.iter().enumerate() {
            h[i] = r as f64 / d;
        }
    }
    h
}

/// Orders column `c` by the mean height of the neighbours of each node that are in `altre` columns (the node keeps its height without).
fn riordina(r: &Rete, cols: &mut [Vec<usize>], c: usize, vicini: &[Vec<usize>], prima: bool) {
    let h = alto(cols, r.mz.len());
    let chiave = |i: usize| {
        let v: Vec<f64> = vicini[i]
            .iter()
            .filter(|&&j| if prima { r.col(j) < c } else { r.col(j) > c })
            .map(|&j| h[j])
            .collect();
        if v.is_empty() {
            h[i]
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    let mut k: Vec<(f64, usize)> = cols[c].iter().map(|&i| (chiave(i), i)).collect();
    crate::sortc::by(&mut k, |a, b| {
        a.0.total_cmp(&b.0)
            .then(r.mz[b.1].total_cmp(&r.mz[a.1]))
            .then(a.1.cmp(&b.1))
    });
    cols[c] = k.into_iter().map(|(_, i)| i).collect();
}

/// The nodes of each column after `giri` sweeps of the barycentre order (one down from the root, one back).
fn baricentro(r: &Rete, giri: usize) -> Vec<Vec<usize>> {
    let n = r.mz.len();
    let ncol = r.livello.iter().map(|&l| l as usize).max().unwrap_or(0);
    let mut cols: Vec<Vec<usize>> = vec![vec![]; ncol];
    for i in 0..n {
        cols[r.col(i)].push(i);
    }
    for c in cols.iter_mut() {
        crate::sortc::by(c, |a, b| r.mz[*b].total_cmp(&r.mz[*a]).then(a.cmp(b)));
    }
    let (mut su, mut giu) = (vec![vec![]; n], vec![vec![]; n]);
    for &(da, a) in r.archi {
        su[a].push(da);
        giu[da].push(a);
    }
    for _ in 0..giri {
        for c in 1..ncol {
            riordina(r, &mut cols, c, &su, true);
        }
        for c in (0..ncol.saturating_sub(1)).rev() {
            riordina(r, &mut cols, c, &giu, false);
        }
    }
    cols
}

fn posizioni(cols: &[Vec<usize>], n: usize) -> Vec<(usize, usize)> {
    let mut pos = vec![(0, 0); n];
    for (c, col) in cols.iter().enumerate() {
        for (riga, &i) in col.iter().enumerate() {
            pos[i] = (c, riga);
        }
    }
    pos
}

/// The barycentre positions (column, row) of every node.
pub(super) fn baricentro_pos(r: &Rete) -> Vec<(usize, usize)> {
    posizioni(&baricentro(r, 1), r.mz.len())
}

/// Fills `pos` (column, row) of every node.
pub(super) fn posa(g: &mut Grafo) {
    let livello: Vec<u8> = g.nodi.iter().map(|x| x.livello).collect();
    let mz: Vec<f64> = g.nodi.iter().map(|x| x.mz).collect();
    let archi: Vec<(usize, usize)> = g.archi.iter().map(|a| (a.da, a.a)).collect();
    let pos = baricentro_pos(&Rete {
        livello: &livello,
        mz: &mz,
        archi: &archi,
    });
    for (n, p) in g.nodi.iter_mut().zip(pos) {
        n.pos = p;
    }
}

/// The arcs between two different columns as (lower column end, higher column end).
fn tra_colonne(r: &Rete) -> Vec<(usize, usize)> {
    r.archi
        .iter()
        .filter(|&&(a, b)| r.col(a) != r.col(b))
        .map(|&(a, b)| if r.col(a) < r.col(b) { (a, b) } else { (b, a) })
        .collect()
}

/// Whether two arcs of the same pair of columns cross: their ends are in opposite order in both columns.
fn si_incrociano(r: &Rete, pos: &[(usize, usize)], x: (usize, usize), y: (usize, usize)) -> bool {
    if r.col(x.0) != r.col(y.0) || r.col(x.1) != r.col(y.1) {
        return false;
    }
    let (a, b) = (
        pos[x.0].1 as i64 - pos[y.0].1 as i64,
        pos[x.1].1 as i64 - pos[y.1].1 as i64,
    );
    a * b < 0
}

/// How many pairs of arcs of the same pair of columns cross (arcs that share an end never do).
pub(super) fn incroci(r: &Rete, pos: &[(usize, usize)]) -> usize {
    let v = tra_colonne(r);
    let mut k = 0;
    for (i, &x) in v.iter().enumerate() {
        k += v[i + 1..]
            .iter()
            .filter(|&&y| si_incrociano(r, pos, x, y))
            .count();
    }
    k
}

/// More swaps than this do not help a graph of this kind and would only cost time.
const GIRI: usize = 8;

/// The tree view: the barycentre order, then passes of swaps of two adjacent rows of a column while a swap removes more crossings than it
/// makes (the pairs of arcs of the two nodes are the only ones that change), at most `GIRI` passes; deterministic.
pub(super) fn albero(r: &Rete) -> Vec<(usize, usize)> {
    let n = r.mz.len();
    // more sweeps of the barycentre while they leave fewer crossings
    let mut cols = baricentro(r, 1);
    let mut meno = incroci(r, &posizioni(&cols, n));
    for giri in 2..=GIRI {
        let c = baricentro(r, giri);
        let k = incroci(r, &posizioni(&c, n));
        if k < meno {
            (cols, meno) = (c, k);
        }
    }
    let mut inc: Vec<Vec<(usize, usize)>> = vec![vec![]; n];
    for (a, b) in tra_colonne(r) {
        inc[a].push((a, b));
        inc[b].push((a, b));
    }
    let mut pos = posizioni(&cols, n);
    for _ in 0..GIRI {
        let mut mosso = false;
        for col in cols.iter_mut() {
            for k in 1..col.len() {
                let (x, y) = (col[k - 1], col[k]);
                let (mut ora, mut dopo) = (0, 0);
                for &ex in &inc[x] {
                    for &ey in &inc[y] {
                        let altro = |e: (usize, usize), n: usize| if e.0 == n { e.1 } else { e.0 };
                        if altro(ex, x) == altro(ey, y) {
                            continue;
                        }
                        if si_incrociano(r, &pos, ex, ey) {
                            ora += 1;
                        } else if r.col(ex.0) == r.col(ey.0) && r.col(ex.1) == r.col(ey.1) {
                            dopo += 1;
                        }
                    }
                }
                if dopo < ora {
                    col.swap(k - 1, k);
                    pos[x].1 = k;
                    pos[y].1 = k - 1;
                    mosso = true;
                }
            }
        }
        if !mosso {
            break;
        }
    }
    pos
}
