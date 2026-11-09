//! The edge cases of a `.mzML.gz`, app against site: the app and the CLI inflate it in `aperti/mzml_gz.rs` and open it, the page streams it through
//! `mzlab_core::gunzip` (`rust-carica.js` `inflate`: a stream that did not end well and gave no byte is `err.file.zlib`) and opens the mzML as a plain
//! upload (`upload::prepare`). Empty, magic bytes only, header only, cut in the middle, padded with zeros after the member: both give the same key.
#![cfg(feature = "aperti")]

mod comune;

use comune::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use mzlab_core::lettori::Limits;
use std::io::Write;

fn gz(b: &[u8]) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(b).unwrap();
    e.finish().unwrap()
}

/// The key the app gives (`"ok"` when the file opens and its index reads).
fn app(b: &[u8]) -> String {
    match mzlab_lettori::apri(file_set("x.mzML.gz", b.to_vec()), &Limits::sito()) {
        Ok(mut l) => match l.index(0) {
            Ok(_) => "ok".into(),
            Err(e) => e.key.to_string(),
        },
        Err(e) => e.key.to_string(),
    }
}

/// The key the page gives: the inflate of `rust-carica.js`, then the plain upload of what it wrote.
fn site(b: &[u8]) -> String {
    let mut g = mzlab_core::gunzip::Gunzip::new(4 << 30);
    let mut out = Vec::new();
    for p in b.chunks(333) {
        match g.push(p) {
            Ok(v) => out.extend(v),
            Err(e) => return e.key.to_string(),
        }
    }
    let (remainder, whole) = match g.finish() {
        Ok(f) => f,
        Err(e) => return e.key.to_string(),
    };
    out.extend(remainder);
    if !whole && g.written() == 0 {
        return "err.file.zlib".into();
    }
    let n = out.len() as u64;
    match mzlab_core::upload::prepare(&mut out, n, "x.mzML") {
        Ok(_) => "ok".into(),
        Err(e) => e.key.to_string(),
    }
}

#[test]
fn gzip_edges_give_the_same_key_on_app_and_site() {
    let good = gz(&mzml_di(&scans(), Cod::Piano, None));
    let mut with_zeros = good.clone();
    with_zeros.extend([0u8; 512]);
    let mut zeros_and_other = with_zeros.clone();
    zeros_and_other.extend(gz(b"<mzML/>"));
    let cases: [(&str, Vec<u8>); 7] = [
        ("good", good.clone()),
        ("empty", Vec::new()),
        ("magic only", vec![0x1f, 0x8b]),
        ("header only", good[..10].to_vec()),
        ("cut in the middle", good[..good.len() / 2].to_vec()),
        ("zeros after the member", with_zeros.clone()),
        ("a member after the zeros", zeros_and_other.clone()),
    ];
    let mut different = Vec::new();
    for (name, b) in cases {
        let (a, s) = (app(&b), site(&b));
        if a != s {
            different.push(format!("{name}: app {a}, site {s}"));
        }
    }
    assert!(different.is_empty(), "{different:#?}");
    assert_eq!(app(&good), "ok");
    // padding to a block size, as GNU gzip reads it: the file opens; something after the padding is damage
    assert_eq!(app(&with_zeros), "ok");
    assert_eq!(app(&zeros_and_other), "err.file.zlib");
}
