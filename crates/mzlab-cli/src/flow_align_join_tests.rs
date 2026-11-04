//! The algorithms `join` of `features.align` and `peak_finder` of `features.gapfill` run from a Flow plan by `mzlab-cli flow run`, on three synthetic
//! high-resolution files: an ion whose RT drifts from file to file, that the join keeps in fewer rows than the RT gaps of the default, and an ion that
//! a file shows only as a rising ramp, which the integration fills and the peak finder (no apex) leaves empty; `--set` reaches every parameter.

use std::fmt::Write as _;

use mzlab_feat::store::read_table;
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_FILLED};

use crate::flow_bentazone_tests::array;
use crate::flow_run_tests::{flow_run, folder, p};

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"a.mzML","type":"raw"},{"name":"b.mzML","type":"raw"},{"name":"c.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:a.mzML","file:b.mzML","file:c.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}},
{"id":"ali","step":"features.align@1","in":{"peaks":["det.peaks"]}},
{"id":"gap","step":"features.gapfill@1","in":{"table":"ali.table","index":["idx.index"]}}]}"#;

/// m/z of the drifting ion, of the ion that file c shows as a ramp and of the ion that file c shows as a small peak under the minimum height.
const DRIFT: f64 = 300.1;
const RAMP: f64 = 420.3;
const SMALL: f64 = 380.2;
const SCANS: usize = 300;

fn gauss(k: f64, apex: f64, h: f64, sigma: f64) -> f64 {
    h * (-0.5 * ((k - apex) / sigma).powi(2)).exp()
}

/// One centroid MS1 run of file `f` (0, 1, 2), 0.02 min a scan: the drifting ion with its apex at scan 100 + 8 f (2.00, 2.16, 2.32 min) and the ion at
/// 3 min, a peak in files a and b, a ramp from 5e3 to 2e4 across 2.7-3.3 min in file c; the ion at 4 min, 5e5 high in files a and b, 1.2e4 in file c;
/// two flat traces at 1e3 keep every scan non-empty.
fn mzml(f: usize) -> String {
    let mut x = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="{SCANS}">"#
    );
    for i in 0..SCANS {
        let k = i as f64;
        let mut pts: Vec<(f64, f64)> = vec![(150.05, 1e3), (500.05, 1e3)];
        let drift = gauss(k, 100.0 + 8.0 * f as f64, 1e6, 3.0);
        let ramp = if f < 2 {
            gauss(k, 150.0, 5e5, 4.0)
        } else if (135..=165).contains(&i) {
            5e3 + 5e2 * (k - 135.0)
        } else {
            0.0
        };
        let small = gauss(k, 200.0, if f < 2 { 5e5 } else { 1.2e4 }, 4.0);
        for (m, h) in [(DRIFT, drift), (RAMP, ramp), (SMALL, small)] {
            if h >= 1e3 {
                pts.push((m + 1e-5 * (i % 3) as f64, h));
            }
        }
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mz, y): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
        let _ = write!(
            x,
            r#"<spectrum index="{i}" id="scan={}" defaultArrayLength="{}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="scan"/><cvParam cvRef="MS" accession="MS:1000512" name="filter string" value="FTMS + p ESI Full ms [100.00-600.00]"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
            i + 1,
            mz.len(),
            k * 0.02,
            array("MS:1000514", &mz),
            array("MS:1000515", &y)
        );
    }
    x + "</spectrumList></run></mzML>"
}

/// The files of the run folder of the plan with the detection of the synthetic files and `set`.
fn run(tag: &str, set: &[&str]) -> Vec<(String, Vec<u8>)> {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_join_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    for (f, name) in ["a", "b", "c"].iter().enumerate() {
        std::fs::write(dir.join(format!("data/{name}.mzML")), mzml(f)).unwrap();
    }
    std::fs::write(dir.join("plan.json"), PLAN).unwrap();
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let mut args = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
    for s in ["det.floor=1000", "det.min_height=3e4", "gap.floor=1000"]
        .iter()
        .chain(set)
    {
        args.extend(["--set", s]);
    }
    let o = flow_run(&args);
    assert_eq!(o.code, 0, "{set:?}: {}", o.stderr);
    let files = folder(&out);
    let _ = std::fs::remove_dir_all(&dir);
    files
}

fn table(files: &[(String, Vec<u8>)], name: &str) -> (Aligned, String) {
    let f = files.iter().find(|f| f.0 == name).unwrap_or_else(|| {
        panic!(
            "{name}: {:?}",
            files.iter().map(|f| &f.0).collect::<Vec<_>>()
        )
    });
    let (t, meta) = read_table(&f.1).unwrap();
    (t, format!("{meta:?}"))
}

/// The rows within 10 ppm of `mz`.
fn rows(t: &Aligned, mz: f64) -> Vec<usize> {
    (0..t.mz.len())
        .filter(|&i| (t.mz[i] - mz).abs() / mz < 1e-5)
        .collect()
}

#[test]
fn the_join_keeps_the_drifting_ion_in_fewer_rows_than_the_rt_gaps() {
    let (groups, _) = table(&run("groups", &[]), "ali.mzfeat");
    assert_eq!(rows(&groups, DRIFT).len(), 3);
    let (join, _) = table(&run("join", &["ali.algorithm=join"]), "ali.mzfeat");
    let r = rows(&join, DRIFT);
    assert_eq!(r.len(), 2);
    let nf = join.n_files;
    let cells: Vec<usize> = r
        .iter()
        .map(|&i| {
            (0..nf)
                .filter(|&k| join.state[i * nf + k] != CELL_ABSENT)
                .count()
        })
        .collect();
    assert_eq!(cells, [2, 1]);
}

/// The state and the height of the cell of file c in the row of `mz`.
fn cell_c(t: &Aligned, mz: f64) -> (u8, f32) {
    let r = rows(t, mz);
    assert_eq!(r.len(), 1, "{mz}");
    (
        t.state[r[0] * t.n_files + 2],
        t.height[r[0] * t.n_files + 2],
    )
}

#[test]
fn the_peak_finder_fills_the_small_peak_and_leaves_empty_the_ramp_that_the_integration_fills() {
    let (integrate, _) = table(&run("integrate", &[]), "gap.mzfeat");
    assert_eq!(cell_c(&integrate, RAMP).0, CELL_FILLED);
    assert_eq!(cell_c(&integrate, SMALL).0, CELL_FILLED);
    let (finder, _) = table(&run("finder", &["gap.algorithm=peak_finder"]), "gap.mzfeat");
    assert_eq!(cell_c(&finder, RAMP).0, CELL_ABSENT);
    let (state, height) = cell_c(&finder, SMALL);
    assert_eq!(state, CELL_FILLED);
    // the peak finder writes the apex it found; the integration keeps an area only
    assert!((height - 1.2e4).abs() < 1e3, "{height}");
}

/// Every parameter of the two algorithms reaches the run: the plan as run carries the value set, and the recipe text in the meta of the table changes.
#[test]
fn every_parameter_of_the_two_algorithms_can_be_set_from_the_command_line() {
    let algos = ["ali.algorithm=join", "gap.algorithm=peak_finder"];
    let base = table(&run("base", &algos), "gap.mzfeat").1;
    for (set, written) in [
        ("ali.mz_weight=2", "\"mz_weight\":2"),
        ("ali.rt_weight=2", "\"rt_weight\":2"),
        ("ali.rt_tol=0.3", "\"rt_tol\":0.3"),
        ("ali.abs_da=0.003", "\"abs_da\":0.003"),
        ("gap.intensity_tol=0.3", "\"intensity_tol\":0.3"),
        ("gap.abs_da=0.003", "\"abs_da\":0.003"),
    ] {
        let mut all = algos.to_vec();
        all.push(set);
        let files = run("set", &all);
        let plan = files.iter().find(|f| f.0 == "plan.json").unwrap();
        let plan = String::from_utf8(plan.1.clone()).unwrap();
        assert!(plan.contains(written), "{set}: {plan}");
        assert_ne!(
            table(&files, "gap.mzfeat").1,
            base,
            "{set} does not reach the recipe"
        );
    }
}
