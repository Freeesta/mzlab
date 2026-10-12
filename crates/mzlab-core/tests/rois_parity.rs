//! `rois::build_rois` and `rois::extract_trace` give what `ionfamily.build_rois` and `ionfamily.extract_trace` give (`tools/roi_riferimento.py`):
//! the same ROIs (count, scans, intensities within 1e-9 relative, centres within 1e-9). Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::rois::{build_rois, extract_trace};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}
fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}
/// ROIs with the same centre have no fixed order in numpy (`argsort` is not stable): rows are put in a canonical order, by centre and then by content.
fn canon(mz: &[f64], x: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    let m = mz.len();
    let mut o: Vec<usize> = (0..m).collect();
    let key = |r: usize| {
        let row = &x[r * n..(r + 1) * n];
        (
            (mz[r] * 1e9).round() as i64,
            row.iter().sum::<f64>().to_bits(),
            row.iter().position(|v| *v != 0.0).unwrap_or(n),
        )
    };
    o.sort_by_key(|&r| key(r));
    (
        o.iter().map(|&i| mz[i]).collect(),
        o.iter()
            .flat_map(|&i| x[i * n..(i + 1) * n].to_vec())
            .collect(),
    )
}

fn same(what: &str, got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{what}: length");
    if let Some(i) = (0..got.len()).find(|&i| !close(got[i], want[i])) {
        if let Ok(n) = std::env::var("ROI_DEBUG") {
            let n: usize = n.parse().unwrap_or(1);
            let r = i / n;
            let row = |a: &[f64]| {
                a[r * n..(r + 1) * n]
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| **v != 0.0)
                    .map(|(c, v)| format!("{c}:{v:.1}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            eprintln!(
                "row {r} col {}\n got  {}\n want {}",
                i % n,
                row(got),
                row(want)
            );
        }
        panic!("{what}: element {i}: {} against {}", got[i], want[i]);
    }
}

#[test]
fn rust_rois_match_python() {
    let out = std::env::temp_dir().join(format!("mzlab-roi-{}", std::process::id()));
    let mut cmd = Command::new("python3");
    cmd.current_dir(root())
        .arg("tools/roi_riferimento.py")
        .arg(&out);
    if let Ok(rd) = std::fs::read_dir(root().join("mzlab/web/esempi")) {
        let mut v: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "mzML"))
            .collect();
        v.sort();
        cmd.args(v);
    }
    let listing = match cmd.output() {
        Ok(o) if o.status.success() => String::from_utf8(o.stdout).unwrap(),
        other => {
            if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
                panic!("the reference did not run: {other:?}");
            }
            eprintln!("python3/numpy not available: test skipped");
            return;
        }
    };
    let mut compared = 0;
    for line in listing.lines().filter(|l| !l.is_empty()) {
        let stem = PathBuf::from(line)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let p = out.join(format!("{stem}.mzidx"));
        let size = std::fs::metadata(&p).unwrap().len();
        let mut idx = Index::open(std::fs::File::open(&p).unwrap(), size).unwrap();
        let cases: Vec<Value> =
            serde_json::from_slice(&std::fs::read(out.join(format!("{stem}.rois.json"))).unwrap())
                .unwrap();
        let Some(si) = idx.section_of(1, None) else {
            continue;
        };
        for c in &cases {
            let (pa, re) = (&c["params"], &c["result"]);
            let what = format!("{stem} {pa}");
            if c["kind"] == "rois" {
                let range = pa["mz_range"]
                    .as_array()
                    .map(|r| (r[0].as_f64().unwrap(), r[1].as_f64().unwrap()));
                let w = build_rois(
                    &mut idx,
                    si,
                    pa["rt0"].as_f64().unwrap(),
                    pa["rt1"].as_f64().unwrap(),
                    pa["tol"].as_f64().unwrap(),
                    pa["min_scans"].as_i64().unwrap(),
                    pa["max_gap"].as_i64().unwrap(),
                    range,
                    pa["min_int"].as_f64().unwrap(),
                )
                .unwrap();
                assert_eq!(
                    w.n_roi() as u64,
                    re["n_roi"].as_u64().unwrap(),
                    "{what}: ROIs"
                );
                assert_eq!(
                    w.n_scans as u64,
                    re["n_scans"].as_u64().unwrap(),
                    "{what}: scans"
                );
                assert_eq!(
                    w.scan_lo as u64,
                    re["scan_lo"].as_u64().unwrap(),
                    "{what}: scan_lo"
                );
                same(&format!("{what} rt"), &w.rt, &nums(&re["rt"]));
                let n = w.n_scans.max(1);
                let (gm, gx) = canon(&w.mz, &w.x, n);
                let (wm, wx) = canon(&nums(&re["mz"]), &nums(&re["x"]), n);
                same(&format!("{what} centres"), &gm, &wm);
                same(&format!("{what} intensities"), &gx, &wx);
                if w.n_roi() > 0 || re["mz_lo"].is_number() {
                    assert_eq!(w.n_peaks, re["n_peaks"].as_u64().unwrap(), "{what}: peaks");
                    assert!(
                        close(w.mz_lo.unwrap(), re["mz_lo"].as_f64().unwrap())
                            && close(w.mz_hi.unwrap(), re["mz_hi"].as_f64().unwrap()),
                        "{what}: range"
                    );
                }
            } else {
                let win = pa["win"]
                    .as_array()
                    .map(|r| (r[0].as_f64().unwrap(), r[1].as_f64().unwrap()));
                let (rt, y) = extract_trace(
                    &mut idx,
                    si,
                    pa["mz"].as_f64().unwrap(),
                    pa["tol"].as_f64().unwrap(),
                    win,
                )
                .unwrap();
                same(&format!("{what} rt"), &rt, &nums(&re["rt"]));
                same(&format!("{what} y"), &y, &nums(&re["y"]));
            }
            compared += 1;
        }
    }
    assert!(compared >= 100, "{compared} comparisons");
    let _ = std::fs::remove_dir_all(&out);
}
