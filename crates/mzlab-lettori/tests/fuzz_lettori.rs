//! The entry points of the fuzz targets of the readers modules (`fuzz/ingressi_lettori.rs`) in the ordinary tests: every family's entry returns on empty, short, pseudo-random and
//! truncated inputs (the coverage-guided search is `.github/workflows/fuzz.yml`).
#![cfg(feature = "tutte")]

mod comune;
#[path = "../../../fuzz/ingressi_lettori.rs"]
#[allow(dead_code)]
mod ingressi_lettori;

use comune::*;
use mzlab_core::numpress::Np;

fn casuali(n: usize, seme: u64) -> Vec<u8> {
    let mut x = seme | 1;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect()
}

/// A fuzz entry: its name and the function it runs on the bytes.
type Entrata = (&'static str, fn(&[u8]));

fn entrate() -> Vec<Entrata> {
    vec![
        ("aperti", ingressi_lettori::lettori_aperti),
        ("andi", ingressi_lettori::lettori_andi),
        ("thermo", ingressi_lettori::lettori_thermo),
        ("agilent", ingressi_lettori::lettori_agilent),
        ("sciex", ingressi_lettori::lettori_sciex),
        ("waters", ingressi_lettori::lettori_waters),
        ("shimadzu", ingressi_lettori::lettori_shimadzu),
        ("bruker", ingressi_lettori::lettori_bruker),
    ]
}

#[test]
fn ogni_bersaglio_ritorna_su_byte_qualsiasi() {
    for (nome, f) in entrate() {
        f(&[]);
        f(&[0]);
        for seme in 1..6 {
            for n in [3, 17, 4096] {
                f(&casuali(n, seme * 7919 + n as u64));
            }
        }
        // a header-like start: the readers get past `riconosci` and into their own parsing
        let mut testa = b"<mzML><spectrum>".to_vec();
        testa.extend(casuali(512, 99));
        f(&testa);
        eprintln!("fuzz entry {nome}: ok");
    }
}

#[test]
fn un_mzml_numpress_vero_e_i_suoi_troncamenti_ritornano() {
    let scan = scansioni();
    let file = mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: false,
        },
        None,
    );
    for quarti in 1..4 {
        ingressi_lettori::lettori_aperti(&file[..file.len() * quarti / 4]);
    }
    ingressi_lettori::lettori_aperti(&file);
}
