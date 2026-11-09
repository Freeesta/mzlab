//! Hostile input for the registry of the module: bytes no reader knows are an error with a key (never a panic), and so are an empty set and a truncated header.
use mzlab_core::readers::{FileSet, Limits, Member};

fn file_set(name: &str, byte: Vec<u8>) -> FileSet {
    FileSet {
        membri: vec![Member {
            nome: name.into(),
            dim: byte.len() as u64,
            dati: Box::new(byte),
            sha1: None,
        }],
    }
}

fn key(fs: FileSet) -> &'static str {
    match mzlab_lettori::apri(fs, &Limits::sito()) {
        Ok(_) => "ok",
        Err(e) => e.key,
    }
}

#[test]
fn unknown_bytes_are_a_keyed_error() {
    assert_eq!(
        key(file_set("x.dat", vec![0xAB; 4096])),
        "err.file.notAccepted"
    );
    assert_eq!(
        key(file_set("x.D/AcqData/MSScan.bin", vec![0; 8])),
        "err.file.notAccepted"
    );
    assert_eq!(
        key(file_set("x.raw", vec![0x01, 0xA1, 0x46])),
        "err.file.notThisFormat"
    );
    assert_eq!(key(file_set("x.wiff", Vec::new())), "err.file.empty");
    // a name the registry does not know stays «not accepted»; a plain .mzML the engine reads is not called damaged by this crate
    assert_eq!(key(file_set("x.cdf", Vec::new())), "err.file.empty");
    assert_eq!(
        key(file_set("x.cdf", b"CDF".to_vec())),
        "err.file.notThisFormat"
    );
    assert_eq!(
        key(file_set("x.mzML", b"<x/>".to_vec())),
        "err.file.notAccepted"
    );
}

#[test]
fn empty_set() {
    assert_eq!(key(FileSet::default()), "err.file.notAccepted");
}

#[cfg(feature = "tutte")]
#[test]
fn module_extensions_contain_the_engine_ones() {
    let m = mzlab_lettori::estensioni();
    for e in mzlab_core::readers::estensioni() {
        assert!(m.contains(&e), "{e}");
    }
}

/// The row of each family in `lettori-famiglie.js` holds the extensions of every reader of the registry of this build (the page sends a file to the module by that table).
#[test]
fn reader_extensions_are_in_the_page_table() {
    let js = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../mzlab/web/lettori-famiglie.js"
    ))
    .unwrap();
    let row = |fam: &str| {
        js.lines()
            .find(|l| l.trim_start().starts_with(&format!("{fam}:")))
            .unwrap_or_else(|| panic!("no row for {fam}"))
            .to_string()
    };
    let per_family: [(&str, &str); 3] = [("agilent", ".d"), ("sciex", ".wiff"), ("thermo", ".raw")];
    for (fam, ext) in per_family {
        let active = mzlab_lettori::famiglia() == "tutte" || mzlab_lettori::famiglia() == fam;
        if active {
            assert!(row(fam).contains(&format!("\"{ext}\"")), "{fam}: {ext}");
        }
    }
    // `mzxml-gz` is a voice of the app and the CLI only: the page inflates a .mzXML.gz itself (gz-flusso.js) and gives the module the .mzXML
    for v in mzlab_lettori::registry()
        .into_iter()
        .filter(|v| v.id != "mzxml-gz")
    {
        let in_table = [
            "agilent", "sciex", "thermo", "aperti", "andi", "waters", "shimadzu", "bruker",
        ]
        .iter()
        .any(|f| {
            v.estensioni
                .iter()
                .all(|e| row(f).contains(&format!("\"{e}\"")))
        });
        assert!(in_table, "{}", v.id);
    }
}
