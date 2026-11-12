//! The mzML header of an ANDI file: the run named after the file, the source with its digest, the instrument and the components of the header.
#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn the_header_names_the_run_after_the_file() {
    let a = AndiHeader {
        model: Some("Model Q".into()),
        series: Some("S-1".into()),
        ionizzazione: Some("Electrospray".into()),
        ..AndiHeader::default()
    };
    let h = a.header("run_1.cdf", Some("ab".into()), "{}".into());
    assert_eq!(h.run, "run_1");
    assert_eq!(h.sources, ["run_1.cdf"]);
    assert_eq!(h.source_sha1, ["ab"]);
    assert_eq!(h.instrument, Some(("MS:1000031".into(), "Model Q".into())));
    assert_eq!(h.serial.as_deref(), Some("S-1"));
    assert_eq!(h.software.0, "mzLab");
    assert_eq!(h.components.len(), 1);
    assert_eq!(h.components[0].2, "ionization type: Electrospray");
    assert_eq!(h.run_params.last().unwrap().1, "{}");
    // a name without an extension is the run as it is
    assert_eq!(
        AndiHeader::default().header("run", None, String::new()).run,
        "run"
    );
}
