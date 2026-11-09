use super::*;

/// `second` as it was when it computed its constants on every call.
fn old_second(buf: &Buf, l: usize, j: usize) -> f64 {
    let m = ((l - 1) / 2) as f64;
    let mu = m * (m + 1.0) / 3.0;
    let den: f64 = (0..l).map(|i| ((i as f64 - m).powi(2) - mu).powi(2)).sum();
    let mut acc = 0.0;
    for i in 0..l {
        let t = j as isize + i as isize - (l as isize - 1) / 2;
        if t >= 0 && (t as usize) < buf.v.len() {
            acc += 2.0 * ((i as f64 - m).powi(2) - mu) / den * buf.v[t as usize];
        }
    }
    acc
}

#[test]
fn second_has_the_same_bits_with_hoisted_constants() {
    let mut s = 99u64;
    let mut buf = Buf::default();
    for _ in 0..200 {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        buf.v
            .push((s.wrapping_mul(0x2545F4914F6CDD1D) >> 40) as f64 / 7.0);
    }
    for l in [5, 7, 9, 11, 13, 15] {
        let c = second_consts(l);
        for j in 0..buf.v.len() {
            assert_eq!(
                second(&buf, l, c, j).to_bits(),
                old_second(&buf, l, j).to_bits()
            );
        }
    }
}
