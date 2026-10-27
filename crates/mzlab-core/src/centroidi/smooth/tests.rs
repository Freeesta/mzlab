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
