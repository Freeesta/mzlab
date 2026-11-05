//! Prototype: area of `features.detect` against the analytic area of synthetic Gaussian peaks.

use std::fmt::Write as _;

use mzlab_feat::store::read_file;

use crate::flow_bentazone_tests::array;
use crate::flow_run_tests::{flow_run, folder, p};

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"a.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:a.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}}]}"#;

const SCANS: usize = 400;
const DT_MIN: f64 = 0.01;
const SIGMA: f64 = 4.0;

fn ions() -> Vec<(f64, f64, f64)> {
    let h = [3e5, 6e5, 1e6, 3e6, 1e7, 3e7];
    (0..h.len())
        .map(|k| (150.0 + 61.3 * k as f64, h[k], 60.0 + 50.0 * k as f64))
        .collect()
}

fn rnd(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

fn y(k: f64, apex: f64, h: f64) -> f64 {
    h * (-0.5 * ((k - apex) / SIGMA).powi(2)).exp()
}

fn mzml(jitter_ppm: f64) -> String {
    let mut seed = 7u64;
    let mut x = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="{SCANS}">"#
    );
    for i in 0..SCANS {
        let k = i as f64;
        let mut pts: Vec<(f64, f64)> = vec![(120.05, 1e3), (590.05, 1e3)];
        for (m, h, a) in ions() {
            let v = y(k, a, h);
            let e = (rnd(&mut seed) * 2.0 - 1.0) * jitter_ppm * 1e-6 * m;
            if v >= 50.0 {
                pts.push((m + e, v));
            }
        }
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (mz, yy): (Vec<f64>, Vec<f64>) = pts.into_iter().unzip();
        let _ = write!(
            x,
            r#"<spectrum index="{i}" id="scan={}" defaultArrayLength="{}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="scan"/><cvParam cvRef="MS" accession="MS:1000512" name="filter string" value="FTMS + p ESI Full ms [100.00-600.00]"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
            i + 1,
            mz.len(),
            k * DT_MIN,
            array("MS:1000514", &mz),
            array("MS:1000515", &yy)
        );
    }
    x + "</spectrumList></run></mzML>"
}

fn run(tag: &str, jitter: f64, set: &[&str]) {
    let dir = std::env::temp_dir().join(format!("mzlab_area_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data/a.mzML"), mzml(jitter)).unwrap();
    std::fs::write(dir.join("plan.json"), PLAN).unwrap();
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let mut args = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
    for s in set {
        args.extend(["--set", s]);
    }
    let o = flow_run(&args);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let files = folder(&out);
    let (f, _) = files.iter().find_map(|f| read_file(&f.1).ok()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    for (m, h, a) in ions() {
        let truth = h * SIGMA * (2.0 * std::f64::consts::PI).sqrt() * DT_MIN * 60.0;
        let rows: Vec<usize> = (0..f.mz.len())
            .filter(|&i| (f.mz[i] - m).abs() < 0.01)
            .collect();
        let mut line = format!("{tag:>14} H={h:>8.0e} ");
        for i in rows {
            let (lo, hi) = (f.scan_lo[i] as usize, f.scan_hi[i] as usize);
            let trap: f64 = (lo + 1..=hi)
                .map(|k| 0.5 * (y(k as f64, a, h) + y(k as f64 - 1.0, a, h)) * DT_MIN * 60.0)
                .sum();
            let _ = write!(
                line,
                "| log2(area/true) {:+.4} log2(area/trap[lo,hi]) {:+.4} lo-apex {:+} hi-apex {:+} ",
                (f.area[i] / truth).log2(),
                (f.area[i] / trap).log2(),
                lo as f64 - a,
                hi as f64 - a
            );
        }
        eprintln!("{line}");
    }
}

#[test]
#[ignore]
fn prototype_area_bias() {
    for j in [0.5, 1.0, 1.5, 2.0, 3.0] {
        run(&format!("mt_j{j}"), j, &[]);
    }
}
