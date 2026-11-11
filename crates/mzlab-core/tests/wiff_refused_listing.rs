//! A pair with a sample the reader refuses lists it with its reason (`readers::samples_json`, the listing of the site and of the app), never as an
//! injection that acquired nothing; the empty injection of the same pair stays `empty` (H6-15). Synthetic pair `rifiutato` of tools/wiff_riferimento.py.

use std::path::Path;
use std::process::Command;

use mzlab_core::readers::{self, FileSet, Limits, Member};

fn pair() -> Option<(Vec<u8>, Vec<u8>)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_rifiutato_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_refused_listing: python3 not available");
        return None;
    }
    let got = (
        std::fs::read(out.join("rifiutato.wiff")).unwrap(),
        std::fs::read(out.join("rifiutato.wiff.scan")).unwrap(),
    );
    let _ = std::fs::remove_dir_all(&out);
    Some(got)
}

/// The listing, byte for byte: the app (`mzlab-desktop` natives_tests.rs) and the site (tests/test_parziali_parita.py) assert this same text.
const LISTING: &str = r#"{"format":"SciexWiff","samples":[{"id":1,"name":"rifiutato_s1","scans":1},{"id":2,"name":"rifiutato_s2","scans":0,"why":"empty"},{"id":3,"name":"rifiutato_s3","scans":0,"why":"refused","refused":{"error_key":"err.file.wiffScanType","params":{"code":"8","experiment":"1","type":"TOF MS"}}}]}"#;

#[test]
fn a_refused_sample_carries_its_reason_and_an_empty_one_stays_empty() {
    let Some((wiff, scan)) = pair() else { return };
    let member = |nome: &str, b: Vec<u8>| Member {
        nome: nome.into(),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    };
    let fs = FileSet {
        membri: vec![
            member("rifiutato.wiff", wiff),
            member("rifiutato.wiff.scan", scan),
        ],
    };
    let mut l = readers::apri(fs, &Limits::sito()).unwrap();
    let got = readers::samples_json(&mut *l);
    assert_eq!(got, LISTING, "the listing of the site and of the app");
    let v: serde_json::Value = serde_json::from_str(&got).unwrap();
    let s = v["samples"].as_array().unwrap();
    assert_eq!(s.len(), 3, "{v}");
    assert_eq!(
        (s[0]["scans"].as_u64(), s[0].get("why")),
        (Some(1), None),
        "{v}"
    );
    assert_eq!(s[2]["why"], "refused", "{v}");
    assert_eq!(s[2]["refused"]["error_key"], "err.file.wiffScanType", "{v}");
    assert_eq!(s[2]["refused"]["params"]["code"], "8", "{v}");
    assert_eq!(
        (s[1]["why"].as_str(), s[1].get("refused")),
        (Some("empty"), None),
        "{v}"
    );
}
