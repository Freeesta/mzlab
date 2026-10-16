//! A `.mzfeatf` / `.mzfeat` written by a newer program (container version or `feat_version` above the known one, or a newer magic) is refused with the typed error
//! `error.newerFormat`; one of the known version reads as before. Synthetic data only.

use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{read_file, read_table, write_file, write_table, Origin};
use mzlab_feat::table::{Aligned, FileFeatures};

/// The bytes with the first `from` of the meta replaced by `to` (same length, so the container stays well formed).
fn patched(mut b: Vec<u8>, from: &str, to: &str) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let at = b
        .windows(from.len())
        .position(|w| w == from.as_bytes())
        .unwrap_or_else(|| panic!("{from} not in the meta"));
    b[at..at + to.len()].copy_from_slice(to.as_bytes());
    b
}

fn err(r: Result<impl Sized, mzlab_core::error::Error>) -> (String, Vec<(String, String)>) {
    let e = r.err().expect("refused");
    (
        e.key.to_string(),
        e.params
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    )
}

fn file() -> Vec<u8> {
    write_file(
        &FileFeatures::default(),
        &Recipe::default(),
        &Origin::default(),
    )
}

fn table() -> Vec<u8> {
    write_table(&Aligned::default(), &Recipe::default(), &Origin::default())
}

#[test]
fn the_known_version_reads() {
    assert!(read_file(&file()).is_ok());
    assert!(read_table(&table()).is_ok());
}

#[test]
fn a_newer_feat_version_is_refused_and_named() {
    let next = format!("\"feat_version\":{}", mzlab_feat::FEAT_VERSION + 1);
    let cur = format!("\"feat_version\":{}", mzlab_feat::FEAT_VERSION);
    let (k, p) = err(read_file(&patched(file(), &cur, &next)));
    assert_eq!(k, "error.newerFormat");
    assert!(
        p.contains(&("kind".into(), "mzfeatf".into()))
            && p.contains(&("version".into(), (mzlab_feat::FEAT_VERSION + 1).to_string())),
        "{p:?}"
    );
    let (k, p) = err(read_table(&patched(table(), &cur, &next)));
    assert_eq!(k, "error.newerFormat");
    assert!(p.contains(&("kind".into(), "mzfeat".into())), "{p:?}");
}

#[test]
fn a_newer_container_version_is_refused() {
    let cur = format!("\"version\":{}", mzlab_feat::store::TABLE_FORMAT);
    let next = format!("\"version\":{}", mzlab_feat::store::TABLE_FORMAT + 1);
    assert_eq!(
        err(read_table(&patched(table(), &cur, &next))).0,
        "error.newerFormat"
    );
    assert_eq!(
        err(read_file(&patched(
            file(),
            "\"version\":1",
            "\"version\":2"
        )))
        .0,
        "error.newerFormat"
    );
}

#[test]
fn a_newer_magic_is_refused_and_a_damaged_file_stays_damaged() {
    assert_eq!(
        err(read_table(&patched(table(), "MZFEAT01", "MZFEAT02"))).0,
        "error.newerFormat"
    );
    assert_eq!(
        err(read_table(&patched(table(), "MZFEAT01", "MZFEATxx"))).0,
        "error.badMzfeat"
    );
    assert_eq!(err(read_file(&file()[..20])).0, "error.badMzfeat");
}
