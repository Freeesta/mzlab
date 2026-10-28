use super::*;

fn mzs() -> Vec<f64> {
    (0..500)
        .map(|i| 100.0 + f64::from(i) * 0.37 + f64::from(i % 7) * 0.001)
        .collect()
}

#[test]
fn linear_round_trip_within_half_a_step() {
    let v = mzs();
    let fp = linear_fixed_point(&v);
    let back = decode(Np::Linear, &encode_linear(&v, fp), v.len()).unwrap();
    assert_eq!(back.len(), v.len());
    for (a, b) in v.iter().zip(&back) {
        assert!((a - b).abs() <= 0.5 / fp + 1e-12, "{a} {b}");
    }
}

#[test]
fn linear_short_inputs() {
    for n in 0..4 {
        let v: Vec<f64> = (0..n).map(|i| 50.0 + f64::from(i)).collect();
        let back = decode(Np::Linear, &encode_linear(&v, 1000.0), n as usize).unwrap();
        assert_eq!(back, v, "{n}");
    }
}

#[test]
fn pic_round_trip_is_the_rounded_integer() {
    let v = [
        0.0,
        1.0,
        7.0,
        15.0,
        16.0,
        255.0,
        4096.0,
        123456.0,
        4_000_000.0,
        3.0,
    ];
    assert_eq!(decode(Np::Pic, &encode_pic(&v), 100).unwrap(), v);
    let w = [0.4, 2.6, 9.5];
    assert_eq!(
        decode(Np::Pic, &encode_pic(&w), 100).unwrap(),
        [0.0, 3.0, 10.0]
    );
}

#[test]
fn pic_odd_and_even_counts_of_halves() {
    for n in 1..40 {
        let v: Vec<f64> = (0..n).map(|i| f64::from(i * i * 13)).collect();
        assert_eq!(decode(Np::Pic, &encode_pic(&v), 100).unwrap(), v, "{n}");
    }
}

#[test]
fn slof_relative_error_is_declared() {
    let v: Vec<f64> = (0..300).map(|i| f64::from(i).powi(3) * 1.7).collect();
    let fp = slof_fixed_point(&v);
    let back = decode(Np::Slof, &encode_slof(&v, fp), v.len()).unwrap();
    for (a, b) in v.iter().zip(&back) {
        // x is rounded to 1/fp in the log domain: relative error <= exp(0.5/fp) - 1
        assert!(
            (a - b).abs() <= (a + 1.0) * ((0.5 / fp).exp() - 1.0) + 1e-9,
            "{a} {b}"
        );
    }
}

#[test]
fn hostile_streams_are_errors() {
    // more values than declared
    let v = mzs();
    assert!(decode(Np::Linear, &encode_linear(&v, 1000.0), 10).is_err());
    assert!(decode(Np::Pic, &encode_pic(&v), 10).is_err());
    assert!(decode(Np::Slof, &encode_slof(&v, 1000.0), 10).is_err());
    // no fixed point, a fixed point that is not a number, an odd slof body, a cut integer
    assert!(decode(Np::Linear, &[1, 2, 3], 10).is_err());
    assert!(decode(Np::Slof, &[0; 8], 10).is_err());
    assert!(decode(Np::Slof, &f64::NAN.to_be_bytes(), 10).is_err());
    assert!(decode(
        Np::Slof,
        &[&1000f64.to_be_bytes()[..], &[1][..]].concat(),
        10
    )
    .is_err());
    assert!(decode(Np::Pic, &[0x00], 10).is_err());
    assert!(decode(
        Np::Linear,
        &[&1000f64.to_be_bytes()[..], &[0; 8][..], &[0x0f][..]].concat(),
        10
    )
    .is_err());
}
