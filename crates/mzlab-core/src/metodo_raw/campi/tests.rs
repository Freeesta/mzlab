use super::*;

#[test]
fn the_clock_reads_the_epoch_as_a_date_without_zone() {
    assert_eq!(clock(0.0).as_deref(), Some("1970-01-01T00:00:00"));
    assert_eq!(
        clock(1_669_447_937.9).as_deref(),
        Some("2022-11-26T07:32:17")
    );
    assert_eq!(clock(951_782_400.0).as_deref(), Some("2000-02-29T00:00:00")); // leap day
    assert_eq!(clock(f64::NAN), None);
}

#[test]
fn identity_is_found_and_scrubbed_in_any_case() {
    let id = Ident {
        computer: "LAB-PC-7".into(),
        paths: vec!["C:\\Data\\Rossi\\run1".into()],
        ..Default::default()
    };
    assert!(id.names("Instrument on lab-pc-7") && id.names("file c:\\data\\rossi\\run1\\a.raw"));
    assert!(!id.names("Orbitrap Fusion"));
    assert_eq!(
        id.scrub("on LAB-PC-7, in C:\\Data\\Rossi\\run1\\a.raw"),
        "on , in run1\\a.raw"
    );
}
