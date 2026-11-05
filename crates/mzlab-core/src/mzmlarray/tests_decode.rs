//! `decode_array` against the code before it decoded first and filtered white space only on failure and converted one loop per number type.

use std::io::Write;

use super::*;

/// The previous `decode_array`: a white-space scan of the whole text first, one `match` on the type per value.
fn old(
    num: Num,
    zlib: bool,
    text: &str,
    expected: usize,
    inf: &mut Inflate,
    want: Want,
) -> Result<(Vec<f64>, Vec<f32>)> {
    let spaced = text
        .bytes()
        .any(|b| matches!(b, b'\t'..=b'\r' | b' ' | 0x80..));
    let clean: String;
    let bytes_in: &[u8] = if spaced {
        clean = text.chars().filter(|c| !c.is_whitespace()).collect();
        clean.as_bytes()
    } else {
        text.as_bytes()
    };
    if bytes_in.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let raw = base64::engine::general_purpose::STANDARD
        .decode(bytes_in)
        .map_err(|e| Error::new("err.file.base64").with("detail", e))?;
    let bytes: &[u8] = if zlib {
        let limit = match expected.saturating_mul(num.width()) {
            0 => 64 << 20,
            n => n,
        };
        inf.run(&raw, limit)?;
        inf.out()
    } else {
        &raw
    };
    let chunks = bytes.chunks_exact(num.width());
    let (mut f64s, mut f32s) = (Vec::new(), Vec::new());
    for c in chunks {
        let v = num.value(c);
        if want != Want::F32 {
            f64s.push(v);
        }
        if want != Want::F64 {
            f32s.push(narrow(v));
        }
    }
    Ok((f64s, f32s))
}

fn b64(b: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(b)
}

fn zlib(b: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(2));
    e.write_all(b).unwrap();
    e.finish().unwrap()
}

/// The same values (bit for bit) or the same error text.
fn same(a: Result<(Vec<f64>, Vec<f32>)>, b: Result<(Vec<f64>, Vec<f32>)>, what: &str) {
    match (a, b) {
        (Ok((x, y)), Ok((u, v))) => {
            assert_eq!(
                x.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                u.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                "{what}"
            );
            assert_eq!(
                y.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                v.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                "{what}"
            );
        }
        (Err(e), Err(f)) => assert_eq!(format!("{e:?}"), format!("{f:?}"), "{what}"),
        (a, b) => panic!("{what}: {:?} / {:?}", a.is_ok(), b.is_ok()),
    }
}

#[test]
fn decode_array_is_unchanged() {
    // bytes that read as NaN, infinities, subnormals and negative zero in every width, plus a partial last value
    let mut bytes: Vec<u8> = (0..4099u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 13) as u8)
        .collect();
    bytes.extend_from_slice(&f64::NAN.to_le_bytes());
    bytes.extend_from_slice(&(-0.0f64).to_le_bytes());
    bytes.extend_from_slice(&f32::INFINITY.to_le_bytes());
    bytes.extend_from_slice(&[1, 2, 3]);
    let plain = b64(&bytes);
    let packed = b64(&zlib(&bytes));
    let wrap = |t: &str| {
        t.as_bytes()
            .chunks(76)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<_>>()
            .join("\r\n ")
    };
    let mut texts: Vec<(bool, String)> = vec![
        (false, plain.clone()),
        (true, packed.clone()),
        (false, wrap(&plain)),
        (true, format!("\n\t{}\n", wrap(&packed))),
        (false, format!("{}\u{a0}", plain)),
        (false, format!("{}é", plain)),
        (false, String::new()),
        (false, " \n\t ".into()),
        (true, String::new()),
        (false, "AAA*".into()),
        (false, "A A=A".into()),
        (true, b64(b"not zlib")),
        (true, b64(&zlib(&bytes)[..20])),
        (false, b64(&[7u8; 7])),
    ];
    texts.push((true, b64(&zlib(&[]))));
    for (z, text) in &texts {
        for num in [Num::F32, Num::F64, Num::I32, Num::I64] {
            for want in [Want::F64, Want::F32, Want::Both] {
                for expected in [0usize, 3, bytes.len() / num.width(), 1 << 20] {
                    let (mut i, mut j) = (Inflate::new(), Inflate::new());
                    same(
                        old(num, *z, text, expected, &mut i, want),
                        decode_array(num, *z, text, expected, &mut j, want),
                        &format!(
                            "zlib {z} width {} expected {expected} text {:.20}",
                            num.width(),
                            text
                        ),
                    );
                }
            }
        }
    }
}
