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

#[test]
fn a_tiny_fixed_point_is_an_error_not_infinity() {
    // the fixed point of a damaged array: finite and not zero (subnormal), but it divides (linear) or scales the exponent (slof) into infinity
    let tiny = 1e-320f64.to_be_bytes();
    let lin = [&tiny[..], &7u32.to_le_bytes()[..], &9u32.to_le_bytes()[..]].concat();
    assert_eq!(
        decode(Np::Linear, &lin, 2).unwrap_err().key,
        "err.file.numpress"
    );
    let slof = [&tiny[..], &[0xff, 0xff][..]].concat();
    assert_eq!(
        decode(Np::Slof, &slof, 1).unwrap_err().key,
        "err.file.numpress"
    );
    // a negative fixed point gives finite values: the file's own numbers, read as they are
    let neg = [&(-1000f64).to_be_bytes()[..], &7u32.to_le_bytes()[..]].concat();
    assert_eq!(decode(Np::Linear, &neg, 1).unwrap(), vec![-0.007]);
}

// The site's wasm takes `exp` and `ln` from libm, a native build from the system library, and the two differ in the last bit on
// about one value in ten: slof decodes and writes the same bits on every face only if it uses libm everywhere.
#[test]
fn slof_gives_the_bits_of_libm_on_every_face() {
    for max in [1.0, 999.0, 2.5e5, 3.7e7, 1e12] {
        let fp = slof_fixed_point(&[max]);
        assert_eq!(
            fp,
            (65535.0 / libm::log(max + 1.0)).floor().clamp(1.0, 65535.0),
            "fixed point of {max}"
        );
        let body: Vec<u8> = (0..=u16::MAX).flat_map(u16::to_le_bytes).collect();
        let data = [fp.to_be_bytes().as_slice(), &body].concat();
        let back = decode(Np::Slof, &data, 1 << 16).unwrap();
        for (q, b) in back.iter().enumerate() {
            assert_eq!(
                b.to_bits(),
                (libm::exp(q as f64 / fp) - 1.0).to_bits(),
                "decode {q}/{fp}"
            );
        }
        let v: Vec<f64> = (0..20_000)
            .map(|i| f64::from(i) * max / 20_000.0 + 0.37)
            .collect();
        let enc = encode_slof(&v, fp);
        for (x, c) in v.iter().zip(enc[8..].chunks_exact(2)) {
            let want = (libm::log(x + 1.0) * fp + 0.5) as u16;
            assert_eq!(u16::from_le_bytes([c[0], c[1]]), want, "encode {x}");
        }
    }
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// Streams written by the reference C++ (through pyopenms 3.6, an independent build): an odd number of half bytes ends with the pad 0x0, which its decoder drops;
/// a pad 0x8 would read back as one more value. The linear fixed point is the reference's own, taken from the stream.
const REFERENCE: [(Np, &[f64], &str); 9] = [
    (
        Np::Linear,
        &[100.0, 101.0, 102.0, 103.0],
        "417446f8600000005890bb7edeffff7f88",
    ),
    (
        Np::Linear,
        &[100.0, 101.0, 102.5],
        "417446f8600000005890bb7edeffff7f23c732a0",
    ),
    (
        Np::Linear,
        &[1.0, 2.0, 3.0],
        "41cfffffff800000ffffff3ffeffff7f80",
    ),
    (
        Np::Linear,
        &[0.0; 5],
        "41dfffffffc0000000000000000000008880",
    ),
    (
        Np::Linear,
        &[1.0, 2.0, 3.0, 5.0],
        "41cfffffff800000ffffff3ffeffff7f80fffffff3",
    ),
    (Np::Pic, &[100.0, 101.0, 102.0, 103.0], "646656666676"),
    (Np::Pic, &[100.0, 101.0, 103.0], "6466566760"),
    (Np::Pic, &[0.0; 5], "888880"),
    (Np::Pic, &[1.0, 2.0, 3.0], "717273"),
];

#[test]
fn the_encoders_write_the_reference_bytes() {
    for (np, v, hex) in REFERENCE {
        let want = unhex(hex);
        let got = match np {
            Np::Linear => encode_linear(v, f64::from_be_bytes(want[..8].try_into().unwrap())),
            _ => encode_pic(v),
        };
        assert_eq!(got, want, "{np:?} {v:?}");
    }
}

#[test]
fn reference_streams_decode_to_the_declared_count() {
    for (np, v, hex) in REFERENCE {
        for expected in [v.len(), 0] {
            let back = decode(np, &unhex(hex), expected).unwrap();
            assert_eq!(back.len(), v.len(), "{np:?} {v:?} declared {expected}");
            for (a, b) in v.iter().zip(&back) {
                assert!((a - b).abs() < 1e-6, "{np:?} {a} {b}");
            }
        }
    }
}

#[test]
fn older_streams_padded_with_0x8_still_decode() {
    // the same streams as mzLab wrote them before matching the reference: the last half byte 0x8
    for (np, v, hex) in [
        (
            Np::Linear,
            &[1.0, 2.0, 3.0][..],
            "41cfffffff800000ffffff3ffeffff7f88",
        ),
        (Np::Pic, &[100.0, 101.0, 103.0][..], "6466566768"),
        (Np::Pic, &[0.0; 5][..], "888888"),
    ] {
        let back = decode(np, &unhex(hex), v.len()).unwrap();
        assert_eq!(back.len(), v.len(), "{np:?} {v:?}");
    }
}
