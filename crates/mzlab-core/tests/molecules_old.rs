//! An archive written before the keys were English is refused with its own message (no migration).

use mzlab_core::molecules::{from_json, route};

/// An archive written before the keys were English is refused with its own message, never read as empty or as nameless records.
#[test]
fn an_old_format_archive_is_refused_with_its_own_message() {
    let old_doc =
        r#"{"molecole":[{"nome":"Esempina","formula":"C10H15NO"}],"schema":"molecules/1"}"#;
    let e = from_json(old_doc.as_bytes()).unwrap_err();
    assert_eq!(
        (e.len(), e[0].key, e[0].line),
        (1, "error.molecules.oldFormat", 0)
    );
    // the document of the page: the new wrapper around old records
    let old_rec = r#"{"schema":"molecules/1","molecules":[{"nome":"Esempina","formula":"C10H15NO"},{"name":"Nuova","formula":"CH4"}]}"#;
    let e = from_json(old_rec.as_bytes()).unwrap_err();
    assert_eq!(
        (e.len(), e[0].key, e[0].line),
        (1, "error.molecules.oldFormat", 1)
    );
    // a record with neither key is just a record without a name
    let e = from_json(br#"{"schema":"molecules/1","molecules":[{"formula":"CH4"}]}"#).unwrap_err();
    assert_eq!(e[0].key, "error.molecules.noName");
    // a document that is neither is a bad document
    let e = from_json(br#"{"schema":"molecules/1"}"#).unwrap_err();
    assert_eq!(e[0].key, "error.molecules.badJson");
    // through the route the answer carries the key for the page
    assert!(route("load", &format!("{{\"user\":{old_rec}}}"))
        .unwrap()
        .contains("error.molecules.oldFormat"));
}
