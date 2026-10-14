//! The recognition of a set (`lettori::riconosci`) looks at the first bytes, never at the extension alone: every prefix of every magic, a magic under the
//! wrong name, a declared size smaller or larger than the bytes, and a Thermo `.raw` that is its magic and little else, must give a refusal or a key.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod robustezza;

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use robustezza::*;

#[global_allocator]
static ALLOCATORE: Contatore = Contatore;

const RAW: [u8; 18] = [
    0x01, 0xA1, b'F', 0, b'i', 0, b'n', 0, b'n', 0, b'i', 0, b'g', 0, b'a', 0, b'n', 0,
];
const OLE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const MS: [u8; 2] = [0x01, 0x32];

fn legge(nome: String, b: Vec<u8>, dim: u64) -> Vec<String> {
    let mut k = Vec::new();
    let set = || FileSet {
        membri: vec![Membro {
            nome: nome.clone(),
            dim,
            dati: Box::new(Letture(b.clone())),
        }],
    };
    let _ = lettori::riconosci(&mut set());
    if let Some(mut l) = nota(lettori::apri(set(), &Limiti::sito()), &mut k) {
        if let Some(ix) = nota(l.indice(0), &mut k).map(<[_]>::to_vec) {
            (0..ix.len().min(4)).for_each(|j| drop(nota(l.scansione(0, j), &mut k)));
        }
        nota(l.metodo(0), &mut k);
    }
    k
}

fn caso(nome: &str, b: Vec<u8>, dim: Option<u64>) -> Vec<String> {
    let (n, totale) = (nome.to_string(), b.len());
    let dim = dim.unwrap_or(totale as u64);
    uno(nome, totale, Box::new(move || legge(n, b, dim)))
}

#[test]
fn every_prefix_of_every_magic() {
    let mut g = Vec::new();
    for (nome, magic) in [
        ("x.raw", &RAW[..]),
        ("x.wiff", &OLE[..]),
        ("x.D/DATA.MS", &MS[..]),
    ] {
        for n in 0..=magic.len() {
            g.extend(caso(
                &format!("{nome} prefix {n}"),
                magic[..n].to_vec(),
                None,
            ));
        }
    }
    verifica("riconoscimento", 29, &g);
}

#[test]
fn a_magic_under_the_wrong_name_and_a_wrong_size() {
    let mut raw = RAW.to_vec();
    raw.extend([0u8; 2000]);
    let mut g = Vec::new();
    for nome in ["x.wiff", "x.mzML", "x.D/MSScan.bin", "x", ""] {
        g.extend(caso(&format!("raw as {nome:?}"), raw.clone(), None));
    }
    g.extend(caso("raw declared 0", raw.clone(), Some(0)));
    g.extend(caso("raw declared 16 EB", raw.clone(), Some(u64::MAX)));
    g.extend(caso("raw of 1 TB declared", raw.clone(), Some(1 << 40)));
    g.extend(caso("raw only the magic", RAW.to_vec(), None));
    g.extend(caso(
        "raw of zeros after the magic",
        [RAW.to_vec(), vec![0u8; 100_000]].concat(),
        None,
    ));
    g.extend(caso(
        "raw of 0xFF after the magic",
        [RAW.to_vec(), vec![0xFFu8; 100_000]].concat(),
        None,
    ));
    verifica("riconoscimento", 11, &g);
}
