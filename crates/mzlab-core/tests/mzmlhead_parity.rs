//! `mzmlhead` reads the chromatogram list, the chromatogram arrays and the metadata of an mzML like `mzlab/reader/mzml.py`
//! (`tools/mzmlhead_riferimento.py`) on the synthetic files and the examples. Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::mzmlhead::Mzml;
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
fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn head_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab-head-{}", std::process::id()));
    let mut cmd = Command::new("python3");
    cmd.current_dir(root())
        .arg("tools/mzmlhead_riferimento.py")
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
    let (mut files, mut chroms, mut srm) = (0, 0, 0);
    for line in listing.lines().filter(|l| !l.is_empty()) {
        let stem = PathBuf::from(line)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let idx_path = out.join(format!("{stem}.mzidx"));
        let size = std::fs::metadata(&idx_path).unwrap().len();
        let idx = Index::open(std::fs::File::open(&idx_path).unwrap(), size).unwrap();
        let scans_end = idx.scans.last().map_or(0, |s| s.end);
        let want: Value =
            serde_json::from_slice(&std::fs::read(out.join(format!("{stem}.head.json"))).unwrap())
                .unwrap();
        let f = std::fs::File::open(line).unwrap();
        let mut m = Mzml::new(f, std::fs::metadata(line).unwrap().len());
        let list = m
            .chromatograms(scans_end)
            .unwrap_or_else(|e| panic!("{stem}: {e}"));
        let w = want["chroms"].as_array().unwrap();
        assert_eq!(list.len(), w.len(), "{stem}: chromatograms");
        for (c, wc) in list.iter().zip(w) {
            let what = format!("{stem} chromatogram {}", c.index);
            assert_eq!(c.id, wc["id"].as_str().unwrap(), "{what}");
            assert_eq!(c.kind, wc["kind"].as_str().unwrap(), "{what}");
            assert_eq!(
                (c.start, c.end),
                (wc["start"].as_u64().unwrap(), wc["end"].as_u64().unwrap()),
                "{what}: offsets"
            );
            for (k, g) in [("q1", c.q1), ("q3", c.q3), ("ce", c.ce), ("dwell", c.dwell)] {
                match (g, wc.get(k).and_then(Value::as_f64)) {
                    (Some(a), Some(b)) => assert!(close(a, b), "{what} {k}"),
                    (None, None) => {}
                    (a, b) => panic!("{what} {k}: {a:?} against {b:?}"),
                }
            }
            assert_eq!(c.n, wc["n"].as_i64(), "{what}: n");
            if c.kind == "srm" {
                assert_eq!(c.name, wc["name"].as_str().unwrap(), "{what}");
                srm += 1;
            }
            let (t, y) = m.chromatogram(c).unwrap_or_else(|e| panic!("{what}: {e}"));
            let (wt, wy) = (nums(&wc["t"]), nums(&wc["y"]));
            assert_eq!((t.len(), y.len()), (wt.len(), wy.len()), "{what}: points");
            assert!(
                t.iter().zip(&wt).all(|(a, b)| close(*a, *b))
                    && y.iter().zip(&wy).all(|(a, b)| close(*a, *b)),
                "{what}: values"
            );
            assert_eq!(
                m.chromatogram_unit(c).unwrap(),
                wc["unit"].as_str().unwrap(),
                "{what}: unit"
            );
            chroms += 1;
        }
        let md = m.metadata().unwrap_or_else(|e| panic!("{stem}: {e}"));
        let wm = &want["metadata"];
        assert_eq!(
            md.instrument,
            wm["instrument"].as_str().unwrap(),
            "{stem}: instrument"
        );
        assert_eq!(md.serial, wm["serial"].as_str().unwrap(), "{stem}: serial");
        assert_eq!(md.components, strs(&wm["components"]), "{stem}: components");
        assert_eq!(md.analyzers, strs(&wm["analyzers"]), "{stem}: analyzers");
        assert_eq!(
            md.source_files,
            strs(&wm["source_files"]),
            "{stem}: source files"
        );
        assert_eq!(md.software, strs(&wm["software"]), "{stem}: software");
        assert_eq!(
            m.header_polarity().unwrap(),
            want["header_polarity"].as_str().unwrap(),
            "{stem}: polarity"
        );
        assert_eq!(
            m.nce().unwrap(),
            want["nce"].as_bool().unwrap(),
            "{stem}: nce"
        );
        files += 1;
    }
    assert!(
        files >= 8 && chroms >= 4 && srm >= 2,
        "{files} files, {chroms} chromatograms ({srm} SRM)"
    );
    let _ = std::fs::remove_dir_all(&out);
}
