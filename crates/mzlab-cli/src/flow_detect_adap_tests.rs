//! The algorithm `adap_localmin` of `features.detect` run from a Flow plan by `mzlab-cli flow run`, on one synthetic high-resolution file: a peak with a
//! shoulder that the local-minimum resolver splits, a jagged peak that the Savitzky-Golay smoothing keeps whole, faint points that the factor of lowest
//! signal drops; the default `masstrace` and its bytes do not change, and `--set det.NAME=VALUE` reaches every parameter of the algorithm.

use std::fmt::Write as _;

use mzlab_feat::store::read_file;
use mzlab_feat::table::FileFeatures;

use crate::flow_bentazone_tests::array;
use crate::flow_run_tests::{flow_run, folder, p};

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"a.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:a.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}}]}"#;

/// m/z of the peak with a shoulder, of the jagged peak and of the faint ion.
const SHOULDER: f64 = 300.1;
const JAGGED: f64 = 350.2;
const FAINT: f64 = 420.3;
const SCANS: usize = 300;

fn gauss(k: f64, apex: f64, h: f64, sigma: f64) -> f64 {
    h * (-0.5 * ((k - apex) / sigma).powi(2)).exp()
}

/// One high-resolution centroid MS1 run (0.02 min a scan): the shoulder (apexes at scans 100 and 114, the valley 0.55 of the lower maximum: above the
/// 0.5 that `resolve.rs` wants, under the 1/1.7 of the resolver), the jagged peak (apex 200, every other scan halved) and the faint ion (apex 2.5 times
/// the lowest signal of a scan) beside two flat traces at 1e3.
fn mzml() -> String {
    let mut x = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="{SCANS}">"#
    );
    for i in 0..SCANS {
        let k = i as f64;
        let mut pts: Vec<(f64, f64)> = vec![(150.05, 1e3), (500.05, 1e3)];
        let sh = gauss(k, 100.0, 1e6, 4.0) + gauss(k, 114.0, 6e5, 4.0);
        let jag = gauss(k, 200.0, 8e5, 10.0) * if i % 2 == 0 { 1.0 } else { 0.5 };
        let faint = gauss(k, 150.0, 2.5e3, 5.0);
        for (m, h) in [(SHOULDER, sh), (JAGGED, jag), (FAINT, faint)] {
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

/// The files of the run folder of the plan with `set`.
fn run(tag: &str, set: &[&str]) -> Vec<(String, Vec<u8>)> {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_adap_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data/a.mzML"), mzml()).unwrap();
    std::fs::write(dir.join("plan.json"), PLAN).unwrap();
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let mut args = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
    for s in set {
        args.extend(["--set", s]);
    }
    let o = flow_run(&args);
    assert_eq!(o.code, 0, "{set:?}: {}", o.stderr);
    let files = folder(&out);
    let _ = std::fs::remove_dir_all(&dir);
    files
}

fn features(files: &[(String, Vec<u8>)]) -> FileFeatures {
    let f = files
        .iter()
        .find(|f| f.0 == "det-a.mzfeatf")
        .expect("det-a.mzfeatf");
    read_file(&f.1).unwrap().0
}

/// The apex RTs (min) of the rows within 10 ppm of `mz`.
fn apexes(f: &FileFeatures, mz: f64) -> Vec<f32> {
    (0..f.mz.len())
        .filter(|&i| (f.mz[i] - mz).abs() / mz < 1e-5)
        .map(|i| f.rt[i])
        .collect()
}

const ADAP: [&str; 3] = [
    "det.algorithm=adap_localmin",
    "det.floor=1000",
    "det.min_height=3e4",
];

/// The rows of the plan run with `set` on top of `adap_localmin` with the floor and height of the synthetic file.
fn adap(tag: &str, set: &[&str]) -> FileFeatures {
    let mut all = ADAP.to_vec();
    all.extend_from_slice(set);
    features(&run(tag, &all))
}

#[test]
fn the_local_minimum_resolver_splits_the_shoulder_that_masstrace_keeps_as_one() {
    let mt = features(&run("mt", &ADAP[1..]));
    assert_eq!(apexes(&mt, SHOULDER), [2.0]);
    assert_eq!(apexes(&adap("ad", &[]), SHOULDER), [2.0, 2.28]);
}

#[test]
fn the_smoothing_keeps_the_jagged_peak_whole_where_the_raw_trace_is_cut() {
    // a search range under one scan lets every dip of the raw trace close a region
    let raw = adap("raw", &["det.search_rt=0.01"]);
    assert_eq!(apexes(&raw, JAGGED), [3.68]);
    let sg = adap("sg", &["det.search_rt=0.01", "det.smooth_window=5"]);
    assert_eq!(apexes(&sg, JAGGED), [4.0]);
}

#[test]
fn the_factor_of_lowest_signal_drops_the_faint_ion_that_an_absolute_noise_keeps() {
    let low = ["det.floor=1", "det.min_height=2000"];
    assert_eq!(apexes(&adap("abs", &low), FAINT), [3.0]);
    let f = adap("factor", &[low[0], low[1], "det.noise_factor=3"]);
    assert!(apexes(&f, FAINT).is_empty());
    assert_eq!(apexes(&f, SHOULDER), [2.0, 2.28]);
}

/// Every parameter of the algorithm reaches the run: the plan as run carries the value set, and the recipe text (the name of the feature file) changes.
#[test]
fn every_parameter_of_the_algorithm_can_be_set_from_the_command_line() {
    let base = run("base", &ADAP);
    let name = |files: &[(String, Vec<u8>)]| {
        let plan = files.iter().find(|f| f.0 == "plan.json").unwrap();
        String::from_utf8(plan.1.clone()).unwrap()
    };
    for (set, written) in [
        ("det.noise_factor=2.5", "\"noise_factor\":2.5"),
        ("det.ppm=8", "\"ppm\":8"),
        ("det.abs_da=0.004", "\"abs_da\":0.004"),
        ("det.smooth_window=7", "\"smooth_window\":7"),
        ("det.chrom_threshold=0.5", "\"chrom_threshold\":0.5"),
        ("det.search_rt=0.05", "\"search_rt\":0.05"),
        ("det.min_ratio=2", "\"min_ratio\":2"),
        ("det.duration=0,2", "\"duration\":[0,2]"),
    ] {
        let mut all = ADAP.to_vec();
        all.push(set);
        let files = run("set", &all);
        assert!(name(&files).contains(written), "{set}: {}", name(&files));
        assert_ne!(
            features_meta(&files),
            features_meta(&base),
            "{set} does not reach the recipe"
        );
    }
}

/// The recipe text in the meta of the feature file.
fn features_meta(files: &[(String, Vec<u8>)]) -> String {
    let f = files.iter().find(|f| f.0 == "det-a.mzfeatf").unwrap();
    format!("{:?}", read_file(&f.1).unwrap().1)
}
