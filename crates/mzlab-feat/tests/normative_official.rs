//! `normative::official::import_official` on an invented ECHA-like CSV and an invented CAS sidecar (invented names, CAS-shaped numbers that fail the check
//! digit, InChIKey-shaped strings). The same inputs drive the CLI, the app route and the .wasm parity tests.
#![cfg(feature = "attributio")]

use mzlab_core::budget::sha256_hex;
use mzlab_feat::normative::official::import_official;
use mzlab_feat::suspects::Suspects;

const CSV: &str = include_str!("data/official/candidate-list.csv");
const SIDECAR: &str = include_str!("data/official/cas.tsv");

const LABEL: &str = "{\"file\":\"candidate-list.csv\",\"date\":\"2026-10-30\"}";

/// (summary JSON, .mzsus) of the envelope.
fn split(b: &[u8]) -> (&str, &[u8]) {
    let n = u32::from_le_bytes(b[..4].try_into().unwrap()) as usize;
    (std::str::from_utf8(&b[4..4 + n]).unwrap(), &b[4 + n..])
}

#[test]
fn an_invented_csv_becomes_a_private_package() {
    let out = import_official("svhc", CSV.as_bytes(), SIDECAR, LABEL).unwrap();
    let (summary, raw) = split(&out);
    let sha = sha256_hex(CSV.as_bytes());
    assert!(summary.contains(&format!(
        "imported by you on 2026-10-30 from candidate-list.csv, SHA-256 {sha}"
    )));
    // 4 entries: alpha and beta resolved, gamma not in the sidecar, the reaction mass is a mixture without CAS
    assert!(summary.contains("\"entries\":4") && summary.contains("\"screenable\":2"));
    assert!(summary.contains("\"unresolved\":1"), "{summary}");
    assert!(summary.contains("not in SusDat"));
    let s = Suspects::open(raw).expect("a valid .mzsus");
    assert_eq!(s.len(), 2);
    let l = s.listings_json().expect("the listings extension");
    assert!(
        l.contains("Testium alpha") && l.contains("Testium beta sodium salt"),
        "{l}"
    );
    assert!(!l.contains("Testium gamma"));
    // the manifest carries the list as the packed lists do (`normative/1`)
    let man = String::from_utf8_lossy(raw);
    assert!(man.contains("\"id\":\"SVHC\"") && man.contains("\"csv_sha256\""));
}

#[test]
fn the_same_inputs_give_the_same_bytes() {
    let a = import_official("svhc", CSV.as_bytes(), SIDECAR, LABEL).unwrap();
    let b = import_official("svhc", CSV.as_bytes(), SIDECAR, LABEL).unwrap();
    assert_eq!(a, b);
}

#[test]
fn a_file_without_the_columns_or_an_unknown_reader_is_refused() {
    assert!(import_official("svhc", b"a,b\n1,2\n", SIDECAR, LABEL).is_err());
    assert!(import_official("nope", CSV.as_bytes(), SIDECAR, LABEL).is_err());
    assert!(import_official("svhc", CSV.as_bytes(), "not a sidecar\n", LABEL).is_err());
}
