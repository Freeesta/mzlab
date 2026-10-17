//! `hr/filters.py::isf_hidden` against the golden file of the reference (`tests/golden/finder_funzioni.json`).

use super::*;

fn file(t: f64) -> FileIn {
    FileIn {
        name: String::new(),
        label: String::new(),
        kind: "sample".into(),
        time: Some(t),
        t,
        cond: String::new(),
        replica: None,
        order: 0,
    }
}

fn tr(area: f64, height: f64, apex: usize) -> Trace {
    Trace { area, height, apex }
}

/// `hr/filters.py::isf_hidden` on the cases of `tests/golden/finder_funzioni.json`: the verdict follows the ratio test or the shift of the apex.
#[test]
fn the_verdict_matches_the_python_reference() {
    let g: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/finder_funzioni.json"
    ))
    .unwrap();
    for case in g["isf_hidden"].as_array().unwrap() {
        let nums = |k: &str| -> Vec<f64> {
            case[k]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        };
        let files: Vec<FileIn> = nums("times").into_iter().map(file).collect();
        let parent: Vec<Trace> = nums("parent")
            .into_iter()
            .map(|a| tr(a, 1e7, 100))
            .collect();
        let ion: Vec<Trace> = case["ion"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| {
                tr(
                    x[0].as_f64().unwrap(),
                    x[2].as_f64().unwrap(),
                    (100.0 + x[1].as_f64().unwrap()) as usize,
                )
            })
            .collect();
        assert_eq!(
            is_hidden(&files, &parent, &ion),
            case["candidate"].as_bool().unwrap(),
            "case {case}"
        );
    }
}
