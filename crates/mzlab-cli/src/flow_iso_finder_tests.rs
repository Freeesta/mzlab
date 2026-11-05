//! The isotope grouping `finder` of `features.detect` (the isotope finder of mzmine 4.10) run from a Flow plan by `mzlab-cli flow run`, on one
//! synthetic high-resolution file with three ions at their own RT: an M+1 at 3 % (no plausible 13C peak: the averagine grouping takes it, the
//! finder does not), an M+1 12 ppm off its step (outside the 5 ppm of averagine, inside the 15 ppm of the finder) and a charge-3 envelope
//! that both find; `--set` reaches every parameter.

use std::fmt::Write as _;

use mzlab_feat::store::read_file;
use mzlab_feat::table::{FileFeatures, FLAG_ISOTOPE};

use crate::flow_bentazone_tests::array;
use crate::flow_run_tests::{flow_run, folder, p};

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"a.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:a.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}}]}"#;

const SCANS: usize = 200;
const C13: f64 = 1.003355;
/// The ion with an M+1 at 3 %, apex at scan 40.
const LOW: f64 = 250.1;
/// The ion with an M+1 4.2 mDa (12 ppm) above its step, apex at scan 160.
const OFF: f64 = 350.1;
const OFF_M1: f64 = OFF + C13 + 0.0042;
/// A charge-3 envelope of about 70 carbons, apex at scan 100.
const Z3: f64 = 501.0;
/// `(m/z, height, apex scan)`.
const IONS: [(f64, f64, f64); 8] = [
    (LOW, 1e6, 40.0),
    (LOW + C13, 3e4, 40.0),
    (OFF, 1e6, 160.0),
    (OFF_M1, 1.8e5, 160.0),
    (Z3, 1e6, 100.0),
    (Z3 + C13 / 3.0, 7.7e5, 100.0),
    (Z3 + 2.0 * C13 / 3.0, 2.9e5, 100.0),
    (Z3 + 3.0 * C13 / 3.0, 0.9e5, 100.0),
];

fn gauss(k: f64, apex: f64, h: f64, sigma: f64) -> f64 {
    h * (-0.5 * ((k - apex) / sigma).powi(2)).exp()
}

/// One centroid MS1 run, 0.02 min a scan; two flat traces at 1e3 keep every scan non-empty.
fn mzml() -> String {
    let mut x = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="{SCANS}">"#
    );
    for i in 0..SCANS {
        let k = i as f64;
        let mut pts: Vec<(f64, f64)> = vec![(150.05, 1e3), (500.05, 1e3)];
        for (m, h, a) in IONS {
            let y = gauss(k, a, h, 3.0);
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
    let dir = std::env::temp_dir().join(format!("mzlab_cli_isofind_{tag}_{}", std::process::id()));
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

/// `(iso_group, isotopologue flag, charge)` of the feature at `mz`.
fn iso(f: &FileFeatures, mz: f64) -> (u32, bool, i8) {
    let i = (0..f.mz.len())
        .find(|&i| (f.mz[i] - mz).abs() < 0.002)
        .unwrap_or_else(|| panic!("{mz}: {:?}", f.mz));
    (f.iso_group[i], f.flags[i] & FLAG_ISOTOPE != 0, f.charge[i])
}

/// Whether the feature at `mz` is an isotopologue in the group of the feature at `parent`, with charge `z`.
fn joined(f: &FileFeatures, parent: f64, mz: f64, z: i8) -> bool {
    let (g, _, zp) = iso(f, parent);
    let (gm, flag, zm) = iso(f, mz);
    g != 0 && g == gm && flag && zp == z && zm == z
}

#[test]
fn the_finder_reads_the_apex_spectrum_where_the_averagine_grouping_reads_the_rows() {
    let (avg, _) = features(&run("avg", &[]));
    assert!(joined(&avg, LOW, LOW + C13, 1));
    assert_eq!(iso(&avg, OFF_M1), (0, false, 0));
    assert!(joined(&avg, Z3, Z3 + C13 / 3.0, 3));
    let (fin, _) = features(&run("fin", &["det.isotopes=finder"]));
    // no plausible 13C peak at 3 %: the finder leaves the ion alone
    assert_eq!(iso(&fin, LOW + C13), (0, false, 0));
    assert!(joined(&fin, OFF, OFF_M1, 1));
    assert!(joined(&fin, Z3, Z3 + C13 / 3.0, 3) && joined(&fin, Z3, Z3 + 2.0 * C13 / 3.0, 3));
    // a tolerance below the offset loses the M+1 that is 12 ppm off
    let tight = &[
        "det.isotopes=finder",
        "det.finder_ppm=2",
        "det.finder_abs_da=0.0005",
    ];
    let (tight, _) = features(&run("tight", tight));
    assert_eq!(iso(&tight, OFF_M1), (0, false, 0));
}

/// Every parameter of the finder reaches the run: the plan as run carries the value set, and the recipe text in the meta of the features changes.
#[test]
fn every_parameter_of_the_finder_can_be_set_from_the_command_line() {
    let on = "det.isotopes=finder";
    let base = features(&run("base", &[on])).1;
    for (set, written) in [
        ("det.finder_ppm=8", "\"finder_ppm\":8"),
        ("det.finder_abs_da=0.003", "\"finder_abs_da\":0.003"),
        ("det.finder_max_charge=2", "\"finder_max_charge\":2"),
        (
            "det.finder_elements=C,H,N,O",
            "\"finder_elements\":\"C,H,N,O\"",
        ),
        (
            "det.finder_require_c13=false",
            "\"finder_require_c13\":false",
        ),
        ("det.finder_auto=false", "\"finder_auto\":false"),
    ] {
        let files = run("set", &[on, set]);
        let plan = files.iter().find(|f| f.0 == "plan.json").unwrap();
        let plan = String::from_utf8(plan.1.clone()).unwrap();
        assert!(plan.contains(written), "{set}: {plan}");
        assert_ne!(features(&files).1, base, "{set} does not reach the recipe");
    }
}

#[test]
fn a_parameter_of_the_finder_without_the_finder_changes_nothing() {
    let base = features(&run("avg_base", &[]));
    let set = features(&run(
        "avg_set",
        &["det.finder_ppm=8", "det.finder_auto=false"],
    ));
    assert_eq!(set, base);
}
