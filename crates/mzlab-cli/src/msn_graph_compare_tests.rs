use crate::msn_graph_compare::compare;

/// A minimal `msn-graph/1`: a root (m/z 305.1) isolated at MS1, a fragment (244.06) isolated at MS2 and an arc between them; `shift` moves the fragment in ppm,
/// `formula` is the one chosen for it and `peaks` the MS2 spectrum of the root.
fn graph(shift: f64, formula: &str, peaks: &[(f64, f64)]) -> String {
    let frag = 244.06 * (1.0 + shift * 1e-6);
    let p: Vec<String> = peaks
        .iter()
        .map(|(m, r)| format!("{{\"mz\":{m},\"rel\":{r},\"node\":null}}"))
        .collect();
    format!(
        "{{\"schema\":\"msn-graph/1\",\"nodes\":[\
         {{\"id\":\"n0\",\"mz\":305.1,\"isolated\":true,\"tree\":[\"305@cid35\"],\"formula\":{{\"choice\":\"C14H15N3O5\"}}}},\
         {{\"id\":\"n1\",\"mz\":{frag},\"isolated\":true,\"tree\":[\"305@cid35>244@cid35\"],\"formula\":{{\"choice\":\"{formula}\"}}}}],\
         \"edges\":[{{\"from\":\"n0\",\"to\":\"n1\",\"measurements\":[{{\"rel\":51.4}}]}}],\
         \"spectra\":[{{\"tree\":\"305@cid35\",\"level\":2,\"activation\":\"cid\",\"ce\":35,\"peaks\":[{}]}}]}}",
        p.join(",")
    )
}

const PEAKS: [(f64, f64); 3] = [(217.12, 23.4), (244.06, 100.0), (261.08, 40.0)];

fn ok_of(report: &str, criterion: &str) -> bool {
    let at = report.find(&format!("\"{criterion}\"")).unwrap();
    report[at..]
        .split("\"ok\":")
        .nth(1)
        .unwrap()
        .starts_with("true")
}

#[test]
fn the_same_tree_twice_is_validated_with_its_numbers() {
    let g = graph(0.0, "C13H10NO4", &PEAKS);
    let r = compare(&g, &g).unwrap();
    assert!(r.starts_with("{\"schema\":\"msn-compare/1\""), "{r}");
    assert!(r.contains("\"validated\":true"), "{r}");
    assert!(
        r.contains("\"common\":2") && r.contains("\"min\":1") && r.contains("\"max\":0"),
        "{r}"
    );
}

#[test]
fn a_shift_a_formula_or_a_spectrum_that_differ_fail_their_own_criterion() {
    let a = graph(0.0, "C13H10NO4", &PEAKS);
    // 4 ppm: still the same node, but past the 3 ppm limit
    let r = compare(&a, &graph(4.0, "C13H10NO4", &PEAKS)).unwrap();
    assert!(
        !ok_of(&r, "deviation_ppm")
            && ok_of(&r, "isolated_nodes")
            && r.contains("\"validated\":false"),
        "{r}"
    );
    // 10 ppm: the isolated node is not found in the other graph
    let r = compare(&a, &graph(10.0, "C13H10NO4", &PEAKS)).unwrap();
    assert!(!ok_of(&r, "isolated_nodes") && !ok_of(&r, "edges"), "{r}");
    // another formula for the same node
    let r = compare(&a, &graph(0.0, "C12H8N2O4", &PEAKS)).unwrap();
    assert!(
        !ok_of(&r, "formulas") && r.contains("\"different\":1"),
        "{r}"
    );
    // an MS2 spectrum with other peaks
    let r = compare(
        &a,
        &graph(0.0, "C13H10NO4", &[(150.0, 100.0), (200.0, 30.0)]),
    )
    .unwrap();
    assert!(!ok_of(&r, "ms2_entropy") && ok_of(&r, "formulas"), "{r}");
}

#[test]
fn a_file_that_is_not_a_graph_is_a_keyed_error() {
    assert_eq!(
        compare("{\"schema\":\"msn-lib/1\"}", "{}")
            .err()
            .unwrap()
            .key,
        "err.msnlib.bad"
    );
}

#[test]
fn the_command_compares_two_graphs_of_the_same_infusion() {
    let d = std::env::temp_dir().join(format!("mzlab_msncmp_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let mzml = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/golden/ref/msntree_rotta/msn.mzML"
    );
    let go =
        |args: &[&str]| crate::execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let g = d.join("g.json");
    let o = go(&[
        "msn-graph",
        mzml,
        "--formula",
        "C14H15N3O5+",
        "--out",
        g.to_str().unwrap(),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let o = go(&[
        "msn-graph",
        "compare",
        g.to_str().unwrap(),
        g.to_str().unwrap(),
    ]);
    let r = String::from_utf8(o.stdout).unwrap();
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        r.contains("\"validated\":true") && r.contains("\"min\":1"),
        "{r}"
    );
    assert_eq!(go(&["msn-graph", "compare", g.to_str().unwrap()]).code, 2);
    let _ = std::fs::remove_dir_all(&d);
}
