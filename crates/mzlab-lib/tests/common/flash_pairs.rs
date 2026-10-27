//! The pair-collecting Flash search that the flat accumulators replaced, kept as the reference of the parity test and as the baseline of `libset_bench`.
use mzlab_lib::clean::weighted;
use mzlab_lib::io::Source;
use mzlab_lib::read::Library;
use mzlab_lib::scores::pair_term;
use mzlab_lib::search::{FlashMode, FlashScore, Query};
use mzlab_lib::sortutil;
use std::ops::Range;

/// The search as it was before the accumulators: all candidate pairs in a vector, sorted, one group per spectrum. Returns the number of competing groups too.
pub fn reference<S: Source>(
    lib: &Library<S>,
    q: &Query,
    mode: FlashMode,
    frag_da: f64,
    ranges: &[Range<u32>],
) -> (Vec<FlashScore>, usize) {
    let light = lib.light().unwrap();
    let pa = weighted(&q.spec.it);
    let mut pairs: Vec<(u32, u16, u32, f64)> = Vec::new();
    let in_ranges = |sp: u32| ranges.iter().any(|x| x.contains(&sp));
    let window = |x: f64, t: f64| -> Option<(u32, u32)> {
        let lo = (x - t).ceil().max(0.0);
        let hi = (x + t).floor();
        (hi >= 0.0 && lo <= hi && hi < u32::MAX as f64).then_some((lo as u32, hi as u32))
    };
    for (i, &mz) in q.spec.mz.iter().enumerate() {
        let t = frag_da * 1e5;
        if mode != FlashMode::NeutralLoss {
            if let Some((lo, hi)) = window(mz * 1e5, t) {
                lib.for_each_entry(false, lo, hi, |mq, sp, qi| {
                    if in_ranges(sp) {
                        let p = qi as f64 / 65535.0 * light.scale[sp as usize] as f64;
                        pairs.push((sp, i as u16, mq, pair_term(pa[i], p)));
                    }
                })
                .unwrap();
            }
        }
        if mode != FlashMode::Open {
            if let Some((lo, hi)) = window((q.prec - mz) * 1e5, t) {
                lib.for_each_entry(true, lo, hi, |nq, sp, qi| {
                    if in_ranges(sp) {
                        let p = qi as f64 / 65535.0 * light.scale[sp as usize] as f64;
                        let mq = light.precq[sp as usize].wrapping_sub(nq);
                        pairs.push((sp, i as u16, mq, pair_term(pa[i], p)));
                    }
                })
                .unwrap();
            }
        }
    }
    sortutil::sort_by(&mut pairs, |a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));
    pairs.dedup_by(|b, a| (a.0, a.1, a.2) == (b.0, b.1, b.2));
    let mut out = Vec::new();
    let mut competing = 0;
    let mut k = 0;
    while k < pairs.len() {
        let sp = pairs[k].0;
        let mut e = k;
        while e < pairs.len() && pairs[e].0 == sp {
            e += 1;
        }
        let g = &pairs[k..e];
        let clean = g.windows(2).all(|w| w[0].1 != w[1].1) && {
            let mut js: Vec<u32> = g.iter().map(|x| x.2).collect();
            js.sort();
            js.windows(2).all(|w| w[0] != w[1])
        };
        let (score, n) = if clean {
            (g.iter().map(|x| x.3).sum::<f64>(), g.len())
        } else {
            competing += 1;
            let mut order: Vec<usize> = (0..g.len()).collect();
            sortutil::sort_by(&mut order, |&a, &b| {
                g[b].3.total_cmp(&g[a].3).then(a.cmp(&b))
            });
            let (mut ui, mut uj) = (Vec::new(), Vec::new());
            let (mut s, mut n) = (0.0, 0);
            for x in order {
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
            index: sp,
            score: score.clamp(0.0, 1.0),
            pairs: n as u32,
        });
        k = e;
    }
    sortutil::sort_by(&mut out, |a, b| {
        b.score.total_cmp(&a.score).then(a.index.cmp(&b.index))
    });
    (out, competing)
}
