//! The radial drawing of `vista`: ring = steps of the best path, angle = the middle of the sector of the node, a sector being as wide as the
//! leaves of its subtree in the best-path tree. Degrees from the leaf counts, no trigonometry: the drawer turns them into x and y.

/// (ring, angle in degrees) of every node, given the parent of each on its best path (`None` = a root). A single root sits at the centre
/// (angle 0); children go by m/z from high to low, then by index.
pub(super) fn radiale(parent: &[Option<usize>], steps: &[usize], mz: &[f64]) -> Vec<(usize, f64)> {
    let n = parent.len();
    let mut figli: Vec<Vec<usize>> = vec![vec![]; n];
    let mut radici = vec![];
    for (i, p) in parent.iter().enumerate() {
        match p {
            Some(p) => figli[*p].push(i),
            None => radici.push(i),
        }
    }
    let order = |v: &mut Vec<usize>| {
        crate::sortc::any_by(v, |a, b| mz[*b].total_cmp(&mz[*a]).then(a.cmp(b)));
    };
    order(&mut radici);
    for f in figli.iter_mut() {
        order(f);
    }
    // leaves of each subtree: deepest nodes first
    let mut by_steps: Vec<usize> = (0..n).collect();
    crate::sortc::any_by(&mut by_steps, |a, b| {
        steps[*b].cmp(&steps[*a]).then(a.cmp(b))
    });
    let mut foglie = vec![0usize; n];
    for &i in &by_steps {
        foglie[i] = if figli[i].is_empty() {
            1
        } else {
            figli[i].iter().map(|&f| foglie[f]).sum()
        };
    }
    let total: usize = radici.iter().map(|&r| foglie[r]).sum();
    let mut out = vec![(0, 0.0); n];
    let mut pila: Vec<usize> = vec![];
    let mut start = 0;
    for &r in radici.iter().rev() {
        // the roots take consecutive sectors in order: push them reversed so the first is popped first
        pila.push(r);
    }
    let mut cum = vec![0usize; n];
    for &r in &radici {
        cum[r] = start;
        start += foglie[r];
    }
    while let Some(i) = pila.pop() {
        let mid = cum[i] as f64 + foglie[i] as f64 / 2.0;
        let angolo = if radici.len() == 1 && parent[i].is_none() {
            0.0
        } else {
            crate::json::round(360.0 * mid / total as f64, 2)
        };
        out[i] = (steps[i], angolo);
        let mut c = cum[i];
        for &f in &figli[i] {
            cum[f] = c;
            c += foglie[f];
            pila.push(f);
        }
    }
    out
}
