//! The listing of a dataset in the app (`natives::samples`) is the listing of the site byte for byte: a refused sample with its reason, apart from the
//! empty injection of the same pair (H6-15). Synthetic pair `rifiutato` of tools/wiff_riferimento.py; the same text is asserted by
//! mzlab-core/tests/wiff_refused_listing.rs (engine) and tests/test_parziali_parita.py (site).

use std::path::Path;
use std::process::Command;

const LISTING: &str = r#"{"format":"SciexWiff","samples":[{"id":1,"name":"rifiutato_s1","scans":1},{"id":2,"name":"rifiutato_s2","scans":0,"why":"empty"},{"id":3,"name":"rifiutato_s3","scans":0,"why":"refused","refused":{"error_key":"err.file.wiffScanType","params":{"code":"8","experiment":"1","type":"TOF MS"}}}]}"#;

#[test]
fn the_app_lists_a_refused_sample_as_the_site_does() {
    let out = std::env::temp_dir().join(format!("mzlab_desk_rifiutato_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
    if !Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .is_ok_and(|o| o.status.success())
    {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP natives_tests: python3 not available");
        return;
    }
    let members: Vec<_> = ["rifiutato.wiff", "rifiutato.wiff.scan"]
        .iter()
        .map(|n| (n.to_string(), out.join(n)))
        .collect();
    let got = crate::natives::samples(&members).unwrap();
    let _ = std::fs::remove_dir_all(&out);
    assert_eq!(got, LISTING);
}
