//! The family the test build serves (`tutte`: every reader, as in the desktop app) and the probe of a dataset that holds no Agilent run.
#![cfg(feature = "tutte")]

use mzlab_core::readers::FileSet;

#[test]
fn the_test_build_serves_every_family() {
    assert_eq!(mzlab_lettori::famiglia(), "tutte");
}

#[test]
fn a_dataset_with_no_agilent_run_has_no_reason() {
    let mut fs = FileSet { membri: Vec::new() };
    assert!(mzlab_lettori::reasons_together(&mut fs).is_empty());
}
