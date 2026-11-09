//! A sample without a `MethodN` of its own borrows the method before it only when that method acquires what its blocks hold (`tools/wiff_misto.py`):
//! the MRM sample s3 keeps its transitions next to the product-ion Method2, the scan sample s6 reads its scans as the MS2 of Method5.
//! Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::Path;
use std::process::Command;

use mzlab_core::mzml::ChromKind;
use mzlab_core::wiff::{container, Wiff};

#[test]
fn a_sample_borrows_only_a_method_that_fits_its_blocks() {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_prestito_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_misto.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_prestito: python3 not available");
        return;
    }
    let ole = container(std::fs::read(out.join("misto.wiff")).unwrap(), "misto").unwrap();
    let mut scan = std::fs::read(out.join("misto.wiff.scan")).unwrap();
    let len = Some(scan.len() as u64);
    let open = |n| Wiff::open(&ole, "misto", len, n).unwrap();

    let s3 = open(3).run(&mut scan).unwrap();
    assert!(
        s3.spectra.is_empty(),
        "the MRM blocks are not read as product-ion scans"
    );
    let srm = s3
        .chromatograms
        .iter()
        .filter(|c| c.kind == ChromKind::Srm)
        .count();
    assert_eq!(srm, 3, "one trace per transition of the blocks");

    let s6 = open(6).run(&mut scan).unwrap();
    assert_eq!(s6.spectra.len(), 12);
    assert!(
        s6.spectra
            .iter()
            .all(|s| s.level == 2 && s.precursor == Some(305.0)),
        "the method of s5 stands for s6"
    );
    std::fs::remove_dir_all(&out).ok();
}
