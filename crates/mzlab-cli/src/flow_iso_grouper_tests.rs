//! The isotope grouping `grouper` of `features.detect` (the isotope grouper of mzmine) run from a Flow plan by `mzlab-cli flow run`, on one synthetic
//! high-resolution file: an ion whose M+1 is stronger than natural abundance allows, which the averagine grouping leaves alone and the grouper
//! groups (it tests no ratio), with the representative after `iso_representative`; `--set` reaches every parameter.

use std::fmt::Write as _;

use mzlab_feat::store::read_file;
use mzlab_feat::table::{FileFeatures, FLAG_ISOTOPE};

use crate::flow_bentazone_tests::array;
use crate::flow_run_tests::{flow_run, folder, p};

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"a.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:a.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}}]}"#;

/// m/z of the ion and of its M+1 (one 13C), whose height is 1.5 times the one of the ion: no averagine of 300 Da has it.
const MONO: f64 = 300.1;
const HEAVY: f64 = MONO + 1.003355;
const SCANS: usize = 200;

fn gauss(k: f64, apex: f64, h: f64, sigma: f64) -> f64 {
    h * (-0.5 * ((k - apex) / sigma).powi(2)).exp()
}

/// One centroid MS1 run, 0.02 min a scan: the ion (1e6) and its M+1 (1.5e6) with the apex at scan 100; two flat traces at 1e3 keep every scan
/// non-empty.
fn mzml() -> String {
    let mut x = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="{SCANS}">"#
    );
    for i in 0..SCANS {
        let k = i as f64;
        let mut pts: Vec<(f64, f64)> = vec![(150.05, 1e3), (500.05, 1e3)];
        for (m, h) in [(MONO, 1e6), (HEAVY, 1.5e6)] {
            let y = gauss(k, 100.0, h, 3.0);
            if y >= 1e3 {
                pts.push((m, y));
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

/// The files of the run folder of the plan with `set`.
fn run(tag: &str, set: &[&str]) -> Vec<(String, Vec<u8>)> {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_isogrp_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data/a.mzML"), mzml()).unwrap();
    std::fs::write(dir.join("plan.json"), PLAN).unwrap();
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let mut args = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
    for s in ["det.floor=1000", "det.min_height=3e4"].iter().chain(set) {
        args.extend(["--set", s]);
    }
    let o = flow_run(&args);
    assert_eq!(o.code, 0, "{set:?}: {}", o.stderr);
    let files = folder(&out);
    let _ = std::fs::remove_dir_all(&dir);
    files
}

/// The features of the file and the text of their meta.
fn features(files: &[(String, Vec<u8>)]) -> (FileFeatures, String) {
    let found = files.iter().find_map(|f| read_file(&f.1).ok());
    let (f, meta) =
        found.unwrap_or_else(|| panic!("{:?}", files.iter().map(|f| &f.0).collect::<Vec<_>>()));
    (f, format!("{meta:?}"))
}

/// `(iso_group, isotopologue flag)` of the feature at `mz`.
fn iso(f: &FileFeatures, mz: f64) -> (u32, bool) {
    let i = (0..f.mz.len())
        .find(|&i| (f.mz[i] - mz).abs() < 0.002)
        .unwrap_or_else(|| panic!("{mz}: {:?}", f.mz));
    (f.iso_group[i], f.flags[i] & FLAG_ISOTOPE != 0)
}

#[test]
fn the_grouper_groups_an_m1_that_the_averagine_ratio_rejects() {
    let (avg, _) = features(&run("avg", &[]));
    assert_eq!(iso(&avg, MONO), (0, false));
    assert_eq!(iso(&avg, HEAVY), (0, false));
    let (grp, _) = features(&run("grp", &["det.isotopes=grouper"]));
    let (g, heavy_flag) = iso(&grp, HEAVY);
    let (g_mono, mono_flag) = iso(&grp, MONO);
    assert!(g != 0 && g == g_mono, "{g} {g_mono}");
    // the most intense row (the M+1) is the representative; the ion is marked
    assert!(!heavy_flag && mono_flag);
    assert_eq!(grp.charge[(g - 1) as usize], 1);
    let low = &["det.isotopes=grouper", "det.iso_representative=lowest_mz"];
    let (low, _) = features(&run("low", low));
    assert!(!iso(&low, MONO).1 && iso(&low, HEAVY).1);
}

/// Every parameter of the grouper reaches the run: the plan as run carries the value set, and the recipe text in the meta of the features changes.
#[test]
fn every_parameter_of_the_grouper_can_be_set_from_the_command_line() {
    let on = "det.isotopes=grouper";
    let base = features(&run("base", &[on])).1;
    for (set, written) in [
        ("det.iso_ppm=8", "\"iso_ppm\":8"),
        ("det.iso_abs_da=0.003", "\"iso_abs_da\":0.003"),
        ("det.iso_rt_tol=0.1", "\"iso_rt_tol\":0.1"),
        ("det.iso_max_charge=3", "\"iso_max_charge\":3"),
        ("det.iso_monotonic=true", "\"iso_monotonic\":true"),
        (
            "det.iso_representative=lowest_mz",
            "\"iso_representative\":\"lowest_mz\"",
        ),
    ] {
        let files = run("set", &[on, set]);
        let plan = files.iter().find(|f| f.0 == "plan.json").unwrap();
        let plan = String::from_utf8(plan.1.clone()).unwrap();
        assert!(plan.contains(written), "{set}: {plan}");
        assert_ne!(features(&files).1, base, "{set} does not reach the recipe");
    }
}

#[test]
fn a_parameter_of_the_grouper_without_the_grouper_changes_nothing() {
    let base = features(&run("avg_base", &[]));
    let set = features(&run("avg_set", &["det.iso_ppm=8", "det.iso_max_charge=3"]));
    assert_eq!(set, base);
}
