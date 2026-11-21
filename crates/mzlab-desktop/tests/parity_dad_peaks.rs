//! `dad peaks` of the CLI holds the bytes of the app's `lr_pda_peaks` (the site's `item_pda_peaks` runs the same `Routes::pda_peaks_item`): the UV peaks with their
//! lambda-max and the MS ions at the delay-corrected time, for the total trace and for a band, with the delay estimated and given.
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::json;
use std::path::{Path, PathBuf};

const NAME: &str = "FullScan_t10.mzML";

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mzlab/web/esempi")
        .join(NAME)
}

#[test]
fn the_cli_dad_peaks_holds_the_bytes_of_the_app_route() {
    let dir = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let (fid, _) = st.pick(example()).unwrap();
    st.rpc("lr_put", &json!({"name": NAME, "fid": fid}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": NAME}), &[]).unwrap();
    // (the app's parameters, the CLI's options): a NaN is not JSON, so what the page leaves out is left out here too
    let cases = [
        (json!({"name": NAME}), vec![]),
        (
            json!({"name": NAME, "top": 2, "offset": 0.0}),
            vec!["--top", "2", "--offset", "0"],
        ),
        (
            json!({"name": NAME, "nm0": 250.0, "nm1": 270.0, "top": 3, "level": 1, "offset": -0.05}),
            vec![
                "--nm", "250,270", "--top", "3", "--level", "1", "--offset", "-0.05",
            ],
        ),
    ];
    for (p, opts) in cases {
        let want = match st.rpc("lr_pda_peaks", &p, &[]).unwrap() {
            Answer::Text(t) => t,
            Answer::Bytes(_) => panic!("bytes"),
        };
        assert!(want.starts_with("{\"peaks\":["), "{want}");
        let mut args: Vec<String> = ["dad", "peaks"].iter().map(|s| s.to_string()).collect();
        args.push(example().to_string_lossy().into_owned());
        args.extend(opts.iter().map(|s| s.to_string()));
        let o = mzlab_cli::execute(&args);
        assert_eq!(o.code, 0, "{args:?}: {}", o.stderr);
        assert_eq!(
            String::from_utf8(o.stdout).unwrap(),
            want + "\n",
            "{args:?}"
        );
    }
}
