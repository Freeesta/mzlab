//! `normative import-official`: the file it writes is the one `mzlab_feat::normative::official::import_official` gives (the function the app and the .wasm run).

use super::run_with;
use crate::normative::fetch::Http;

const CSV: &str = include_str!("../../../mzlab-feat/tests/data/official/candidate-list.csv");
const CAS: &str = include_str!("../../../mzlab-feat/tests/data/official/cas.tsv");

#[test]
fn the_command_writes_the_bytes_of_the_shared_function() {
    let d = std::env::temp_dir().join(format!("mzlab-import-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let (csv, cas, out) = (
        d.join("candidate-list.csv"),
        d.join("cas.tsv"),
        d.join("o.mzsus"),
    );
    std::fs::write(&csv, CSV).unwrap();
    std::fs::write(&cas, CAS).unwrap();
    let a = |s: &std::path::Path| s.to_string_lossy().into_owned();
    let args = [
        "import-official",
        "svhc",
        &a(&csv),
        &a(&cas),
        &a(&out),
        "--date",
        "2026-10-30",
    ]
    .map(String::from)
    .to_vec();
    let o = run_with(args, Http::default());
    assert_eq!(o.code, 0, "{}", o.stderr);
    let want = mzlab_feat::normative::official::import_official(
        "svhc",
        CSV.as_bytes(),
        CAS,
        r#"{"file":"candidate-list.csv","date":"2026-10-30"}"#,
    )
    .unwrap();
    let n = u32::from_le_bytes([want[0], want[1], want[2], want[3]]) as usize;
    assert_eq!(std::fs::read(&out).unwrap(), &want[4 + n..]);
    assert_eq!(
        String::from_utf8(o.stdout).unwrap(),
        format!("{}\n", String::from_utf8_lossy(&want[4..4 + n]))
    );
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_wrong_call_is_a_usage_error() {
    let o = run_with(
        vec!["import-official".into(), "svhc".into()],
        Http::default(),
    );
    assert_eq!(o.code, 2);
}
