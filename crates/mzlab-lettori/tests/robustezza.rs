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

#[test]
fn le_estensioni_del_modulo_contengono_quelle_del_motore() {
    let m = mzlab_lettori::estensioni();
    for e in mzlab_core::lettori::estensioni() {
        assert!(m.contains(&e), "{e}");
    }
}
