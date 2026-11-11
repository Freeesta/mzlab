use super::*;

/// A reader of nothing, enough to open and to count.
struct Empty {
    campioni: Vec<Sample>,
    indice: Vec<ScanEntry>,
}

impl Reader for Empty {
    fn format(&self) -> Format {
        Format::Altro("prova")
    }
    fn samples(&self) -> &[Sample] {
        &self.campioni
    }
    fn index(&mut self, _: usize) -> Result<&[ScanEntry]> {
        Ok(&self.indice)
    }
    fn scan(&mut self, _: usize, k: usize) -> Result<Spectrum> {
        Err(Error::new("error.badIndex").with("k", k))
    }
    fn scan_header(&mut self, _: usize, _: usize) -> Result<Vec<Field>> {
        Ok(vec![])
    }
    fn chromatograms(&mut self, _: usize) -> Result<Vec<ChromatogramEntry>> {
        Ok(vec![])
    }
    fn chromatogram(&mut self, _: usize, _: &str) -> Result<Chromatogram> {
        Err(Error::new("error.badIndex"))
    }
    fn method(&mut self, _: usize) -> Result<String> {
        Ok(String::new())
    }
}

fn open_empty(_: FileSet, _: &Limits) -> Result<Box<dyn Reader>> {
    Ok(Box::new(Empty {
        campioni: vec![Sample {
            id: 1,
            nome: "a".into(),
            scansioni: 0,
            size: None,
        }],
        indice: vec![],
    }))
}

fn file_set(name: &str, byte: &[u8]) -> FileSet {
    FileSet {
        membri: vec![Member {
            nome: name.into(),
            dim: byte.len() as u64,
            dati: Box::new(byte.to_vec()),
            sha1: None,
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
    if fs.find(".dat").is_some() {
        Fiducia::Forse
    } else {
        Fiducia::No
    }
}

const REGISTRY: &[ReaderEntry] = &[
    ReaderEntry {
        id: "forse",
        contratto: CONTRATTO,
        estensioni: &[".dat", ".x"],
        riconosci: riconosci_forse,
        apri: open_empty,
    },
    ReaderEntry {
        id: "a",
        contratto: CONTRATTO,
        estensioni: &[".x", ".a"],
        riconosci: riconosci_a,
        apri: open_empty,
    },
    ReaderEntry {
        id: "vecchio",
        contratto: 0,
        estensioni: &[".old"],
        riconosci: |_| Fiducia::Si,
        apri: open_empty,
    },
];

#[test]
fn the_surest_reader_wins_and_a_reader_of_another_contract_is_not_asked() {
    let mut fs = file_set("dati.dat", b"AAxx");
    assert_eq!(riconosci_in(REGISTRY, &mut fs).map(|v| v.id), Some("a")); // the bytes beat the extension
    let mut fs = file_set("dati.dat", b"BBxx");
    assert_eq!(riconosci_in(REGISTRY, &mut fs).map(|v| v.id), Some("forse"));
    let mut fs = file_set("dati.zip", b"BBxx");
    assert!(riconosci_in(REGISTRY, &mut fs).is_none());
}

#[test]
fn the_extensions_for_the_pickers_come_from_the_registry_without_repeats() {
    assert_eq!(estensioni_in(REGISTRY), [".dat", ".x", ".a", ".old"]);
    // the real registry: whatever is registered, in lower case with the dot and without repeats
    let real = estensioni();
    assert!(real
        .iter()
        .all(|e| e.starts_with('.') && *e == e.to_lowercase()));
    assert!(real.iter().enumerate().all(|(i, e)| !real[..i].contains(e)));
}

#[test]
fn an_unknown_set_is_an_error_with_a_key_and_the_head_is_read_at_most_to_the_end() {
    let e = apri(file_set("x.bin", b"zz"), &Limits::default())
        .err()
        .unwrap();
    assert_eq!(e.key, "err.file.notAccepted");
    let mut fs = file_set("x.bin", b"abc");
    assert_eq!(fs.testa(0, 100), b"abc");
    assert!(fs.testa(5, 10).is_empty());
}

#[test]
fn the_limits_refuse_what_a_file_asks_beyond_them() {
    let l = Limits {
        profondita: 4,
        entries: 1000,
        byte: 1 << 20,
    };
    assert_eq!(l.ok_entries(1000).unwrap(), 1000);
    assert_eq!(l.ok_entries(1001).unwrap_err().key, "error.memoryBudget");
    assert_eq!(
        l.ok_entries(u64::MAX).unwrap_err().key,
        "error.memoryBudget"
    );
    assert!(l.profondita_ok(4).is_ok());
    assert_eq!(l.profondita_ok(5).unwrap_err().key, "err.dam.unreadable");
    assert!(Limits::desktop().byte > Limits::sito().byte);
}

#[test]
fn a_reader_serves_the_trait() {
    let mut l = open_empty(FileSet::default(), &Limits::default()).unwrap();
    assert_eq!(l.format().id(), "prova");
    assert_eq!(l.samples().len(), 1);
    assert!(l.index(0).unwrap().is_empty());
    assert!(l.scan(0, 3).is_err());
}

fn entry(rt: f64, level: u8, tic: f64) -> ScanEntry {
    ScanEntry {
        id: String::new(),
        rt,
        livello: level,
        polarita: 1,
        tic,
        base_peak: None,
        filtro: String::new(),
        precursore: None,
        position: None,
    }
}

#[test]
fn the_early_tic_keeps_the_ms1_scans_or_every_scan_when_none_is_ms1() {
    let v = [entry(0.1, 1, 10.0), entry(0.2, 2, 3.0), entry(0.3, 1, 12.0)];
    assert_eq!(tic_ms1(&v), (vec![0.1, 0.3], vec![10.0, 12.0]));
    let v = [entry(0.1, 2, 4.0), entry(0.2, 2, 5.0)];
    assert_eq!(tic_ms1(&v), (vec![0.1, 0.2], vec![4.0, 5.0]));
    assert_eq!(tic_ms1(&[]), (vec![], vec![]));
}

/// The default account of a read: the rows of the index plus the scans `missing` counts, the cause only when something is lost.
#[test]
fn losses_count_the_index_and_the_missing_scans() {
    use super::outcome::ReadOutcomes;
    assert_eq!(Losses::of(3, 3, TRUNCATED), Losses::complete(3));
    assert!(!Losses::complete(3).partial());
    let l = Losses::of(5, 2, TRUNCATED);
    assert!(l.partial());
    assert_eq!((l.lost(), l.cause), (3, TRUNCATED));
    // a reader that reads more than it declares is not partial
    assert_eq!(Losses::of(1, 2, TRUNCATED), Losses::complete(2));
    let mut r = Empty {
        campioni: vec![],
        indice: vec![],
    };
    let o = r.index_outcome(0).unwrap();
    assert_eq!((o.data.len(), o.losses), (0, Losses::complete(0)));
    assert_eq!(
        r.chromatograms_outcome(0).unwrap().losses,
        Losses::complete(0)
    );
}
