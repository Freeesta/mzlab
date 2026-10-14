use super::*;

/// A reader of nothing, enough to open and to count.
struct Vuoto {
    campioni: Vec<Campione>,
    indice: Vec<VoceScansione>,
}

impl Lettore for Vuoto {
    fn formato(&self) -> Formato {
        Formato::Altro("prova")
    }
    fn campioni(&self) -> &[Campione] {
        &self.campioni
    }
    fn indice(&mut self, _: usize) -> Result<&[VoceScansione]> {
        Ok(&self.indice)
    }
    fn scansione(&mut self, _: usize, k: usize) -> Result<Spectrum> {
        Err(Error::new("error.badIndex").with("k", k))
    }
    fn intestazione(&mut self, _: usize, _: usize) -> Result<Vec<Campo>> {
        Ok(vec![])
    }
    fn cromatogrammi(&mut self, _: usize) -> Result<Vec<VoceCromatogramma>> {
        Ok(vec![])
    }
    fn cromatogramma(&mut self, _: usize, _: &str) -> Result<Chromatogram> {
        Err(Error::new("error.badIndex"))
    }
    fn metodo(&mut self, _: usize) -> Result<String> {
        Ok(String::new())
    }
}

fn apri_vuoto(_: FileSet, _: &Limiti) -> Result<Box<dyn Lettore>> {
    Ok(Box::new(Vuoto {
        campioni: vec![Campione {
            id: 1,
            nome: "a".into(),
            scansioni: 0,
        }],
        indice: vec![],
    }))
}

fn insieme(nome: &str, byte: &[u8]) -> FileSet {
    FileSet {
        membri: vec![Membro {
            nome: nome.into(),
            dim: byte.len() as u64,
            dati: Box::new(byte.to_vec()),
        }],
    }
}

/// Recognised by its first bytes, whatever the name says.
fn riconosci_a(fs: &mut FileSet) -> Fiducia {
    if fs.testa(0, 2) == b"AA" {
        Fiducia::Si
    } else {
        Fiducia::No
    }
}

fn riconosci_forse(fs: &mut FileSet) -> Fiducia {
    if fs.trova(".dat").is_some() {
        Fiducia::Forse
    } else {
        Fiducia::No
    }
}

const REGISTRO: &[VoceLettore] = &[
    VoceLettore {
        id: "forse",
        contratto: CONTRATTO,
        estensioni: &[".dat", ".x"],
        riconosci: riconosci_forse,
        apri: apri_vuoto,
    },
    VoceLettore {
        id: "a",
        contratto: CONTRATTO,
        estensioni: &[".x", ".a"],
        riconosci: riconosci_a,
        apri: apri_vuoto,
    },
    VoceLettore {
        id: "vecchio",
        contratto: 0,
        estensioni: &[".old"],
        riconosci: |_| Fiducia::Si,
        apri: apri_vuoto,
    },
];

#[test]
fn the_surest_reader_wins_and_a_reader_of_another_contract_is_not_asked() {
    let mut fs = insieme("dati.dat", b"AAxx");
    assert_eq!(riconosci_in(REGISTRO, &mut fs).map(|v| v.id), Some("a")); // the bytes beat the extension
    let mut fs = insieme("dati.dat", b"BBxx");
    assert_eq!(riconosci_in(REGISTRO, &mut fs).map(|v| v.id), Some("forse"));
    let mut fs = insieme("dati.zip", b"BBxx");
    assert!(riconosci_in(REGISTRO, &mut fs).is_none());
}

#[test]
fn the_extensions_for_the_pickers_come_from_the_registry_without_repeats() {
    assert_eq!(estensioni_in(REGISTRO), [".dat", ".x", ".a", ".old"]);
    // the real registry: whatever is registered, in lower case with the dot and without repeats
    let real = estensioni();
    assert!(real
        .iter()
        .all(|e| e.starts_with('.') && *e == e.to_lowercase()));
    assert!(real.iter().enumerate().all(|(i, e)| !real[..i].contains(e)));
}

#[test]
fn an_unknown_set_is_an_error_with_a_key_and_the_head_is_read_at_most_to_the_end() {
    let e = apri(insieme("x.bin", b"zz"), &Limiti::default())
        .err()
        .unwrap();
    assert_eq!(e.key, "err.file.notAccepted");
    let mut fs = insieme("x.bin", b"abc");
    assert_eq!(fs.testa(0, 100), b"abc");
    assert!(fs.testa(5, 10).is_empty());
}

#[test]
fn the_limits_refuse_what_a_file_asks_beyond_them() {
    let l = Limiti {
        profondita: 4,
        voci: 1000,
        byte: 1 << 20,
    };
    assert_eq!(l.voci_ok(1000).unwrap(), 1000);
    assert_eq!(l.voci_ok(1001).unwrap_err().key, "error.memoryBudget");
    assert_eq!(l.voci_ok(u64::MAX).unwrap_err().key, "error.memoryBudget");
    assert!(l.profondita_ok(4).is_ok());
    assert_eq!(l.profondita_ok(5).unwrap_err().key, "err.dam.unreadable");
    assert!(Limiti::desktop().byte > Limiti::sito().byte);
}

#[test]
fn a_reader_serves_the_trait() {
    let mut l = apri_vuoto(FileSet::default(), &Limiti::default()).unwrap();
    assert_eq!(l.formato().id(), "prova");
    assert_eq!(l.campioni().len(), 1);
    assert!(l.indice(0).unwrap().is_empty());
    assert!(l.scansione(0, 3).is_err());
}
