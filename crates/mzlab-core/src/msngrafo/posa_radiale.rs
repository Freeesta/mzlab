//! The radial drawing of `vista`: ring = steps of the best path, angle = the middle of the sector of the node, a sector being as wide as the
//! leaves of its subtree in the best-path tree. Degrees from the leaf counts, no trigonometry: the drawer turns them into x and y.

/// (ring, angle in degrees) of every node, given the parent of each on its best path (`None` = a root). A single root sits at the centre
/// (angle 0); children go by m/z from high to low, then by index.
pub(super) fn radiale(padre: &[Option<usize>], passi: &[usize], mz: &[f64]) -> Vec<(usize, f64)> {
    let n = padre.len();
    let mut figli: Vec<Vec<usize>> = vec![vec![]; n];
    let mut radici = vec![];
    for (i, p) in padre.iter().enumerate() {
        match p {
            Some(p) => figli[*p].push(i),
            None => radici.push(i),
        }
    }
    let ordine = |v: &mut Vec<usize>| {
        crate::sortc::any_by(v, |a, b| mz[*b].total_cmp(&mz[*a]).then(a.cmp(b)));
    };
    ordine(&mut radici);
    for f in figli.iter_mut() {
        ordine(f);
    }
    // leaves of each subtree: deepest nodes first
    let mut per_passi: Vec<usize> = (0..n).collect();
    crate::sortc::any_by(&mut per_passi, |a, b| {
        passi[*b].cmp(&passi[*a]).then(a.cmp(b))
    });
    let mut foglie = vec![0usize; n];
    for &i in &per_passi {
        foglie[i] = if figli[i].is_empty() {
            1
        } else {
            figli[i].iter().map(|&f| foglie[f]).sum()
        };
    }
    let totale: usize = radici.iter().map(|&r| foglie[r]).sum();
    let mut out = vec![(0, 0.0); n];
    let mut pila: Vec<usize> = vec![];
    let mut inizio = 0;
    for &r in radici.iter().rev() {
        // the roots take consecutive sectors in order: push them reversed so the first is popped first
        pila.push(r);
    }
    let mut cum = vec![0usize; n];
    for &r in &radici {
        cum[r] = inizio;
        inizio += foglie[r];
    }
    while let Some(i) = pila.pop() {
        let mid = cum[i] as f64 + foglie[i] as f64 / 2.0;
        let angolo = if radici.len() == 1 && padre[i].is_none() {
            0.0
        } else {
            crate::json::round(360.0 * mid / totale as f64, 2)
        };
        out[i] = (passi[i], angolo);
        let mut c = cum[i];
        for &f in &figli[i] {
            cum[f] = c;
            c += foglie[f];
            pila.push(f);
        }
    }
    out
}
