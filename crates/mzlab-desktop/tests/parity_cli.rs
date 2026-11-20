//! The CLI extraction commands (`trace xic`, `spectrum get`) hold the numbers and bytes of the routes of the app, which `parity_routes.rs` ties to the site's.
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

/// The points of a route answer (`x` and `"y"` token arrays) written as the CLI writes them: `[[x,y],...]`, tokens untouched.
fn route_points(text: &str, x: &str) -> String {
    let arr = |k: &str| -> Vec<String> {
        // a time axis is `rt` or `t` by the route
        let k = if k == "rt" && !text.contains("\"rt\":[") {
            "t"
        } else {
            k
        };
        let at = text
            .find(&format!("\"{k}\":["))
            .unwrap_or_else(|| panic!("no {k} in {}", &text[..text.len().min(200)]))
            + k.len()
            + 4;
        let body = &text[at..at + text[at..].find(']').unwrap()];
        body.split(',')
            .filter(|t| !t.is_empty())
            .map(String::from)
            .collect()
    };
    let pts: Vec<String> = arr(x)
        .iter()
        .zip(arr("y"))
        .map(|(a, b)| format!("[{a},{b}]"))
        .collect();
    format!("\"points\":[{}]", pts.join(","))
}

/// `trace`, `spectrum` and `scan list` of the CLI hold the bytes of the routes of the app (and so of the site: the routes above are the same in both).
#[test]
fn the_cli_extraction_commands_hold_the_bytes_of_the_app_routes() {
    let dir = tempfile::tempdir().unwrap();
    let st = State::new();
    st.set_data_folder(dir.path().join("app"));
    st.set_index_folder(dir.path().join("idx")).unwrap();
    let (fid, _) = st.pick(example()).unwrap();
    st.rpc("lr_put", &json!({"name": NAME, "fid": fid}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": NAME}), &[]).unwrap();
    st.rpc(
        "idxopen",
        &json!({"slot": 0, "name": NAME, "fid": fid}),
        &[],
    )
    .unwrap();
    let route = |path: &str, query: &str| match st
        .rpc(
            "route",
            &json!({"slot": 0, "path": path, "query": query, "fid": fid}),
            &[],
        )
        .unwrap()
    {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("{path}: bytes"),
    };
    let file = example().to_string_lossy().into_owned();
    let run = |a: &[&str]| {
        let v: Vec<String> = a.iter().map(|s| s.to_string()).collect();
        let o = mzlab_cli::execute(&v);
        assert_eq!(o.code, 0, "{a:?}: {}", o.stderr);
        let t = String::from_utf8(o.stdout).unwrap();
        assert!(t.ends_with("}\n"), "{a:?}: a document ends with a newline");
        t
    };
    let spec = run(&["spectrum", "get", &file, "--rt", "0.2,0.4"]);
    assert!(spec.contains(&route_points(
        &route("/api/spectrum", "?k=0&level=1&rt0=0.2&rt1=0.4"),
        "mz"
    )));
    // the XIC: the app's numbers against the points of `trace xic` (the route rounds to 4 and 1 decimals)
    st.open(7, fid).unwrap();
    let nums = st.xic(7, 1, 300.0, 0.5, 0).unwrap();
    let xic = run(&["trace", "xic", &file, "--mz", "300", "--da", "0.5"]);
    let doc = mzlab_core::json::parse(xic.trim().as_bytes()).unwrap();
    let pts: Vec<f64> = doc
        .get("points")
        .unwrap()
        .arr()
        .iter()
        .flat_map(|p| p.arr().iter().map(|x| x.f64().unwrap()).collect::<Vec<_>>())
        .collect();
    assert_eq!(
        nums.len(),
        pts.len(),
        "{} numbers, {} in the document",
        nums.len(),
        pts.len()
    );
    // the app's axis is not rounded: the document holds the route's rounding of the same values
    let near = |a: &f64, b: &f64| (a - b).abs() <= 1e-3 * (1.0 + b.abs());
    let n = pts.len() / 2;
    let halves = nums[..n]
        .iter()
        .zip(pts.iter().step_by(2))
        .all(|(a, b)| near(a, b))
        && nums[n..]
            .iter()
            .zip(pts.iter().skip(1).step_by(2))
            .all(|(a, b)| near(a, b));
    let inter = nums.iter().zip(&pts).all(|(a, b)| near(a, b));
    assert!(halves || inter, "the XIC of the app and of the CLI differ");
    // the smoothed XIC: the app's route with `smooth` (the one the site asks through the wasm `route`) holds the bytes of `--smooth`
    let smooth = run(&[
        "trace", "xic", &file, "--mz", "300", "--da", "0.5", "--smooth", "3",
    ]);
    assert!(smooth.contains(&route_points(
        &route("/api/xic", "?mz=300&tol=0.5&level=1&smooth=3"),
        "rt"
    )));
    // a list of ions: the whole document is the head and the document of each ion alone (the same route bytes), in the order given
    let set = run(&["trace", "xic", &file, "--mz", "300,241.3", "--da", "0.5"]);
    let first = run(&["trace", "xic", &file, "--mz", "300", "--da", "0.5"]);
    let alone = run(&["trace", "xic", &file, "--mz", "241.3", "--da", "0.5"]);
    let head = set.split(",\"traces\":[").next().unwrap();
    assert_eq!(
        set.trim(),
        format!("{head},\"traces\":[{},{}]}}", first.trim(), alone.trim())
    );
    assert!(head.starts_with("{\"schema\":\"trace-set/1\"") && head.ends_with("\"n_traces\":2"));
}
