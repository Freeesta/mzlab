//! `area_from` of `features.detect` run from a Flow plan by `mzlab-cli flow run`, on one synthetic high-resolution file: six Gaussian ions
//! (3e5 to 3e7 counts) whose m/z wanders ±2 ppm from scan to scan. With `channel` (the default) the points that fall into the next channel
//! of `masstrace` are lost from the area; with `window` the area is the one of the Gaussian within a few hundredths of log2. At ±3 ppm the
//! `strict` stitch splits ions into pieces; `stitch=wide` keeps one row per ion. When MS2 scans space the MS1 scans unevenly, only `xic`
//! (the trapezoid on the real retention times) keeps the area: `channel` and `window` multiply by the median scan interval.

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

/// Regular scans, `DT` apart.
fn regular() -> Vec<f64> {
    (0..SCANS).map(|i| i as f64 * DT).collect()
}

/// Scans `DT` apart, 2.5 `DT` apart while an ion is within 1.5 sigma of its apex (the MS2 scans a DDA cycle puts between them).
fn uneven() -> Vec<f64> {
    let mut t = vec![0.0];
    while t.len() < SCANS {
        let now = *t.last().unwrap();
        let busy = ions()
            .iter()
            .any(|&(_, _, a)| (now / DT - a).abs() < 1.5 * SIGMA);
        t.push(now + if busy { 2.5 * DT } else { DT });
    }
    t
}

/// One centroid MS1 run at `times` (minutes); the ions are Gaussians in time. Points below 50 counts are not written, two flat traces at 1e3
/// keep every scan non-empty.
fn mzml(jitter_ppm: f64, times: &[f64]) -> String {
    let mut seed = 7u64;
    let mut x = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="{SCANS}">"#
    );
    for (i, &t) in times.iter().enumerate() {
        let k = t / DT;
        let mut pts: Vec<(f64, f64)> = vec![(120.05, 1e3), (590.05, 1e3)];
        for (m, h, a) in ions() {
            let y = h * (-0.5 * ((k - a) / SIGMA).powi(2)).exp();
            let e = (rnd(&mut seed) * 2.0 - 1.0) * jitter_ppm * 1e-6 * m;
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
            t,
            array("MS:1000514", &mz),
            array("MS:1000515", &y)
        );
    }
    x + "</spectrumList></run></mzML>"
}

/// The files of the run folder of the plan with `set`, on the ions wandering ±2 ppm.
fn run(tag: &str, set: &[&str]) -> Vec<(String, Vec<u8>)> {
    run_at(tag, set, 2.0)
}

fn run_at(tag: &str, set: &[&str], jitter_ppm: f64) -> Vec<(String, Vec<u8>)> {
    run_on(tag, set, jitter_ppm, &regular())
}

fn run_on(tag: &str, set: &[&str], jitter_ppm: f64, times: &[f64]) -> Vec<(String, Vec<u8>)> {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_area_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data/a.mzML"), mzml(jitter_ppm, times)).unwrap();
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

/// |log2(area / XIC area)| of each ion, from its largest row: the trapezoid of the noise-free Gaussian on the real times of the row's scans
/// (the chromatogram a user integrates), so the tails below the intensity floor do not count.
fn to_xic(f: &FileFeatures, times: &[f64]) -> Vec<f64> {
    ions()
        .iter()
        .map(|&(m, h, a)| {
            let r = (0..f.mz.len())
                .filter(|&i| (f.mz[i] - m).abs() < 0.01)
                .max_by(|&i, &j| f.area[i].total_cmp(&f.area[j]))
                .unwrap();
            let y = |t: f64| h * (-0.5 * ((t / DT - a) / SIGMA).powi(2)).exp();
            let (lo, hi) = (f.scan_lo[r] as usize, f.scan_hi[r] as usize);
            let xic: f64 = (lo..hi)
                .map(|k| 0.5 * (y(times[k]) + y(times[k + 1])) * (times[k + 1] - times[k]) * 60.0)
                .sum();
            (f.area[r] / xic).log2().abs()
        })
        .collect()
}

/// Rows within 0.01 of the m/z of each ion.
fn rows_per_ion(f: &FileFeatures) -> Vec<usize> {
    ions()
        .iter()
        .map(|&(m, _, _)| f.mz.iter().filter(|&&x| (x - m).abs() < 0.01).count())
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

/// At ±3 ppm the pieces of an ion alternate between two channels and touch without overlapping: `strict` joins touching pieces only
/// above 2e6 counts, `wide` joins them all and keeps the low ones, so each ion is one row with its whole area.
#[test]
fn the_wide_stitch_keeps_one_row_per_ion_at_three_ppm() {
    let window = "det.area_from=window";
    let strict = features(&run_at("strict3", &[window], 3.0)).0;
    let wide = features(&run_at("wide3", &[window, "det.stitch=wide"], 3.0)).0;
    let (n_strict, n_wide) = (rows_per_ion(&strict), rows_per_ion(&wide));
    let (b_strict, b_wide) = (bias(&strict), bias(&wide));
    eprintln!("rows strict {n_strict:?} wide {n_wide:?}\n|log2| strict {b_strict:.4?}\n|log2| wide   {b_wide:.4?}");
    assert!(n_strict.iter().any(|&n| n > 1), "{n_strict:?}");
    assert!(n_wide.iter().all(|&n| n == 1), "{n_wide:?}");
    assert!(b_strict.iter().any(|&b| b > 0.5), "{b_strict:?}");
    assert!(b_wide.iter().all(|&b| b <= 0.05), "{b_wide:?}");
}

/// `stitch` reaches the run like `area_from`; `strict` is the default.
#[test]
fn stitch_can_be_set_from_the_command_line() {
    let base = run("sbase", &[]);
    assert_eq!(
        features(&run("sstrict", &["det.stitch=strict"])),
        features(&base)
    );
    let wide = run("swide", &["det.stitch=wide"]);
    let plan = wide.iter().find(|f| f.0 == "plan.json").unwrap();
    let plan = String::from_utf8(plan.1.clone()).unwrap();
    assert!(plan.contains("\"stitch\":\"wide\""), "{plan}");
    assert_ne!(features(&wide).1, features(&base).1);
}

/// MS1 scans 2.5 times further apart around the apices: the median interval undercounts every peak, the trapezoid on the real times does not.
#[test]
fn the_xic_area_follows_uneven_scans() {
    let t = uneven();
    let area = |tag: &str, from: &str| {
        let set = format!("det.area_from={from}");
        to_xic(&features(&run_on(tag, &[&set], 0.0, &t)).0, &t)
    };
    let (window, xic) = (area("uwin", "window"), area("uxic", "xic"));
    eprintln!("|log2| to the XIC: window {window:.4?}\n|log2| to the XIC: xic    {xic:.4?}");
    assert!(median(&window) > 0.5, "{window:?}");
    assert!(xic.iter().all(|&b| b <= 0.005), "{xic:?}");
}

/// `area_from=xic` reaches the run, also on wandering ions (its window is the one of `window`), and changes the recipe of the features.
#[test]
fn the_xic_area_can_be_set_from_the_command_line() {
    let xic = run("xic", &["det.area_from=xic"]);
    let plan = xic.iter().find(|f| f.0 == "plan.json").unwrap();
    let plan = String::from_utf8(plan.1.clone()).unwrap();
    assert!(plan.contains("\"area_from\":\"xic\""), "{plan}");
    let (f, meta) = features(&xic);
    assert!(meta.contains("area_xic"), "{meta}");
    let b = bias(&f);
    assert!(median(&b) <= 0.025, "{b:?}");
}
