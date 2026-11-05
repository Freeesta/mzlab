//! `area_from` of `features.detect` run from a Flow plan by `mzlab-cli flow run`, on one synthetic high-resolution file: six Gaussian ions
//! (3e5 to 3e7 counts) whose m/z wanders ±2 ppm from scan to scan. With `channel` (the default) the points that fall into the next channel
//! of `masstrace` are lost from the area; with `window` the area is the one of the Gaussian within a few hundredths of log2.

use std::fmt::Write as _;

use mzlab_feat::store::read_file;
use mzlab_feat::table::FileFeatures;

use crate::flow_bentazone_tests::array;
use crate::flow_run_tests::{flow_run, folder, p};

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"a.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:a.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}}]}"#;

const SCANS: usize = 400;
/// Minutes between scans.
const DT: f64 = 0.01;
/// Width of the peaks, scans.
const SIGMA: f64 = 4.0;
const JITTER_PPM: f64 = 2.0;

/// m/z, height and apex scan of the ions.
fn ions() -> Vec<(f64, f64, f64)> {
    [3e5, 6e5, 1e6, 3e6, 1e7, 3e7]
        .iter()
        .enumerate()
        .map(|(k, &h)| (150.0 + 61.3 * k as f64, h, 60.0 + 50.0 * k as f64))
        .collect()
}

fn rnd(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

/// One centroid MS1 run; points below 50 counts are not written, two flat traces at 1e3 keep every scan non-empty.
fn mzml() -> String {
    let mut seed = 7u64;
    let mut x = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="{SCANS}">"#
    );
    for i in 0..SCANS {
        let k = i as f64;
        let mut pts: Vec<(f64, f64)> = vec![(120.05, 1e3), (590.05, 1e3)];
        for (m, h, a) in ions() {
            let y = h * (-0.5 * ((k - a) / SIGMA).powi(2)).exp();
            let e = (rnd(&mut seed) * 2.0 - 1.0) * JITTER_PPM * 1e-6 * m;
            if y >= 50.0 {
                pts.push((m + e, y));
            }
        }
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mz, y): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
        let _ = write!(
            x,
            r#"<spectrum index="{i}" id="scan={}" defaultArrayLength="{}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="scan"/><cvParam cvRef="MS" accession="MS:1000512" name="filter string" value="FTMS + p ESI Full ms [100.00-600.00]"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
            i + 1,
            mz.len(),
            k * DT,
            array("MS:1000514", &mz),
            array("MS:1000515", &y)
        );
    }
    x + "</spectrumList></run></mzML>"
}

/// The files of the run folder of the plan with `set`.
fn run(tag: &str, set: &[&str]) -> Vec<(String, Vec<u8>)> {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_area_{tag}_{}", std::process::id()));
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

fn features(files: &[(String, Vec<u8>)]) -> (FileFeatures, String) {
    let (f, meta) = files.iter().find_map(|f| read_file(&f.1).ok()).unwrap();
    (f, format!("{meta:?}"))
}

/// |log2(area / area of the Gaussian)| of each ion, from its largest row.
fn bias(f: &FileFeatures) -> Vec<f64> {
    ions()
        .iter()
        .map(|&(m, h, _)| {
            let truth = h * SIGMA * (2.0 * std::f64::consts::PI).sqrt() * DT * 60.0;
            let area = (0..f.mz.len())
                .filter(|&i| (f.mz[i] - m).abs() < 0.01)
                .map(|i| f.area[i])
                .fold(0.0, f64::max);
            (area / truth).log2().abs()
        })
        .collect()
}

fn median(v: &[f64]) -> f64 {
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    0.5 * (v[(v.len() - 1) / 2] + v[v.len() / 2])
}

#[test]
fn the_window_area_removes_the_bias_of_a_wandering_mass() {
    let channel = bias(&features(&run("channel", &[])).0);
    let window = bias(&features(&run("window", &["det.area_from=window"])).0);
    eprintln!("|log2| channel {channel:.4?}\n|log2| window  {window:.4?}");
    assert!(median(&channel) > 0.1, "{channel:?}");
    assert!(median(&window) <= 0.025, "{window:?}");
    assert!(window.iter().all(|&b| b <= 0.03), "{window:?}");
}

/// `area_from` reaches the run: the plan as run carries it and the recipe text in the meta of the features changes; `channel` is the default.
#[test]
fn area_from_can_be_set_from_the_command_line() {
    let base = run("base", &[]);
    let channel = run("chan", &["det.area_from=channel"]);
    assert_eq!(features(&channel), features(&base));
    let window = run("win", &["det.area_from=window"]);
    let plan = window.iter().find(|f| f.0 == "plan.json").unwrap();
    let plan = String::from_utf8(plan.1.clone()).unwrap();
    assert!(plan.contains("\"area_from\":\"window\""), "{plan}");
    assert_ne!(features(&window).1, features(&base).1);
}
