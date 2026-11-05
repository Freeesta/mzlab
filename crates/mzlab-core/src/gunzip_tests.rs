//! The streaming gunzip of the page against the reader of the app: same bytes for any piece size, members, zero padding, damage.

use super::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Read;

fn gz(dati: &[u8]) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(dati).unwrap();
    e.finish().unwrap()
}

fn testo(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| format!("<spectrum index=\"{i}\" id=\"scan={i}\"/>\n"))
        .collect::<String>()
        .into_bytes()
}

fn a_pezzi(file: &[u8], pezzo: usize) -> (Vec<u8>, bool) {
    let mut g = Gunzip::new(u64::MAX);
    let mut out = Vec::new();
    for p in file.chunks(pezzo) {
        out.extend(g.push(p).unwrap());
    }
    let (resto, intero) = g.finish().unwrap();
    out.extend(resto);
    (out, intero)
}

#[test]
fn same_bytes_as_the_reader_of_the_app_for_any_piece_size() {
    let a = testo(20_000);
    let b = testo(3_000);
    let mut file = gz(&a);
    file.extend(gz(&b));
    let mut atteso = Vec::new();
    flate2::read::MultiGzDecoder::new(&file[..])
        .read_to_end(&mut atteso)
        .unwrap();
    assert_eq!(app(&file).unwrap(), atteso);
    assert_eq!(atteso.len(), a.len() + b.len());
    for pezzo in [1, 7, 4096, 1 << 20, file.len()] {
        let (out, intero) = a_pezzi(&file, pezzo);
        assert!(intero);
        assert_eq!(out, atteso, "piece {pezzo}");
    }
}

#[test]
fn a_cut_file_keeps_what_it_inflated() {
    let file = gz(&testo(20_000));
    let (out, intero) = a_pezzi(&file[..file.len() / 2], 1000);
    assert!(!intero);
    assert!(!out.is_empty());
    assert!(testo(20_000).starts_with(&out));
}

#[test]
fn not_gzip_and_over_the_cap_are_errors() {
    let mut g = Gunzip::new(u64::MAX);
    assert_eq!(
        g.push(b"<mzML>not gzip</mzML>").unwrap_err().key,
        "err.file.zlib"
    );
    let mut g = Gunzip::new(1000);
    let e = g.push(&gz(&testo(1000))).unwrap_err();
    assert_eq!(e.key, "err.memory.over4gb");
}

fn app(file: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    Membri::new(file).read_to_end(&mut out).map(|_| out)
}

#[test]
fn zero_padding_after_a_member_ends_the_stream_on_both_sides() {
    let a = testo(5_000);
    let mut file = gz(&a);
    file.extend(gz(&testo(7)));
    let atteso = app(&file).unwrap();
    file.extend([0u8; 1000]);
    assert_eq!(app(&file).unwrap(), atteso);
    for pezzo in [1, 7, 4096, file.len()] {
        assert_eq!(
            a_pezzi(&file, pezzo),
            (atteso.clone(), true),
            "piece {pezzo}"
        );
    }
}

#[test]
fn data_after_the_padding_is_an_error_on_both_sides() {
    let mut file = gz(&testo(100));
    file.extend([0u8; 10]);
    file.extend(gz(&testo(3)));
    assert!(app(&file).is_err());
    for pezzo in [1, 7, file.len()] {
        let mut g = Gunzip::new(u64::MAX);
        let e = file.chunks(pezzo).map(|p| g.push(p)).find_map(|r| r.err());
        assert_eq!(e.map(|e| e.key), Some("err.file.zlib"), "piece {pezzo}");
    }
}
