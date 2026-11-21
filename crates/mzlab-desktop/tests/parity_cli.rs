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
    smoothed_and_listed_xic(&run, &route, &file);
    swept_xic(&run, &route, &file);
    offset_xic(&run, &route, &file);
}

/// The smoothed XIC holds the bytes of the app's route with `smooth`, and a list of ions the documents of each ion alone.
fn smoothed_and_listed_xic(
    run: &dyn Fn(&[&str]) -> String,
    route: &dyn Fn(&str, &str) -> String,
    file: &str,
) {
    // the smoothed XIC: the app's route with `smooth` (the one the site asks through the wasm `route`) holds the bytes of `--smooth`
    let smooth = run(&[
        "trace", "xic", file, "--mz", "300", "--da", "0.5", "--smooth", "3",
    ]);
    assert!(smooth.contains(&route_points(
        &route("/api/xic", "?mz=300&tol=0.5&level=1&smooth=3"),
        "rt"
    )));
    // a list of ions: the whole document is the head and the document of each ion alone (the same route bytes), in the order given
    let set = run(&["trace", "xic", file, "--mz", "300,241.3", "--da", "0.5"]);
    let first = run(&["trace", "xic", file, "--mz", "300", "--da", "0.5"]);
    let alone = run(&["trace", "xic", file, "--mz", "241.3", "--da", "0.5"]);
    let head = set.split(",\"traces\":[").next().unwrap();
    assert_eq!(
        set.trim(),
        format!("{head},\"traces\":[{},{}]}}", first.trim(), alone.trim())
    );
    assert!(head.starts_with("{\"schema\":\"trace-set/1\"") && head.ends_with("\"n_traces\":2"));
}

/// A sweep: `--sweep` is the bytes of `--mz` with the expanded list, and each trace of the app's route (`/api/xic?sweep=`, the one the site asks through
/// the wasm `route`) is the points of the same ion in the CLI document.
fn swept_xic(run: &dyn Fn(&[&str]) -> String, route: &dyn Fn(&str, &str) -> String, file: &str) {
    let sweep = run(&[
        "trace",
        "xic",
        file,
        "--sweep",
        "299:302:1",
        "--da",
        "0.5",
        "--smooth",
        "3",
    ]);
    let list = run(&[
        "trace",
        "xic",
        file,
        "--mz",
        "299,300,301,302",
        "--da",
        "0.5",
        "--smooth",
        "3",
    ]);
    assert_eq!(sweep, list);
    let routed = route("/api/xic", "?sweep=299:302:1&tol=0.5&level=1&smooth=3");
    assert!(routed.starts_with("{\"schema\":\"trace-set/1\",\"n_traces\":4,"));
    let members: Vec<&str> = routed.split("{\"id\":").skip(1).collect();
    assert_eq!(members.len(), 4);
    for m in members {
        assert!(
            sweep.contains(&route_points(m, "rt")),
            "{}",
            &m[..m.len().min(80)]
        );
    }
}

/// `--mz-offset`: the trace of the CLI is the app's route with `mzoff` (the one the site asks through the wasm `route`), for a number, for `auto:` and in a
/// sweep, and the evidence of the offset is the same text.
fn offset_xic(run: &dyn Fn(&[&str]) -> String, route: &dyn Fn(&str, &str) -> String, file: &str) {
    for spec in ["0.24", "-0.31", "auto:300", "auto:300@0.3"] {
        let doc = run(&[
            "trace",
            "xic",
            file,
            "--mz",
            "300",
            "--da",
            "0.5",
            "--mz-offset",
            spec,
        ]);
        let routed = route("/api/xic", &format!("?mz=300&tol=0.5&level=1&mzoff={spec}"));
        assert!(doc.contains(&route_points(&routed, "rt")), "{spec}");
        // `"mz_offset_da":D,"mz_offset":{...}` (the evidence object holds no other braces)
        let evidence = |t: &str| {
            let from = &t[t.find("\"mz_offset_da\":").unwrap()..];
            from[..from.find('}').unwrap() + 1].to_string()
        };
        assert_eq!(evidence(&doc), evidence(&routed), "{spec}");
    }
    let sweep = run(&[
        "trace",
        "xic",
        file,
        "--sweep",
        "299:302:1",
        "--da",
        "0.5",
        "--mz-offset",
        "0.24",
    ]);
    let routed = route("/api/xic", "?sweep=299:302:1&tol=0.5&level=1&mzoff=0.24");
    for m in routed.split("{\"id\":").skip(1) {
        assert!(
            sweep.contains(&route_points(m, "rt")),
            "{}",
            &m[..m.len().min(80)]
        );
    }
    // the evidence of an estimate is `api/mzcal`
    let plan = run(&[
        "trace",
        "xic",
        file,
        "--mz",
        "300",
        "--da",
        "0.5",
        "--mz-offset",
        "auto:300",
        "--dry-run",
    ]);
    let mzcal = route("/api/mzcal", "?mz=300");
    assert!(
        plan.contains(mzcal.trim_start_matches('{').trim_end_matches('}')),
        "{mzcal}"
    );
}
