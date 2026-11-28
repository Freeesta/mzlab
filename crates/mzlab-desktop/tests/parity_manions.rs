//! The ion scale of the integrations table in the app (`rpc.rs` op `manint` with `ions`, and `manionchoices`) is the text `mzlab-cli peak table --ions`
//! writes for the same traces and request; the unchanged op without `ions` stays the cps table. The wasm leg is `tests/test_manions_wasm.py`.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::json;

/// Two samples of counts (20 cps per ion) with a peak at 2 min, and one window on each: a trace of `trace xic --ions` for the second.
fn trace(scale: f64) -> String {
    let (mut t, mut y) = (Vec::new(), Vec::new());
    let mut s: u64 = 3;
    for i in 0..400 {
        let x = i as f64 * 0.01;
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let bg = ((s >> 33) % 3) as f64;
        let peak = (40.0 * scale * (-((x - 2.0) / 0.1).powi(2)).exp()).round();
        t.push(format!("{x:.2}"));
        y.push(format!("{}", (bg + peak) * 20.0));
    }
    format!(
        "{{\"q1\":100.1,\"rt\":[{}],\"y\":[{}]}}",
        t.join(","),
        y.join(",")
    )
}

const WINDOWS: &str =
    "[{\"file\":0,\"tr\":0,\"a\":1.7,\"b\":2.3},{\"file\":1,\"tr\":0,\"a\":1.7,\"b\":2.3}]";

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

#[test]
fn the_app_and_the_cli_write_the_same_ion_table_and_choices() {
    let dir = tempfile::tempdir().unwrap();
    let files = format!(
        "[{{\"name\":\"a.mzML\",\"transitions\":[{}]}},{{\"name\":\"b.mzML\",\"transitions\":[{}]}}]",
        trace(1.0),
        trace(2.0)
    );
    let (tp, wp) = (dir.path().join("t.json"), dir.path().join("w.json"));
    std::fs::write(&tp, &files).unwrap();
    std::fs::write(&wp, WINDOWS).unwrap();
    // the CLI names both samples ems with a 100 ms fill time
    let o = mzlab_cli::execute(
        &[
            "peak",
            "table",
            "--traces",
            &tp.to_string_lossy(),
            "--windows",
            &wp.to_string_lossy(),
            "--ions",
            "--scan-type",
            "ems",
            "--set",
            "fill_ms=100",
            "--sums",
        ]
        .map(String::from),
    );
    assert_eq!(o.code, 0, "{}", o.stderr);
    let one =
        "{\"scan_type\":\"ems\",\"scan_from\":\"scan type given\",\"fill_ms\":100,\"traces\":[]}";
    let ions = format!("{{\"samples\":[{one},{one}]}}");
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    let got = text(
        st.rpc(
            "manint",
            &json!({"files": files, "windows": WINDOWS, "sums": true, "ions": ions}),
            &[],
        )
        .unwrap(),
    );
    assert_eq!(got.as_bytes(), &o.stdout[..]);
    // the same op without `ions` is the cps table, and the ion table starts with its bytes
    let plain = text(
        st.rpc(
            "manint",
            &json!({"files": files, "windows": WINDOWS, "sums": true}),
            &[],
        )
        .unwrap(),
    );
    assert_eq!(
        plain,
        mzlab_core::manint::table_text(&files, WINDOWS, true).unwrap()
    );
    for (a, b) in plain.lines().zip(got.lines()) {
        assert!(b.starts_with(a));
    }
    let choices = text(
        st.rpc("manionchoices", &json!({"files": files, "ions": ions}), &[])
            .unwrap(),
    );
    assert_eq!(
        choices,
        mzlab_core::manint::choices_text(&files, &ions).unwrap()
    );
    assert!(choices.contains("\"sample\":\"a\"") && choices.contains("\"sample\":\"b\""));
    // a scan type the engine does not know is its error
    assert!(text(
        st.rpc("manint", &json!({"files": files, "windows": WINDOWS, "ions": "{\"samples\":[{\"scan_type\":\"qqq\"}]}"}), &[])
            .unwrap()
    )
    .starts_with("E{"));
}
