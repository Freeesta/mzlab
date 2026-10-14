//! Hostile input for the registry of the module: bytes no reader knows are an error with a key (never a panic), and so are an empty set and a truncated header.
use mzlab_core::lettori::{FileSet, Limiti, Membro};

fn insieme(nome: &str, byte: Vec<u8>) -> FileSet {
    FileSet { membri: vec![Membro { nome: nome.into(), dim: byte.len() as u64, dati: Box::new(byte) }] }
}

fn chiave(fs: FileSet) -> &'static str {
    match mzlab_lettori::apri(fs, &Limiti::sito()) {
        Ok(_) => "ok",
        Err(e) => e.key,
    }
}

#[test]
fn byte_sconosciuti_sono_un_errore_con_chiave() {
    assert_eq!(chiave(insieme("x.dat", vec![0xAB; 4096])), "err.file.notAccepted");
    assert_eq!(chiave(insieme("x.D/AcqData/MSScan.bin", vec![0; 8])), "err.file.notAccepted");
    assert_eq!(chiave(insieme("x.raw", vec![0x01, 0xA1, 0x46])), "err.file.notAccepted");
    assert_eq!(chiave(insieme("x.wiff", Vec::new())), "err.file.notAccepted");
}

#[test]
fn insieme_vuoto() {
    assert_eq!(chiave(FileSet::default()), "err.file.notAccepted");
}

#[cfg(feature = "tutte")]
#[test]
fn le_estensioni_del_modulo_contengono_quelle_del_motore() {
    let m = mzlab_lettori::estensioni();
    for e in mzlab_core::lettori::estensioni() {
        assert!(m.contains(&e), "{e}");
    }
}

/// The row of each family in `lettori-famiglie.js` holds the extensions of every reader of the registry of this build (the page sends a file to the module by that table).
#[test]
fn le_estensioni_dei_lettori_stanno_nella_tabella_della_pagina() {
    let js = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../mzlab/web/lettori-famiglie.js")).unwrap();
    let riga = |fam: &str| js.lines().find(|l| l.trim_start().starts_with(&format!("{fam}:"))).unwrap_or_else(|| panic!("no row for {fam}")).to_string();
    let per_famiglia: [(&str, &str); 3] = [("agilent", ".d"), ("sciex", ".wiff"), ("thermo", ".raw")];
    for (fam, ext) in per_famiglia {
        let attiva = mzlab_lettori::famiglia() == "tutte" || mzlab_lettori::famiglia() == fam;
        if attiva {
            assert!(riga(fam).contains(&format!("\"{ext}\"")), "{fam}: {ext}");
        }
    }
    for v in mzlab_lettori::registro() {
        let in_tabella = ["agilent", "sciex", "thermo", "aperti", "waters", "shimadzu", "bruker"].iter().any(|f| v.estensioni.iter().all(|e| riga(f).contains(&format!("\"{e}\""))));
        assert!(in_tabella, "{}", v.id);
    }
}
