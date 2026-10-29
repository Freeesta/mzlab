//! One-to-one resolution of the Flash candidates of the spectra whose pairs are not already one-to-one (used by `flash`).

use crate::flash::FlashScore;
use crate::sortutil;

/// Best one-to-one score per spectrum from the `(spectrum, query peak, library peak, contribution)` candidates (unsorted result).
/// The candidates are grouped by spectrum with a counting sort, then each small group is ordered and resolved on its own.
pub(crate) fn score_pairs(pairs: Vec<(u32, u16, u32, f64)>) -> Vec<FlashScore> {
    let (Some(lo), Some(hi)) = (
        pairs.iter().map(|p| p.0).min(),
        pairs.iter().map(|p| p.0).max(),
    ) else {
        return Vec::new();
    };
    let span = (hi - lo) as usize + 1;
    let mut start = vec![0u32; span + 1];
    for p in &pairs {
        if let Some(c) = start.get_mut((p.0 - lo) as usize + 1) {
            *c += 1;
        }
    }
    for i in 0..span {
        start[i + 1] += start[i];
    }
    let mut cursor = start.clone();
    let mut grouped = pairs.clone();
    for p in &pairs {
        let c = &mut cursor[(p.0 - lo) as usize];
        grouped[*c as usize] = *p;
        *c += 1;
    }
    drop(pairs);
    let mut out: Vec<FlashScore> = Vec::new();
    // scratch buffers shared by all groups
    let (mut js, mut order, mut ui, mut uj): (Vec<u32>, Vec<usize>, Vec<u16>, Vec<u32>) =
        Default::default();
    for sp_off in 0..span {
        let (a, e) = (start[sp_off] as usize, start[sp_off + 1] as usize);
        if a == e {
            continue;
        }
        let g = &mut grouped[a..e];
        sortutil::sort_by(g, |x, y| (x.1, x.2).cmp(&(y.1, y.2)));
        // drop repeated (query peak, library peak) candidates
        let mut m = 1;
        for k in 1..g.len() {
            if (g[k].1, g[k].2) != (g[m - 1].1, g[m - 1].2) {
                g[m] = g[k];
                m += 1;
            }
        }
        let g = &g[..m];
        let clean_group = g.windows(2).all(|w| w[0].1 != w[1].1) && {
            js.clear();
            js.extend(g.iter().map(|x| x.2));
            sortutil::sort_by(&mut js, |a, b| a.cmp(b));
            js.windows(2).all(|w| w[0] != w[1])
        };
        let (score, n) = if clean_group {
            (g.iter().map(|x| x.3).sum::<f64>(), g.len())
        } else {
            order.clear();
            order.extend(0..g.len());
            sortutil::sort_by(&mut order, |&a, &b| {
                g[b].3.total_cmp(&g[a].3).then(a.cmp(&b))
            });
            ui.clear();
            uj.clear();
            let (mut s, mut n) = (0.0, 0);
            for &x in &order {
                if !ui.contains(&g[x].1) && !uj.contains(&g[x].2) {
                    ui.push(g[x].1);
                    uj.push(g[x].2);
                    s += g[x].3;
                    n += 1;
                }
            }
            (s, n)
        };
        out.push(FlashScore {
            index: lo + sp_off as u32,
            score: score.clamp(0.0, 1.0),
            pairs: n as u32,
        });
    }
    out
}
