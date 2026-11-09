//! The recognition of a set (`readers::riconosci`) looks at the first bytes, never at the extension alone: every prefix of every magic, a magic under the
//! wrong name, a declared size smaller or larger than the bytes, and a Thermo `.raw` that is its magic and little else, must give a refusal or a key.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod robustness;

use mzlab_core::readers::{self, FileSet, Limits, Member};
use robustness::*;

#[global_allocator]
static ALLOCATORE: Counter = Counter;

const RAW: [u8; 18] = [
    0x01, 0xA1, b'F', 0, b'i', 0, b'n', 0, b'n', 0, b'i', 0, b'g', 0, b'a', 0, b'n', 0,
];
const OLE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const MS: [u8; 2] = [0x01, 0x32];

fn reads(name: String, b: Vec<u8>, dim: u64) -> Vec<String> {
    let mut k = Vec::new();
    let set = || FileSet {
        membri: vec![Member {
            nome: name.clone(),
            dim,
            dati: Box::new(Letture(b.clone())),
            sha1: None,
        }],
    };
    let _ = readers::riconosci(&mut set());
    if let Some(mut l) = note(readers::apri(set(), &Limits::sito()), &mut k) {
        if let Some(ix) = note(l.index(0), &mut k).map(<[_]>::to_vec) {
            (0..ix.len().min(4)).for_each(|j| drop(note(l.scan(0, j), &mut k)));
        }
        note(l.method(0), &mut k);
    }
    k
}

fn case(name: &str, b: Vec<u8>, dim: Option<u64>) -> Vec<String> {
    let (n, total) = (name.to_string(), b.len());
    let dim = dim.unwrap_or(total as u64);
    one(name, total, Box::new(move || reads(n, b, dim)))
}

#[test]
fn every_prefix_of_every_magic() {
    let mut g = Vec::new();
    for (name, magic) in [
        ("x.raw", &RAW[..]),
        ("x.wiff", &OLE[..]),
        ("x.D/DATA.MS", &MS[..]),
    ] {
        for n in 0..=magic.len() {
            g.extend(case(
                &format!("{name} prefix {n}"),
                magic[..n].to_vec(),
                None,
            ));
        }
    }
    verify("riconoscimento", 29, &g);
}

#[test]
fn a_magic_under_the_wrong_name_and_a_wrong_size() {
    let mut raw = RAW.to_vec();
    raw.extend([0u8; 2000]);
    let mut g = Vec::new();
    for name in ["x.wiff", "x.mzML", "x.D/MSScan.bin", "x", ""] {
        g.extend(case(&format!("raw as {name:?}"), raw.clone(), None));
    }
    g.extend(case("raw declared 0", raw.clone(), Some(0)));
    g.extend(case("raw declared 16 EB", raw.clone(), Some(u64::MAX)));
    g.extend(case("raw of 1 TB declared", raw.clone(), Some(1 << 40)));
    g.extend(case("raw only the magic", RAW.to_vec(), None));
    g.extend(case(
        "raw of zeros after the magic",
        [RAW.to_vec(), vec![0u8; 100_000]].concat(),
        None,
    ));
    g.extend(case(
        "raw of 0xFF after the magic",
        [RAW.to_vec(), vec![0xFFu8; 100_000]].concat(),
        None,
    ));
    verify("riconoscimento", 11, &g);
}
