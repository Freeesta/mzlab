//! A file that is no mzML is refused with `err.file.notMzml` and its name (as `mzlab/reader/mzml.py` does), not loaded as a file with no scans.

use mzlab_core::upload::prepare;

#[test]
fn text_without_mzml_is_refused_with_its_name() {
    let mut bytes = b"\x00\x01\x02 this is not xml ".repeat(400);
    let size = bytes.len() as u64;
    let e = prepare(&mut bytes, size, "nonxml.mzML").err().expect("refused");
    assert_eq!(e.key, "err.file.notMzml");
    assert_eq!(e.params, vec![("name", "nonxml.mzML".to_string())]);
}

#[test]
fn empty_input_is_refused_too() {
    let mut bytes: Vec<u8> = Vec::new();
    let e = prepare(&mut bytes, 0, "vuoto.mzML").err().expect("refused");
    assert_eq!(e.key, "err.file.notMzml");
}
