//! `testo_errore`: catalogue keys to English text, the parameters filled in, the hint of the command run.
use super::{testo_di, umano, CATALOGO};
use mzlab_core::error::Error;

#[test]
fn a_key_becomes_its_english_text_with_the_key_kept() {
    let e = Error::new("err.file.wiffNoScan").with("name", "X Y.wiff");
    assert_eq!(
        umano(&e.to_string()),
        "The .wiff.scan file of X Y.wiff is missing: both are needed, with the same name. (err.file.wiffNoScan)"
    );
    let e = Error::new("err.file.zlib")
        .with("detail", "truncated")
        .with("at", 12);
    assert_eq!(
        umano(&e.to_string()),
        "The file is damaged: a compressed peak array cannot be unpacked (truncated). Copy the file again or export it again from the instrument software. (err.file.zlib, at=12)"
    );
}

#[test]
fn every_key_of_the_catalogue_is_found() {
    let keys: Vec<&str> = CATALOGO
        .lines()
        .filter_map(|l| {
            l.trim()
                .strip_prefix('"')?
                .split_once("\": \"")
                .map(|(k, _)| k)
        })
        .filter(|k| k.starts_with("err.") || k.starts_with("error."))
        .collect();
    assert!(keys.len() > 300, "{}", keys.len());
    for k in keys {
        assert!(
            testo_di(k).is_some_and(|t| !t.is_empty() && !t.contains("\n")),
            "{k}"
        );
    }
}

#[test]
fn other_messages_pass_unchanged() {
    for m in [
        "unknown option --x",
        "err.nonexistent.key a=1",
        "no input file",
        "error: x",
    ] {
        assert_eq!(umano(m), m);
    }
}

/// An engine error reaches the terminal as its English text with the key in brackets, and the hint names the command that was run.
#[test]
fn an_engine_error_is_english_text_and_the_hint_is_this_command() {
    let dir = std::env::temp_dir().join(format!("mzlab-canali-err-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("sola.wiff");
    std::fs::write(&f, b"not a wiff at all").unwrap();
    let o = crate::execute(&["channels".to_string(), f.to_string_lossy().into_owned()]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(o.code, 2, "{}", o.stderr);
    assert!(o.stderr.starts_with("error: "), "{}", o.stderr);
    assert!(!o.stderr.contains("[(\""), "{}", o.stderr);
    assert!(o.stderr.contains(" (err.file."), "{}", o.stderr);
    assert!(
        o.stderr.ends_with("(mzlab-cli channels --help)\n"),
        "{}",
        o.stderr
    );
}
