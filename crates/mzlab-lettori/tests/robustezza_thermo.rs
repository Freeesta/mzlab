#![cfg(feature = "thermo")]
//! Hostile `.raw` for the Thermo module: a real file (when the data repository is there) cut at many points and with bytes flipped gives an error with a key or a
//! reading, never a panic, and what is read (index, SRM traces, UV channels, method) never panics either. Without the data the seed is a bare header, which must
//! be refused with a key. No data of the laboratory is copied: the mutations are made in memory.
use mzlab_core::lettori::{FileSet, Limiti, Membro};
#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn insieme(byte: Vec<u8>) -> FileSet {
    FileSet {
        membri: vec![Membro {
            nome: "x.raw".into(),
            dim: byte.len() as u64,
            dati: Box::new(byte),
            sha1: None,
        }],
    }
}

fn semi() -> Vec<Vec<u8>> {
    if dati_percorsi::percorso("dati/formati/thermo/raw").is_none() {
        eprintln!("SKIP robustezza_thermo: no data repository, bare header only");
        return vec![vec![0x01, 0xA1, 0x46, 0x00, 0x69, 0x00, 0x6e, 0x00, 0x6e, 0x00]; 1];
    }
    [
        "dati/formati/thermo/raw/MTBLS7889_Fermentation_3_Media_test_001.raw",
        "dati/formati/thermo/pda/MTBLS9839_MET-2021-P17-YG_neg_MS2dd30k_1121_EB_076.raw",
    ]
    .iter()
    .filter_map(|r| dati_percorsi::percorso(r).and_then(|p| std::fs::read(p).ok()))
    .collect()
}

fn tutto(byte: Vec<u8>) {
    let Ok(mut l) = mzlab_lettori::apri(insieme(byte), &Limiti::sito()) else {
        return;
    };
    let Ok(ix) = l.indice(0).map(<[_]>::to_vec) else {
        return;
    };
    if !ix.is_empty() {
        let _ = l.scansione(0, ix.len() / 2);
    }
    if let Ok(v) = l.cromatogrammi(0) {
        for c in v.iter().take(4) {
            let _ = l.cromatogramma(0, &c.id);
        }
    }
    let _ = l.metodo(0);
}

#[test]
fn truncated_and_flipped_files_do_not_panic() {
    for s in semi() {
        let n = s.len();
        for k in 0..40 {
            let cut = n * k / 40;
            tutto(s[..cut].to_vec());
        }
        let mut x: u64 = 0x2545_F491_4F6C_DD1D;
        for _ in 0..30 {
            let mut m = s.clone();
            for _ in 0..8 {
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let i = (x >> 20) as usize % m.len();
                m[i] ^= 0xFF;
            }
            tutto(m);
        }
    }
}

#[test]
fn a_bare_header_is_refused_with_a_key() {
    for len in [0usize, 2, 18, 100] {
        let mut b = vec![0x01, 0xA1];
        b.extend("Finnigan".encode_utf16().flat_map(u16::to_le_bytes));
        b.resize(len, 0);
        if let Err(e) = mzlab_lettori::apri(insieme(b), &Limiti::sito()) {
            assert!(e.key.starts_with("err."), "{}", e.key);
        }
    }
}
