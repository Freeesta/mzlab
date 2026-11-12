//! The acquisition program a timsTOF folder names (`AcquisitionSoftware`) goes in the mzML as the generic term `software` with the name in a
//! userParam, so the strict check of a conversion accepts the file (a vendor name on `MS:1000799` was refused as `err.file.mzmlInvalid`).
#![cfg(feature = "bruker")]

use mzlab_core::readers::{FileSet, Limits, Member};

mod common_sqlite;

#[test]
fn the_acquisition_program_is_a_valid_software_term() {
    let (tdf, bin) = common_sqlite::pasef::folder_with(
        2,
        500,
        200,
        &[
            ("AcquisitionSoftware", "Bruker otofControl"),
            ("AcquisitionSoftwareVersion", "6.0"),
        ],
    );
    let m = |n: &str, b: Vec<u8>| Member {
        nome: format!("x.d/{n}"),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    };
    let fs = FileSet {
        membri: vec![m("analysis.tdf", tdf), m("analysis.tdf_bin", bin)],
    };
    let mut l = mzlab_lettori::apri(fs, &Limits::desktop()).unwrap();
    let mut out = Vec::new();
    mzlab_core::readers_mzml::scrivi(&mut *l, 0, &mut out, &mut |_, _| {}).unwrap();
    mzlab_core::mzml_validate::check_written(&out[..]).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains(r#"<cvParam cvRef="MS" accession="MS:1000531" name="software" value=""/><userParam name="software" value="Bruker otofControl"/>"#),
        "{}",
        &text[..text.find("</softwareList>").unwrap_or(0)]
    );
}
