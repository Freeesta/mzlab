//! Tests of the sibling module (loaded with `#[path]`, so they see its private items).

use super::*;
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// `ridge_lines` as it was: every local maximum is tried against every ridge in creation order.
fn old_ridge_lines(c: &[Vec<f64>], widths: &[f64]) -> Vec<Ridge> {
    let (nw, n) = (widths.len(), c[0].len());
    let peaks_per_scale: Vec<Vec<usize>> = (0..nw)
        .map(|i| {
            (1..n - 1)
                .filter(|&j| c[i][j] > c[i][j - 1] && c[i][j] >= c[i][j + 1] && c[i][j] > 0.0)
                .collect()
        })
        .collect();
    let mut scratch = Vec::new();
    let scale_noise: Vec<f64> = (0..nw)
        .map(|i| {
            let m = median_in(c[i].iter().copied(), &mut scratch);
            (median_in(c[i].iter().map(|v| (v - m).abs()), &mut scratch) * 1.4826).max(1e-12)
        })
        .collect();
    let mut ridges: Vec<Ridge> = Vec::new();
    for (i, mx) in peaks_per_scale.iter().enumerate() {
        for &pos in mx {
            let mut joined = false;
            for r in ridges.iter_mut() {
                if r.last_scale + 1 == i || r.last_scale + 2 == i {
                    let last = *r.pos.last().unwrap() as f64;
                    if (last - pos as f64).abs() <= (widths[i] / 2.0).max(1.0) {
                        r.pos.push(pos);
                        r.val.push(c[i][pos] / scale_noise[i]);
                        r.scale.push(i);
                        r.last_scale = i;
                        joined = true;
                        break;
                    }
                }
            }
            if !joined {
                ridges.push(Ridge {
                    pos: vec![pos],
                    val: vec![c[i][pos] / scale_noise[i]],
                    scale: vec![i],
                    last_scale: i,
                });
            }
        }
    }
    ridges
}

/// `quad_fit` as it was: the nine entries of the normal matrix each summed with their own `powi`.
fn old_quad_fit(rt: &[f64], y: &[f64], i: usize) -> Fit {
    let n = y.len();
    let a = i.saturating_sub(2);
    let b = (i + 3).min(n);
    if b - a < 3 {
        return Fit::Short;
    }
    let len = b - a;
    let mut xs = [0.0; 5];
    for (x, t) in xs.iter_mut().zip(&rt[a..b]) {
        *x = t - rt[i];
    }
    let mut ata = [0.0; 9];
    let mut aty = [0.0; 3];
    for (x, &yy) in xs[..len].iter().zip(&y[a..b]) {
        for j in 0..3 {
            aty[j] += x.powi(j as i32) * yy;
            for k in 0..3 {
                ata[j * 3 + k] += x.powi((j + k) as i32);
            }
        }
    }
    let Some(c) = solve3(ata, aty) else {
        return Fit::Flat;
    };
    if c[2] >= 0.0 {
        return Fit::Flat;
    }
    let xa = (-c[1] / (2.0 * c[2])).clamp(xs[0], xs[len - 1]);
    let height = c[0] + c[1] * xa + c[2] * xa * xa;
    Fit::Peak(Quad {
        xs,
        len,
        ata,
        c,
        xa,
        height,
    })
}

fn old_trapz(y: &[f64], x: &[f64]) -> f64 {
    (1..y.len())
        .map(|i| (y[i] + y[i - 1]) / 2.0 * (x[i] - x[i - 1]))
        .sum()
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[test]
fn quad_fit_and_the_streaming_area_keep_their_bits() {
    let mut rng = Rng(3);
    for case in 0..2000 {
        let n = 3 + (rng.next() * 30.0) as usize;
        let mut t = 0.0;
        let rt: Vec<f64> = (0..n)
            .map(|_| {
                t += 0.001 + rng.next() * 0.05;
                t
            })
            .collect();
        let y: Vec<f64> = (0..n)
            .map(|i| {
                let c = n as f64 / 2.0;
                1e5 * (-((i as f64 - c) / 2.5).powi(2)).exp() + rng.next() * 50.0 - 10.0
            })
            .collect();
        for i in 0..n {
            match (quad_fit(&rt, &y, i), old_quad_fit(&rt, &y, i)) {
                (Fit::Peak(a), Fit::Peak(b)) => {
                    assert_eq!(bits(&a.ata), bits(&b.ata), "case {case}");
                    assert_eq!(bits(&a.c), bits(&b.c));
                    assert_eq!(
                        (a.xa.to_bits(), a.height.to_bits()),
                        (b.xa.to_bits(), b.height.to_bits())
                    );
                }
                (Fit::Flat, Fit::Flat) | (Fit::Short, Fit::Short) => {}
                _ => panic!("case {case}: the fits disagree at {i}"),
            }
        }
        let (lo, hi) = (rng_idx(&mut rng, n), rng_idx(&mut rng, n));
        let (lo, hi) = (lo.min(hi), lo.max(hi));
        if hi > lo {
            let seg: Vec<f64> = y[lo..=hi].iter().map(|v| v.max(0.0)).collect();
            assert_eq!(
                trapz_pos(&y, &rt, lo, hi).to_bits(),
                old_trapz(&seg, &rt[lo..=hi]).to_bits(),
                "case {case}"
            );
        }
    }
}

fn rng_idx(rng: &mut Rng, n: usize) -> usize {
    ((rng.next() * n as f64) as usize).min(n - 1)
}

fn same(a: &[Ridge], b: &[Ridge]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.pos == y.pos
                && x.scale == y.scale
                && x.last_scale == y.last_scale
                && x.val
                    .iter()
                    .map(|v| v.to_bits())
                    .eq(y.val.iter().map(|v| v.to_bits()))
        })
}

#[test]
fn ridge_lines_join_the_same_maxima_as_the_all_pairs_search() {
    let mut rng = Rng(31);
    let mut long = 0;
    for case in 0..400 {
        let n = 20 + (rng.next() * 400.0) as usize;
        let widths = scale_widths(n);
        let smooth = 1 + case % 5;
        // noisy scales whose maxima are dense, with ties and plateaus
        let c: Vec<Vec<f64>> = (0..widths.len())
            .map(|s| {
                let raw: Vec<f64> = (0..n).map(|_| (rng.next() * 8.0).floor()).collect();
                (0..n)
                    .map(|i| {
                        let a = i.saturating_sub(smooth * s / 3);
                        let b = (i + 1 + smooth * s / 3).min(n);
                        raw[a..b].iter().sum::<f64>() / (b - a) as f64
                    })
                    .collect()
            })
            .collect();
        let (new, old) = (ridge_lines(&c, &widths), old_ridge_lines(&c, &widths));
        assert!(same(&new, &old), "case {case}");
        long += new.iter().filter(|r| r.pos.len() >= 3).count();
    }
    assert!(
        long > 400,
        "the cases must join maxima across scales ({long})"
    );
}
