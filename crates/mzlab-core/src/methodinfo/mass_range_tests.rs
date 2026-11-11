//! The counted grammar of `MassRangeEx` on streams built here: exactly the announced entries, every other length an error.

use super::*;

fn entry(q1: f32, q3: f32, name: &str, params: &[(&str, f32)]) -> Vec<u8> {
    let mut b = Vec::new();
    for v in [q1, 0.0, q3, 25.0] {
        b.extend(v.to_le_bytes());
    }
    b.extend([0; 4]);
    let nm: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
    b.extend((nm.len() as u16).to_le_bytes());
    b.extend(nm);
    for (id, v) in params {
        let t: Vec<u8> = id.encode_utf16().flat_map(u16::to_le_bytes).collect();
        b.extend((t.len() as u16).to_le_bytes());
        b.extend(t);
        for x in [*v, *v, 0.0] {
            b.extend(x.to_le_bytes());
        }
    }
    b
}

fn stream(n_params: u32, entries: &[Vec<u8>]) -> Vec<u8> {
    let mut d = vec![0u8; 40];
    d[32..36].copy_from_slice(&2u32.to_le_bytes());
    d[36..40].copy_from_slice(&n_params.to_le_bytes());
    entries.iter().for_each(|e| d.extend(e));
    d
}

#[test]
fn exactly_the_announced_entries_are_read() {
    let p = [("DP", 50.0), ("CE", 20.0)];
    let d = stream(
        2,
        &[
            entry(364.1, 194.1, "Quant", &p),
            entry(364.1, 152.1, "", &p),
        ],
    );
    let e = parse(&d, 2).unwrap();
    assert_eq!((e[0].name.as_str(), e[1].name.as_str()), ("Quant", ""));
    assert_eq!(
        (e[1].m0, e[1].m2, e[1].params[1].start),
        (364.1, 152.1, 20.0)
    );
    assert!(parse(&d, 1).unwrap_err().contains("end at byte"));
    assert!(parse(&d, 3).unwrap_err().contains("past the end"));
    let mut long = d.clone();
    long.push(0);
    assert!(parse(&long, 2).is_err());
    assert!(parse(&d[..30], 0).is_err());
}
