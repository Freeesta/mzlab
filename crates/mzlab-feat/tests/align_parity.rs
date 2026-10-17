//! Alignment and RT map against the frozen answers of the former Python engine (`tests/golden/feat/allinea_sintetico.json`):
//! same groups (m/z, RT, area, height, apex, width) with and without a span limit and artefacts, same shift curves; the answer is the same
//! with the input files in any order of equal rows (two runs give the same bytes).

use std::path::Path;

use mzlab_feat::align::{align, AlignParams};
use mzlab_feat::rtmap::{build, RtParams};
use mzlab_feat::table::{Aligned, FileFeatures, CELL_ABSENT, CELL_DETECTED, FLAG_FT_LOBE};
use serde_json::Value;

fn golden() -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/feat/allinea_sintetico.json");
    serde_json::from_slice(&std::fs::read(&p).unwrap_or_else(|e| panic!("{p:?}: {e}"))).unwrap()
}

fn f64s(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap_or(f64::NAN))
        .collect()
}

fn file(d: &Value) -> FileFeatures {
    let f32s = |k: &str| {
        f64s(&d[k])
            .into_iter()
            .map(|x| x as f32)
            .collect::<Vec<f32>>()
    };
    let mz = f64s(&d["mz"]);
    let n = mz.len();
    FileFeatures {
        mz,
        rt: f32s("rt"),
        left: f32s("left"),
        right: f32s("right"),
        area: f64s(&d["area"]),
        height: f32s("height"),
        flags: d["artefact"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| {
                if b.as_bool().unwrap() {
                    FLAG_FT_LOBE
                } else {
                    0
                }
            })
            .collect(),
        apex: vec![0; n],
        ..FileFeatures::default()
    }
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a.is_nan() && b.is_nan()) || (a - b).abs() <= tol * b.abs().max(1.0)
}

fn same(got: &Aligned, want: &Value, what: &str) {
    let mz = f64s(&want["mz"]);
    assert_eq!(got.len(), mz.len(), "{what}: number of groups");
    let cols: [(&str, Vec<f64>, Vec<f64>, f64); 6] = [
        ("mz", got.mz.clone(), mz, 1e-9),
        ("rt", got.rt.clone(), f64s(&want["rt"]), 1e-6),
        ("area", got.area.clone(), f64s(&want["area"]), 1e-9),
        (
            "height",
            got.height.iter().map(|&x| x as f64).collect(),
            f64s(&want["height"]),
            1e-6,
        ),
        (
            "apex_rt",
            got.apex_rt.iter().map(|&x| x as f64).collect(),
            f64s(&want["apex_rt"]),
            1e-6,
        ),
        (
            "width",
            got.width.iter().map(|&x| x as f64).collect(),
            f64s(&want["width"]),
            1e-5,
        ),
    ];
    for (name, g, w, tol) in cols {
        assert_eq!(g.len(), w.len(), "{what}: length of {name}");
        if let Some(i) = (0..g.len()).find(|&i| !close(g[i], w[i], tol)) {
            panic!("{what}: {name}[{i}] = {} instead of {}", g[i], w[i]);
        }
    }
    for (i, &a) in got.area.iter().enumerate() {
        let present = !got.apex_rt[i].is_nan();
        assert_eq!(
            got.state[i],
            if present { CELL_DETECTED } else { CELL_ABSENT },
            "{what}: state of cell {i} (area {a})"
        );
    }
}

#[test]
fn groups_equal_python() {
    let g = golden();
    let files: Vec<FileFeatures> = g["files"].as_array().unwrap().iter().map(file).collect();
    let refs: Vec<&FileFeatures> = files.iter().collect();
    same(
        &align(&refs, &AlignParams::default()).unwrap(),
        &g["align"],
        "default",
    );
    let span = AlignParams {
        max_rt_span: Some(0.5),
        ..AlignParams::default()
    };
    same(
        &align(&refs, &span).unwrap(),
        &g["align_span"],
        "max_rt_span 0.5",
    );
    let keep = AlignParams {
        drop_artefacts: false,
        ..AlignParams::default()
    };
    same(
        &align(&refs, &keep).unwrap(),
        &g["align_keep"],
        "artefacts kept",
    );
}

#[test]
fn rt_map_equals_python() {
    let g = golden();
    let files: Vec<FileFeatures> = g["files"].as_array().unwrap().iter().map(file).collect();
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let al = align(&refs, &AlignParams::default()).unwrap();
    let m = build(&al, &RtParams::default());
    let w = &g["rtmap"];
    assert_eq!(m.ok(), w["ok"].as_bool().unwrap());
    assert_eq!(m.n_anchors as u64, w["anchors"].as_u64().unwrap());
    let near = |a: &[f64], b: &[f64], what: &str| {
        assert_eq!(a.len(), b.len(), "{what}");
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            assert!((x - y).abs() <= 1e-4, "{what}[{i}]: {x} instead of {y}");
        }
    };
    near(&m.grid, &f64s(&w["grid"]), "grid");
    near(&m.shift, &f64s(&w["shift"]), "shift");
    near(&m.rms, &f64s(&w["rms"]), "rms");
    let probe = f64s(&w["probe"]);
    for k in 0..m.n_files {
        let row = |key: &str| f64s(&w[key][k]);
        near(
            &probe.iter().map(|&r| m.at(k, r)).collect::<Vec<_>>(),
            &row("at"),
            "at",
        );
        near(
            &probe.iter().map(|&r| m.expected(r, k)).collect::<Vec<_>>(),
            &row("expected"),
            "expected",
        );
        near(
            &probe.iter().map(|&r| m.reference(r, k)).collect::<Vec<_>>(),
            &row("reference"),
            "reference",
        );
    }
}

#[test]
fn two_runs_same_bytes_and_empty_series() {
    let g = golden();
    let files: Vec<FileFeatures> = g["files"].as_array().unwrap().iter().map(file).collect();
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let run = || format!("{:?}", align(&refs, &AlignParams::default()).unwrap());
    assert_eq!(run(), run());
    let empty = FileFeatures::default();
    let al = align(&[&empty, &empty], &AlignParams::default()).unwrap();
    assert!(al.is_empty() && al.n_files == 2);
    assert!(!build(&al, &RtParams::default()).ok());
}
