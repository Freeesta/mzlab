use super::*;
use crate::centroidi::{Mode, Params};

fn params(step: f64) -> Params {
    Params {
        mode: Mode::Counts,
        unit: 1.0,
        step,
        w: 0.7,
        k: 3.0,
        vmax: 0.0,
        area: true,
    }
}

fn next(s: &mut u64) -> f64 {
    *s ^= *s >> 12;
    *s ^= *s << 25;
    *s ^= *s >> 27;
    (s.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
}

/// The grid code as it was before the guards (reference for the bit-identity test).
fn old_fill(mz: &[f64], y: &[f64], step: f64, buf: &mut Buf) {
    buf.x.clear();
    buf.v.clear();
    let regular = mz.iter().all(|&m| {
        let q = (m - mz[0]) / step;
        (q - q.round()).abs() <= 0.05
    });
    if regular {
        let last = ((mz[mz.len() - 1] - mz[0]) / step).round() as usize;
        for b in 0..=last {
            buf.x.push(mz[0] + b as f64 * step);
            buf.v.push(0.0);
        }
        for (i, &m) in mz.iter().enumerate() {
            let b = ((m - mz[0]) / step).round() as usize;
            buf.x[b] = m;
            buf.v[b] += y[i];
        }
    } else {
        sparse(mz, y, step, buf);
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[test]
fn same_bits_as_before_on_regular_and_irregular_scans() {
    let mut s = 12345u64;
    for case in 0..400 {
        let step = [0.06, 0.05, 0.01, 1.0][case % 4];
        let n = 1 + (next(&mut s) * 60.0) as usize;
        let start = 50.0 + next(&mut s) * 100.0;
        let irregular = case % 3 == 0;
        let mut b = 0usize;
        let (mut mz, mut y) = (vec![], vec![]);
        for _ in 0..n {
            b += 1 + (next(&mut s) * 4.0) as usize;
            let jitter = if irregular {
                next(&mut s) * 0.5 * step
            } else {
                0.0
            };
            mz.push(start + b as f64 * step + jitter);
            y.push((next(&mut s) * 1000.0).round());
        }
        let (mut a, mut c) = (Buf::default(), Buf::default());
        fill(&mz, &y, &params(step), &mut a);
        old_fill(&mz, &y, step, &mut c);
        assert_eq!(bits(&a.x), bits(&c.x), "x, case {case}");
        assert_eq!(bits(&a.v), bits(&c.v), "v, case {case}");
    }
}

#[test]
fn unusable_steps_ascending_order_and_huge_ranges_do_not_panic() {
    let mut buf = Buf::default();
    let (mz, y) = (vec![100.0, 100.06, 100.12], vec![1.0, 2.0, 3.0]);
    for step in [0.0, -0.06, f64::NAN, f64::INFINITY] {
        fill(&mz, &y, &params(step), &mut buf);
        assert!(buf.x.len() >= 3 && buf.x.len() == buf.v.len());
    }
    let (desc, y) = (vec![100.12, 100.06, 100.0], vec![1.0, 2.0, 3.0]);
    fill(&desc, &y, &params(0.06), &mut buf);
    assert_eq!(buf.x.len(), 3);
    let (far, y) = (vec![1.0, 1e12], vec![1.0, 1.0]);
    fill(&far, &y, &params(0.06), &mut buf);
    assert!(buf.x.len() <= 4);
    let (inf, y) = (vec![1.0, f64::INFINITY, f64::NAN], vec![1.0; 3]);
    fill(&inf, &y, &params(0.06), &mut buf);
    assert_eq!(buf.x.len(), buf.v.len());
}
