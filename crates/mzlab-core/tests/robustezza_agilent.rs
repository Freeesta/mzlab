//! The Agilent `.D` reader against folders that are not what they say: no `AcqData`, files of size zero, a declared size of 16 EB, a size that does not divide
//! by the record, `MSPeak.bin` shorter than the records say, a `DATA.MS` that is only its magic.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod agilent_synth;
mod robustezza;

use agilent_synth::{encode, sample};
use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use robustezza::*;

#[global_allocator]
static ALLOCATORE: Contatore = Contatore;

/// A member whose declared size (`dim`) can differ from its bytes.
fn membro(nome: &str, b: &[u8], dim: Option<u64>) -> Membro {
    Membro {
        nome: nome.into(),
        dim: dim.unwrap_or(b.len() as u64),
        dati: Box::new(Letture(b.to_vec())),
        sha1: None,
    }
}

fn legge(membri: Vec<(String, Vec<u8>, Option<u64>)>) -> Vec<String> {
    let mut k = Vec::new();
    let mut fs = FileSet {
        membri: membri.iter().map(|(n, b, d)| membro(n, b, *d)).collect(),
    };
    let _ = lettori::riconosci(&mut fs);
    let fs = FileSet {
        membri: membri.iter().map(|(n, b, d)| membro(n, b, *d)).collect(),
    };
    if let Some(mut l) = nota(lettori::apri(fs, &Limiti::sito()), &mut k) {
        if let Some(ix) = nota(l.indice(0), &mut k).map(<[_]>::to_vec) {
            (0..ix.len()).for_each(|j| drop(nota(l.scansione(0, j), &mut k)));
        }
        nota(l.metodo(0), &mut k);
    }
    k
}

fn caso(nome: &str, m: Vec<(&str, Vec<u8>, Option<u64>)>) -> Vec<String> {
    let totale = m.iter().map(|x| x.1.len()).sum();
    let m = m
        .into_iter()
        .map(|(n, b, d)| (n.to_string(), b, d))
        .collect();
    uno(nome, totale, Box::new(move || legge(m)))
}

#[test]
fn folders_that_are_not_what_they_say() {
    let (idx, peak) = encode(&sample());
    let (scan, pk) = ("x.D/AcqData/MSScan.bin", "x.D/AcqData/MSPeak.bin");
    let mut g = Vec::new();
    g.extend(caso(
        "no AcqData",
        vec![("x.D/readme.txt", b"hello".to_vec(), None)],
    ));
    g.extend(caso("no members", vec![]));
    g.extend(caso(
        "empty files",
        vec![(scan, vec![], None), (pk, vec![], None)],
    ));
    g.extend(caso(
        "declared 16 EB",
        vec![
            (scan, idx.clone(), Some(u64::MAX)),
            (pk, peak.clone(), None),
        ],
    ));
    g.extend(caso(
        "peak declared 16 EB",
        vec![
            (scan, idx.clone(), None),
            (pk, peak.clone(), Some(u64::MAX)),
        ],
    ));
    g.extend(caso(
        "peak declared 4 GB",
        vec![(scan, idx.clone(), None), (pk, peak.clone(), Some(1 << 32))],
    ));
    let mut storto = idx.clone();
    storto.truncate(idx.len() - 7);
    g.extend(caso(
        "size not a multiple of the record",
        vec![(scan, storto, None), (pk, peak.clone(), None)],
    ));
    g.extend(caso(
        "peak shorter than the records",
        vec![
            (scan, idx.clone(), None),
            (pk, peak[..peak.len() / 2].to_vec(), None),
        ],
    ));
    g.extend(caso(
        "peak with the header only",
        vec![(scan, idx.clone(), None), (pk, peak[..68].to_vec(), None)],
    ));
    g.extend(caso(
        "DATA.MS with its magic only",
        vec![("x.D/DATA.MS", vec![0x01, 0x32], None)],
    ));
    g.extend(caso(
        "DATA.MS declared 16 EB",
        vec![("x.D/DATA.MS", vec![0x01, 0x32, 0, 0], Some(u64::MAX))],
    ));
    verifica("agilent", 11, &g);
}
