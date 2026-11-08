//! The edge cases of a `.mzML.gz`, app against site: the app and the CLI inflate it in `aperti/mzml_gz.rs` and open it, the page streams it through
//! `mzlab_core::gunzip` (`rust-carica.js` `inflate`: a stream that did not end well and gave no byte is `err.file.zlib`) and opens the mzML as a plain
//! upload (`upload::prepare`). Empty, magic bytes only, header only, cut in the middle, padded with zeros after the member: both give the same key.
#![cfg(feature = "aperti")]

mod comune;

use comune::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use mzlab_core::lettori::Limiti;
use std::io::Write;

fn gz(b: &[u8]) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(b).unwrap();
    e.finish().unwrap()
}

/// The key the app gives (`"ok"` when the file opens and its index reads).
fn app(b: &[u8]) -> String {
    match mzlab_lettori::apri(insieme("x.mzML.gz", b.to_vec()), &Limiti::sito()) {
        Ok(mut l) => match l.index(0) {
            Ok(_) => "ok".into(),
            Err(e) => e.key.to_string(),
        },
        Err(e) => e.key.to_string(),
    }
}

/// The key the page gives: the inflate of `rust-carica.js`, then the plain upload of what it wrote.
fn sito(b: &[u8]) -> String {
    let mut g = mzlab_core::gunzip::Gunzip::new(4 << 30);
    let mut out = Vec::new();
    for p in b.chunks(333) {
        match g.push(p) {
            Ok(v) => out.extend(v),
            Err(e) => return e.key.to_string(),
        }
    }
    let (resto, intero) = match g.finish() {
        Ok(f) => f,
        Err(e) => return e.key.to_string(),
    };
    out.extend(resto);
    if !intero && g.written() == 0 {
        return "err.file.zlib".into();
    }
    let n = out.len() as u64;
    match mzlab_core::upload::prepare(&mut out, n, "x.mzML") {
        Ok(_) => "ok".into(),
        Err(e) => e.key.to_string(),
    }
}

#[test]
fn i_bordi_del_gzip_danno_la_stessa_chiave_su_app_e_sito() {
    let buono = gz(&mzml_di(&scansioni(), Cod::Piano, None));
    let mut con_zeri = buono.clone();
    con_zeri.extend([0u8; 512]);
    let mut zeri_e_altro = con_zeri.clone();
    zeri_e_altro.extend(gz(b"<mzML/>"));
    let casi: [(&str, Vec<u8>); 7] = [
        ("good", buono.clone()),
        ("empty", Vec::new()),
        ("magic only", vec![0x1f, 0x8b]),
        ("header only", buono[..10].to_vec()),
        ("cut in the middle", buono[..buono.len() / 2].to_vec()),
        ("zeros after the member", con_zeri.clone()),
        ("a member after the zeros", zeri_e_altro.clone()),
    ];
    let mut diversi = Vec::new();
    for (nome, b) in casi {
        let (a, s) = (app(&b), sito(&b));
        if a != s {
            diversi.push(format!("{nome}: app {a}, site {s}"));
        }
    }
    assert!(diversi.is_empty(), "{diversi:#?}");
    assert_eq!(app(&buono), "ok");
    // padding to a block size, as GNU gzip reads it: the file opens; something after the padding is damage
    assert_eq!(app(&con_zeri), "ok");
    assert_eq!(app(&zeri_e_altro), "err.file.zlib");
}
