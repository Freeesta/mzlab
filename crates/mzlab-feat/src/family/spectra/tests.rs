//! `Top`: the kept peaks of a spectrum against a scan of the whole list on every push.

use super::Top;

/// The rule `Top::push` keeps: when full, the first weakest peak gives way to a stronger one (searched on every push).
fn by_scanning(top: &mut (Vec<f64>, Vec<f64>), cap: usize, m: f64, y: f64) {
    if top.0.len() < cap {
        top.0.push(m);
        top.1.push(y);
        return;
    }
    let mut w = 0;
    for k in 1..top.1.len() {
        if top.1[k] < top.1[w] {
            w = k;
        }
    }
    if w < top.1.len() && y > top.1[w] {
        top.0[w] = m;
        top.1[w] = y;
    }
}

#[test]
fn the_kept_peaks_are_the_ones_of_scanning_on_every_push() {
    let mut x = 0x1234_5678_9abc_def1u64;
    for cap in [0, 1, 2, 5, 40] {
        let (mut top, mut want) = (Top::default(), (Vec::new(), Vec::new()));
        for n in 0..5_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            // few distinct intensities: many ties
            let y = (x % 23) as f64;
            top.push(cap, n as f64, y);
            by_scanning(&mut want, cap, n as f64, y);
            assert_eq!(
                (&top.mz, &top.it),
                (&want.0, &want.1),
                "cap {cap} after {n}"
            );
        }
    }
}
