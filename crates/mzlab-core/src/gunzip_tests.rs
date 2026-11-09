//! The streaming gunzip of the page against the reader of the app: same bytes for any piece size, members, zero padding, damage.

use super::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Read;

fn gz(data: &[u8]) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

fn text(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| format!("<spectrum index=\"{i}\" id=\"scan={i}\"/>\n"))
        .collect::<String>()
        .into_bytes()
}

fn a_pezzi(file: &[u8], piece: usize) -> (Vec<u8>, bool) {
    let mut g = Gunzip::new(u64::MAX);
    let mut out = Vec::new();
    for p in file.chunks(piece) {
        out.extend(g.push(p).unwrap());
    }
    let (rest, integer) = g.finish().unwrap();
    out.extend(rest);
    (out, integer)
}

#[test]
fn same_bytes_as_the_reader_of_the_app_for_any_piece_size() {
    let a = text(20_000);
    let b = text(3_000);
    let mut file = gz(&a);
    file.extend(gz(&b));
    let mut expected = Vec::new();
    flate2::read::MultiGzDecoder::new(&file[..])
        .read_to_end(&mut expected)
        .unwrap();
    assert_eq!(app(&file).unwrap(), expected);
    assert_eq!(expected.len(), a.len() + b.len());
    for piece in [1, 7, 4096, 1 << 20, file.len()] {
        let (out, integer) = a_pezzi(&file, piece);
        assert!(integer);
        assert_eq!(out, expected, "piece {piece}");
    }
}

#[test]
fn a_cut_file_keeps_what_it_inflated() {
    let file = gz(&text(20_000));
    let (out, integer) = a_pezzi(&file[..file.len() / 2], 1000);
    assert!(!integer);
    assert!(!out.is_empty());
    assert!(text(20_000).starts_with(&out));
}

#[test]
fn not_gzip_and_over_the_cap_are_errors() {
    let mut g = Gunzip::new(u64::MAX);
    assert_eq!(
        g.push(b"<mzML>not gzip</mzML>").unwrap_err().key,
        "err.file.zlib"
    );
    let mut g = Gunzip::new(1000);
    let e = g.push(&gz(&text(1000))).unwrap_err();
    assert_eq!(e.key, "err.memory.over4gb");
}

fn app(file: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    Members::new(file).read_to_end(&mut out).map(|_| out)
}

#[test]
fn zero_padding_after_a_member_ends_the_stream_on_both_sides() {
    let a = text(5_000);
    let mut file = gz(&a);
    file.extend(gz(&text(7)));
    let expected = app(&file).unwrap();
    file.extend([0u8; 1000]);
    assert_eq!(app(&file).unwrap(), expected);
    for piece in [1, 7, 4096, file.len()] {
        assert_eq!(
            a_pezzi(&file, piece),
            (expected.clone(), true),
            "piece {piece}"
        );
    }
}

#[test]
fn data_after_the_padding_is_an_error_on_both_sides() {
    let mut file = gz(&text(100));
    file.extend([0u8; 10]);
    file.extend(gz(&text(3)));
    assert!(app(&file).is_err());
    for piece in [1, 7, file.len()] {
        let mut g = Gunzip::new(u64::MAX);
        let e = file.chunks(piece).map(|p| g.push(p)).find_map(|r| r.err());
        assert_eq!(e.map(|e| e.key), Some("err.file.zlib"), "piece {piece}");
    }
}
