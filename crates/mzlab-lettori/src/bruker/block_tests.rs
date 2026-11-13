//! LZF references, the zstd window and the empty scans of an LZF frame of `analysis.tdf_bin`.
#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn lzf_literals_short_and_long_references() {
    let mut out = Vec::new();
    // a literal of 3, a reference of 3 bytes 3 back, a long reference (length nibble 7 + 1) of 10 bytes 1 back
    lzf(
        &[0x02, b'a', b'b', b'c', 0x20, 2, 0xe0, 1, 0],
        &mut out,
        100,
    )
    .unwrap();
    assert_eq!(out, b"abcabccccccccccc");
    assert!(lzf(&[0x02, b'a', b'b', b'c'], &mut Vec::new(), 2).is_err());
}

#[test]
fn a_zstd_window_counts_the_dictionary_id_before_the_size() {
    // single segment with a 4-byte dictionary id: the 1-byte content size follows it
    let z = [0x28, 0xb5, 0x2f, 0xfd, 0x23, 9, 9, 9, 9, 200];
    assert_eq!(zstd_window(&z), Some(200));
    // single segment, 2-byte content size: stored minus 256
    assert_eq!(
        zstd_window(&[0x28, 0xb5, 0x2f, 0xfd, 0x60, 0x10, 0x00]),
        Some(272)
    );
    // a window descriptor: 2^(10 + exponent) plus eighths
    assert_eq!(
        zstd_window(&[0x28, 0xb5, 0x2f, 0xfd, 0x00, 0x09]),
        Some(2048 + 256)
    );
    assert_eq!(zstd_window(&[0, 0, 0, 0, 0]), None);
}

#[test]
fn an_lzf_frame_keeps_its_empty_scans() {
    // 2 scans: the first empty, the second the words 5 and -2 and 7 (intensity 5 at bin 0, skip 2, intensity 7 at bin 3)
    let words: Vec<u8> = [5i32, -2, 7].iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut blob = vec![u8::try_from(words.len() - 1).unwrap()];
    blob.extend(&words);
    let start = 8 + 3 * 4;
    let mut b = vec![0u8; 8];
    for o in [start, start, start + blob.len()] {
        b.extend(u32::try_from(o).unwrap().to_le_bytes());
    }
    b.extend(&blob);
    let p = decode1(&b, 2, 2, &Limits::default()).unwrap();
    assert_eq!(
        (p.start, p.tof, p.int),
        (vec![0, 0, 2], vec![0, 3], vec![5, 7])
    );
}
