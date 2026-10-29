use super::*;

/// `noise` as it was with the temporary vector of first differences.
fn old_noise(buf: &mut Buf, wb: usize) -> f64 {
    let d: Vec<f64> = buf.v.windows(2).map(|w| w[1] - w[0]).collect();
    let bl = 10 * wb;
    let mut sig = Vec::new();
    for ch in d.chunks(bl).filter(|c| c.len() >= 5) {
        buf.pad.clear();
        buf.pad.extend_from_slice(ch);
        let med = median(&mut buf.pad);
        buf.pad.clear();
        buf.pad.extend(ch.iter().map(|v| (v - med).abs()));
        sig.push(1.4826 * median(&mut buf.pad) / std::f64::consts::SQRT_2);
    }
    if sig.is_empty() {
        buf.pad.clear();
        buf.pad.extend_from_slice(&d);
        if buf.pad.is_empty() {
            return 1e-300;
        }
        let med = median(&mut buf.pad);
        buf.pad.clear();
        buf.pad.extend(d.iter().map(|v| (v - med).abs()));
        sig.push(1.4826 * median(&mut buf.pad) / std::f64::consts::SQRT_2);
    }
    lower(&mut sig, 0.25).max(1e-300)
}

#[test]
fn noise_has_the_same_bits_without_the_difference_vector() {
    let mut s = 7u64;
    for n in (0..40).chain([100, 333, 1000, 1001]) {
        for wb in [1, 2, 3, 7, 12] {
            let mut a = Buf::default();
            for _ in 0..n {
                s ^= s >> 12;
                s ^= s << 25;
                s ^= s >> 27;
                a.v.push((s.wrapping_mul(0x2545F4914F6CDD1D) >> 40) as f64 / 13.0);
            }
            let mut b = Buf {
                v: a.v.clone(),
                ..Buf::default()
            };
            assert_eq!(
                noise(&mut a, wb).to_bits(),
                old_noise(&mut b, wb).to_bits(),
                "n {n} wb {wb}"
            );
        }
    }
}

/// `background` as it was: every block sorts its whole window again.
fn old_background(buf: &mut Buf, wb: usize, mean: bool) -> Vec<f64> {
    let n = buf.s.len();
    let bs = ((2.5 * wb as f64).round() as usize).max(1);
    let half = 5 * wb;
    let nb = n.div_ceil(bs);
    let (mut cen, mut val) = (Vec::new(), Vec::new());
    for b in 0..nb {
        let c = (b * bs + bs / 2).min(n - 1);
        let (lo, hi) = (c.saturating_sub(half), (c + half).min(n - 1));
        let mut tmp = buf.s[lo..=hi].to_vec();
        cen.push(c);
        let q = lower(&mut tmp, 0.25);
        val.push(if mean {
            let k = ((tmp.len() - 1) as f64 * 0.75).floor() as usize;
            tmp[..=k].iter().sum::<f64>() / (k + 1) as f64
        } else {
            q
        });
    }
    let mut out = Vec::new();
    let mut b = 0;
    for j in 0..n {
        while b + 1 < nb && cen[b + 1] <= j {
            b += 1;
        }
        if j <= cen[0] || b + 1 >= nb {
            out.push(if j <= cen[0] { val[0] } else { val[nb - 1] });
        } else {
            let f = (j - cen[b]) as f64 / (cen[b + 1] - cen[b]) as f64;
            out.push(val[b] + f * (val[b + 1] - val[b]));
        }
    }
    out
}

#[test]
fn the_sliding_background_has_the_same_bits_as_sorting_every_window() {
    let mut s = 12345u64;
    for n in (3..60).chain([200, 999, 1000, 4001, 9000]) {
        for wb in [1, 2, 3, 7, 12, 40] {
            for mean in [false, true] {
                let mut a = Buf::default();
                for _ in 0..n {
                    s ^= s >> 12;
                    s ^= s << 25;
                    s ^= s >> 27;
                    let r = s.wrapping_mul(0x2545F4914F6CDD1D) >> 40;
                    // many zeros and repeated values, some large ones
                    a.s.push(if r.is_multiple_of(3) {
                        0.0
                    } else {
                        (r % 17) as f64 / 3.0 + if r.is_multiple_of(101) { 500.0 } else { 0.0 }
                    });
                }
                let want = old_background(&mut a, wb, mean);
                background(&mut a, wb, mean);
                let (x, y): (Vec<u64>, Vec<u64>) = (
                    a.b.iter().map(|v| v.to_bits()).collect(),
                    want.iter().map(|v| v.to_bits()).collect(),
                );
                assert_eq!(x, y, "n={n} wb={wb} mean={mean}");
            }
        }
    }
}
