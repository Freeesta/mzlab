//! The drawing of `msngrafo`: one column per MS level, the rows ordered by the barycentre of the neighbours in the other columns (one pass
//! from the root, one back), starting from the m/z from high to low. The page only draws these positions.

use super::Grafo;

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
fn riordina(g: &Grafo, cols: &mut [Vec<usize>], c: usize, vicini: &[Vec<usize>], prima: bool) {
    let h = alto(cols, g.nodi.len());
    let col = |i: usize| g.nodi[i].livello as usize - 1;
    let chiave = |i: usize| {
        let v: Vec<f64> = vicini[i]
            .iter()
            .filter(|&&j| if prima { col(j) < c } else { col(j) > c })
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
            .then(g.nodi[b.1].mz.total_cmp(&g.nodi[a.1].mz))
            .then(a.1.cmp(&b.1))
    });
    cols[c] = k.into_iter().map(|(_, i)| i).collect();
}

/// Fills `pos` (column, row) of every node.
pub(super) fn posa(g: &mut Grafo) {
    let n = g.nodi.len();
    let ncol = g.nodi.iter().map(|x| x.livello as usize).max().unwrap_or(0);
    let mut cols: Vec<Vec<usize>> = vec![vec![]; ncol];
    for i in 0..n {
        cols[g.nodi[i].livello as usize - 1].push(i);
    }
    for c in cols.iter_mut() {
        crate::sortc::by(c, |a, b| {
            g.nodi[*b].mz.total_cmp(&g.nodi[*a].mz).then(a.cmp(b))
        });
    }
    let (mut su, mut giu) = (vec![vec![]; n], vec![vec![]; n]);
    for a in &g.archi {
        su[a.a].push(a.da);
        giu[a.da].push(a.a);
    }
    for c in 1..ncol {
        riordina(g, &mut cols, c, &su, true);
    }
    for c in (0..ncol.saturating_sub(1)).rev() {
        riordina(g, &mut cols, c, &giu, false);
    }
    for (c, col) in cols.iter().enumerate() {
        for (r, &i) in col.iter().enumerate() {
            g.nodi[i].pos = (c, r);
        }
    }
}
