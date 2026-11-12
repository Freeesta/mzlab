//! `testo_errore`: catalogue keys to English text, the parameters filled in, the hint of the command run.
use super::{human, human_error, text_of, CATALOG};
use mzlab_core::error::Error;

#[test]
fn a_key_becomes_its_english_text_with_the_key_kept() {
    let e = Error::new("err.file.wiffNoScan").with("name", "X Y.wiff");
    assert_eq!(
        human(&e.to_string()),
        "The .wiff.scan file of X Y.wiff is missing: both are needed, with the same name. (err.file.wiffNoScan)"
    );
    let e = Error::new("err.file.zlib")
        .with("detail", "truncated")
        .with("at", 12);
    assert_eq!(
        human(&e.to_string()),
        "The file is damaged: a compressed peak array cannot be unpacked (truncated). Copy the file again or export it again from the instrument software. (err.file.zlib, at=12)"
    );
}

/// A .raw with no finished run (`opentfraw::Error::NoRun`, a run header with last scan 0): the page's text, which names the limit, what was done and the next
/// step without the reader's English detail; the detail and the size stay after the key for scripts.
#[test]
fn an_unfinished_raw_is_the_text_of_the_page() {
    // as `mzlab_core::raw::thermo_error` builds it (its parameters: `robustness_thermo_openings.rs`)
    let e = Error::new("err.file.rawUnfinished")
        .with("detail", "the run header declares no scan")
        .with("size", 5_670_000);
    let t = human(&e.to_string());
    assert_eq!(
        t,
        format!(
            "{} (err.file.rawUnfinished, detail=the run header declares no scan, size=5670000)",
            text_of("err.file.rawUnfinished").unwrap()
        )
    );
    assert!(
        t.contains("no scan index was written")
            && t.contains("Nothing was opened")
            && t.contains("instrument PC"),
        "{t}"
    );
}

/// A .raw whose counts point past its end: the calm text of the page, the reader's detail and the stage after the key.
#[test]
fn a_damaged_raw_is_the_text_of_the_page() {
    let e = Error::new("err.file.rawDamaged")
        .with("stage", "SeqRow")
        .with("detail", "refusing to allocate");
    let t = human(&e.to_string());
    assert_eq!(
        t,
        format!(
            "{} (err.file.rawDamaged, stage=SeqRow, detail=refusing to allocate)",
            text_of("err.file.rawDamaged").unwrap()
        )
    );
    assert!(
        t.contains("is damaged") && t.contains("instrument PC"),
        "{t}"
    );
}

/// A .cdf whose header describes more data than the file holds: the calm text of the page, the reader's detail after the key.
#[test]
fn a_damaged_cdf_is_the_text_of_the_page() {
    let e = Error::new("err.file.cdfDamaged").with("detail", "no scan_index");
    let t = human(&e.to_string());
    assert_eq!(
        t,
        format!(
            "{} (err.file.cdfDamaged, detail=no scan_index)",
            text_of("err.file.cdfDamaged").unwrap()
        )
    );
    assert!(
        t.contains("damaged or cut short") && t.contains("netCDF-3/AIA"),
        "{t}"
    );
}

/// A record of a .raw folder that no acquisition writes: the calm text of the page, the reader's detail after the key.
#[test]
fn a_damaged_folder_is_the_text_of_the_page() {
    let e = Error::new("err.file.folderDamaged").with("detail", "intensity word of the record");
    let t = human(&e.to_string());
    assert_eq!(
        t,
        format!(
            "{} (err.file.folderDamaged, detail=intensity word of the record)",
            text_of("err.file.folderDamaged").unwrap()
        )
    );
    assert!(
        t.contains("is damaged") && t.contains("whole folder"),
        "{t}"
    );
}

/// A part of a .D folder, of a .wiff pair or of a Bruker .d that a mutated copy breaks: the calm text of the page, the reader's detail only after the key.
#[test]
fn a_damaged_agilent_or_wiff_is_the_text_of_the_page() {
    for (key, detail, says) in [
        (
            "err.file.agilent",
            "MSScan.bin: 2460 bytes, records from 160, a record of 220 bytes does not fit",
            "whole folder again",
        ),
        (
            "err.file.wiff",
            "no scan type code (experiment 1)",
            "its .wiff.scan again",
        ),
        (
            "err.file.brukerDamaged",
            "sqlite: page size",
            "whole .d folder again",
        ),
    ] {
        let t = human(&Error::new(key).with("detail", detail).to_string());
        let text = text_of(key).unwrap();
        assert_eq!(t, format!("{text} ({key}, detail={detail})"));
        assert!(
            !text.contains(detail) && text.contains("nothing was loaded") && t.contains(says),
            "{t}"
        );
    }
}

#[test]
fn every_key_of_the_catalogue_is_found() {
    let keys: Vec<&str> = CATALOG
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
            text_of(k).is_some_and(|t| !t.is_empty() && !t.contains("\n")),
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
        assert_eq!(human(m), m);
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

#[test]
fn a_missing_input_names_its_path_in_english() {
    let p = std::env::temp_dir().join(format!("mzlab-absent-{} dir/x.mzML", std::process::id()));
    let o = crate::execute(&["channels".to_string(), p.to_string_lossy().into_owned()]);
    assert_eq!(o.code, 2, "{}", o.stderr);
    assert!(
        o.stderr
            .starts_with("error: A file or folder could not be read or written ("),
        "{}",
        o.stderr
    );
    assert!(
        o.stderr.contains(&format!("{}: ", p.display())) && o.stderr.contains("(err.flow.io)"),
        "{}",
        o.stderr
    );
}

/// The memory refusals read as in the desktop app (`tests/dati/avvisi_memoria.json`, checked against `I18N.err` by tests/test_avviso_memoria.py):
/// the limit is this computer's memory, the phase has its people's name, the key stays in brackets.
#[test]
fn a_memory_refusal_reads_as_in_the_desktop_app() {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/dati/avvisi_memoria.json")).unwrap();
    for case in f["casi"].as_array().unwrap() {
        let mut e = Error::memory_budget();
        for p in case["params"].as_array().unwrap() {
            let k: &'static str = Box::leak(p[0].as_str().unwrap().to_string().into_boxed_str());
            e = e.with(k, p[1].as_str().unwrap());
        }
        // after the text, the key and the parameters the text does not use, as for every key (the phase by its id, for scripts)
        let rendered = human(&e.to_string());
        let (text, tail) = rendered.split_once(" (error.memoryBudget").unwrap();
        assert_eq!(text, case["testo"].as_str().unwrap());
        assert!(
            !tail.contains("stima_mb") && !tail.contains("index peaks"),
            "{tail}"
        );
    }
}

/// A parameter whose value holds spaces and `name=` words stays one value when the error is rendered from itself; other parameters keep their place.
#[test]
fn a_value_with_name_words_stays_whole() {
    let id = "sim:SIM SIC Q1=237.1 sample=1 dwell=50 name=ionA";
    let e = Error::new("err.canali.unknown")
        .with("id", id)
        .with("at", 3);
    assert_eq!(
        human_error(&e),
        format!("Channel {id} does not exist in this file. (err.canali.unknown, at=3)")
    );
}
