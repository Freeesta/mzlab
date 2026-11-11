//! The `crc32` of each section of a `.mzidx`: the builder writes one sum per array, whatever the budget; an index without the key (written
//! before the sums) still opens and reads; a byte changed in an array read whole, or a malformed key, is `error.badIndex`, never wrong numbers.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::disallowed_methods)]

mod fake_reader;

use std::io::Cursor;

use fake_reader::Finto;
use mzlab_core::idxread::Index;
use mzlab_core::{mzidx, upload};
use serde_json::Value;

const NAMES: [&str; 10] = [
    "rt", "tic", "scan_ids", "sumint", "bp_mz", "bp_inten", "fence", "mz", "inten", "pos",
];

fn try_build(budget: usize) -> mzlab_core::Result<(Vec<u8>, mzidx::Stats)> {
    let mut l = Finto::vario(300, 200, 4242);
    let p = upload::prepare_reader(&mut l, 0, &[], "x.mzML").unwrap();
    let mut out = Cursor::new(Vec::new());
    let st = mzidx::build_reader(&p.table, &mut l, 0, &mut out, budget)?;
    Ok((out.into_inner(), st))
}

fn build(budget: usize) -> Vec<u8> {
    try_build(budget).unwrap().0
}

fn u64_at(b: &[u8], at: usize) -> usize {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap()) as usize
}

fn meta(b: &[u8]) -> Value {
    let (off, len) = (u64_at(b, 16), u64_at(b, 24));
    serde_json::from_slice(&b[off..off + len]).unwrap()
}

/// The index with its metadata replaced by `m` at the end of the file (the header points at the new place and size).
fn with_meta(b: &[u8], m: &Value) -> Vec<u8> {
    let m = serde_json::to_vec(m).unwrap();
    let mut out = b.to_vec();
    let at = out.len() as u64;
    out.extend_from_slice(&m);
    out[16..24].copy_from_slice(&at.to_le_bytes());
    out[24..32].copy_from_slice(&(m.len() as u64).to_le_bytes());
    let size = out.len() as u64;
    out[48..56].copy_from_slice(&size.to_le_bytes());
    out
}

fn open(b: Vec<u8>) -> mzlab_core::Result<Index<Vec<u8>>> {
    let size = b.len() as u64;
    Index::open(b, size)
}

fn crc(b: &[u8]) -> u32 {
    let mut h = flate2::Crc::new();
    h.update(b);
    h.sum()
}

/// Every array of every section has the CRC-32 of its bytes, with the large budget and with rounds of buckets.
#[test]
fn every_array_has_its_sum() {
    let mut saw_rounds = false;
    for budget in [mzidx::DEFAULT_BUDGET, 1 << 20, 1 << 18, 1 << 16] {
        let Ok((b, st)) = try_build(budget) else {
            continue;
        };
        saw_rounds |= st.passes > 1;
        let m = meta(&b);
        for s in m["sections"].as_array().unwrap() {
            let w = if s["dtype"] == "<f4" { 4 } else { 8 };
            let (n, p, f) = (
                s["n_scans"].as_u64().unwrap(),
                s["n_peaks"].as_u64().unwrap(),
                s["n_fence"].as_u64().unwrap(),
            );
            let lens = [
                8 * n,
                8 * n,
                8 * n,
                8 * n,
                8 * n,
                8 * n,
                w * f,
                w * p,
                w * p,
                4 * p,
            ];
            for (k, len) in NAMES.iter().zip(lens) {
                let at = s["off"][k].as_u64().unwrap() as usize;
                let want = crc(&b[at..at + len as usize]);
                assert_eq!(
                    s["crc32"][k].as_u64(),
                    Some(want as u64),
                    "budget {budget}, array {k}"
                );
            }
        }
    }
    assert!(saw_rounds, "no budget forced the rounds");
}

#[test]
fn new_and_old_indexes_both_open_and_read() {
    let b = build(mzidx::DEFAULT_BUDGET);
    let mut old = meta(&b);
    for s in old["sections"].as_array_mut().unwrap() {
        s.as_object_mut().unwrap().remove("crc32");
    }
    let (mut a, mut z) = (open(b.clone()).unwrap(), open(with_meta(&b, &old)).unwrap());
    for si in 0..a.sections.len() {
        assert_eq!(a.arrays(si).unwrap().rt, z.arrays(si).unwrap().rt);
        assert_eq!(
            a.search(si, 500.0, false).unwrap(),
            z.search(si, 500.0, false).unwrap()
        );
    }
}

fn flipped(b: &[u8], array: &str) -> Vec<u8> {
    let at = meta(b)["sections"][0]["off"][array].as_u64().unwrap() as usize;
    let mut d = b.to_vec();
    d[at + 3] ^= 0x10;
    d
}

#[test]
fn a_changed_byte_in_the_scan_arrays_is_a_bad_index() {
    let b = build(mzidx::DEFAULT_BUDGET);
    for array in ["rt", "scan_ids", "sumint", "bp_mz", "bp_inten"] {
        let mut ix = open(flipped(&b, array)).unwrap();
        let e = ix
            .arrays(0)
            .err()
            .unwrap_or_else(|| panic!("{array}: no error"));
        assert_eq!(e.key, "error.badIndex", "{array}");
    }
}

#[test]
fn a_changed_byte_in_the_fence_is_a_bad_index() {
    let b = build(mzidx::DEFAULT_BUDGET);
    assert!(meta(&b)["sections"][0]["n_peaks"].as_u64().unwrap() > 0);
    let mut ix = open(flipped(&b, "fence")).unwrap();
    assert_eq!(
        ix.search(0, 500.0, false).unwrap_err().key,
        "error.badIndex"
    );
}

#[test]
fn a_malformed_sum_is_a_bad_index() {
    let b = build(mzidx::DEFAULT_BUDGET);
    for bad in [
        Value::from("x"),
        Value::from(-1),
        Value::from(1.5),
        Value::from(1u64 << 33),
    ] {
        let mut m = meta(&b);
        m["sections"][0]["crc32"]["tic"] = bad.clone();
        assert_eq!(
            open(with_meta(&b, &m)).err().unwrap().key,
            "error.badIndex",
            "{bad}"
        );
    }
    let mut m = meta(&b);
    m["sections"][0]["crc32"]
        .as_object_mut()
        .unwrap()
        .remove("pos");
    assert_eq!(open(with_meta(&b, &m)).err().unwrap().key, "error.badIndex");
}
