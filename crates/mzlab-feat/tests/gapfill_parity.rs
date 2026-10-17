//! `gapfill::fill_file` against the frozen answers of the former Python `fill_gaps` (`tests/golden/feat/riempimento.json`, on the identity series of
//! `tools/feat_sintetici.py --solo identita`): the same cells are filled, with and without the RT map, and the areas agree within 1e-4 (relative). Filled cells have `CELL_FILLED` and never `CELL_DETECTED`. Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_feat::gapfill::{fill_file, FillParams};
use mzlab_feat::rtmap::RtMap;
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED, CELL_FILLED};
use mzlab_feat::trace::Params;
use serde_json::Value;

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap_or(f64::NAN))
        .collect()
}

fn aligned(d: &Value) -> Aligned {
    let nf = d["n_files"].as_u64().unwrap() as usize;
    let area = nums(&d["area"]);
    let apex: Vec<f32> = nums(&d["apex_rt"]).into_iter().map(|x| x as f32).collect();
    Aligned {
        n_files: nf,
        mz: nums(&d["mz"]),
        rt: nums(&d["rt"]),
        state: apex
            .iter()
            .map(|a| {
                if a.is_nan() {
                    CELL_ABSENT
                } else {
                    CELL_DETECTED
                }
            })
            .collect(),
        height: vec![0.0; area.len()],
        width: nums(&d["width"]).into_iter().map(|x| x as f32).collect(),
        apex_rt: apex,
        area,
        ..Aligned::default()
    }
}

fn rt_map(d: &Value, nf: usize) -> RtMap {
    let m = &d["rtmap"];
    RtMap {
        n_files: nf,
        grid: nums(&m["grid"]),
        shift: nums(&m["shift"]),
        n_anchors: m["anchors"].as_u64().unwrap() as usize,
        rms: nums(&m["rms"]),
    }
}

fn run_fill(d: &Value, dir: &Path, map: Option<&RtMap>) -> (Aligned, usize) {
    let mut al = aligned(d);
    let mut total = 0;
    for (k, name) in d["files"].as_array().unwrap().iter().enumerate() {
        let f = std::fs::File::open(dir.join(name.as_str().unwrap())).unwrap();
        let size = f.metadata().unwrap().len();
        let mut ix = Index::open(f, size).unwrap();
        total += fill_file(
            &mut ix,
            &mut al,
            k,
            map,
            &Params::default(),
            &FillParams::default(),
        )
        .unwrap();
    }
    (al, total)
}

fn check(what: &str, got: &Aligned, want: &Value) {
    let (area, filled) = (nums(&want["area"]), nums(&want["filled"]));
    for i in 0..area.len() {
        let is = got.state[i] == CELL_FILLED;
        assert_eq!(is, filled[i] > 0.0, "{what}: cell {i} filled");
        let tol = 1e-4 * area[i].abs().max(1.0);
        assert!(
            (got.area[i] - area[i]).abs() <= tol,
            "{what}: cell {i} area {} against {}",
            got.area[i],
            area[i]
        );
    }
}

#[test]
fn fill_like_the_golden_file() {
    let out = std::env::temp_dir().join(format!("mzlab_fill_{}", std::process::id()));
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ok = Command::new("python3")
        .arg(root.join("tools/feat_sintetici.py"))
        .arg(&out)
        .args(["--solo", "identita"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return;
    }
    let d: Value = serde_json::from_slice(
        &std::fs::read(root.join("tests/golden/feat/riempimento.json")).unwrap(),
    )
    .unwrap();
    let map = rt_map(&d, d["n_files"].as_u64().unwrap() as usize);
    let (with_map, n1) = run_fill(&d, &out, Some(&map));
    check("with the map", &with_map, &d["with_map"]);
    let (no_map, n2) = run_fill(&d, &out, None);
    check("without the map", &no_map, &d["no_map"]);
    // a filled cell is not a detected one: the detected cells are the same before and after
    let detected = |a: &Aligned| a.state.iter().filter(|&&s| s == CELL_DETECTED).count();
    assert_eq!(detected(&with_map), detected(&aligned(&d)));
    assert_eq!(detected(&no_map), detected(&aligned(&d)));
    assert!((0..with_map.state.len())
        .all(|i| with_map.state[i] != CELL_FILLED || with_map.apex_rt[i].is_nan()));
    let _ = std::fs::remove_dir_all(&out);
    assert!(
        n1 >= 20 && n2 >= 20,
        "the synthetic series fills too little: {n1} and {n2}"
    );
}
