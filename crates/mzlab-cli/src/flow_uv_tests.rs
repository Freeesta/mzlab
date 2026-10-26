//! `chrom.uv_peaks` on the example file `FullScan_t10.mzML` (it carries a synthetic UV matrix): the CSV, the same narrow rows with and without the
//! broad bands, and `err.pda.none` for a file without a matrix.

use std::path::{Path, PathBuf};

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};

const HEADER: &str = "file,kind,rt_apex_min,rt_lo_min,rt_hi_min,fwhm_s,height_mau,area_mau_s,snr";

fn example() -> Vec<u8> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi/FullScan_t10.mzML");
    std::fs::read(p).expect("example file")
}

/// A one-scan mzML with no optical matrix.
fn no_matrix() -> Vec<u8> {
    let mut w = MzmlWriter::new(Vec::new(), &Header::default()).unwrap();
    w.open_spectra(1).unwrap();
    w.spectrum(&Spectrum {
        id: "scan=1".into(),
        level: 1,
        rt: 1.0,
        polarity: 1,
        mz: vec![100.0],
        intensity: vec![10.0],
        ..Spectrum::default()
    })
    .unwrap();
    w.finish().unwrap()
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_uv_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(d.join("work")).unwrap();
    d
}

fn run(dir: &Path, params: &str) -> Result<String, String> {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"run.mzML","type":"mzml"}}],"steps":[
        {{"id":"uv","step":"chrom.uv_peaks@1","in":{{"source":["file:run.mzML"]}},"params":{params}}}]}}"#
    );
    let (run, prov) = crate::flow::run_plan(plan.as_bytes(), dir, &dir.join("work"), 1, None)
        .map_err(|e| e.key.to_string())?;
    if !run.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(std::fs::read_to_string(dir.join("work/uv.csv")).unwrap())
}

#[test]
fn the_example_gives_a_csv_of_peaks() {
    let d = dir("csv");
    std::fs::write(d.join("run.mzML"), example()).unwrap();
    let csv = run(&d, "{}").unwrap();
    let mut lines = csv.lines();
    assert_eq!(lines.next(), Some(HEADER));
    let rows: Vec<&str> = lines.collect();
    assert!(!rows.is_empty(), "{csv}");
    assert!(
        rows.iter()
            .all(|r| r.starts_with("run.mzML,narrow,") && r.split(',').count() == 9),
        "{csv}"
    );
    assert_eq!(run(&d, "{}").unwrap(), csv);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn narrow_rows_do_not_change_with_the_broad_bands() {
    let d = dir("broad");
    std::fs::write(d.join("run.mzML"), example()).unwrap();
    let narrow = |csv: &str| -> Vec<String> {
        csv.lines()
            .filter(|l| l.contains(",narrow,"))
            .map(String::from)
            .collect()
    };
    let (off, on) = (
        run(&d, r#"{"broad":false}"#).unwrap(),
        run(
            &d,
            r#"{"broad":true,"broad_min_s":20,"broad_max_s":200,"broad_snr":6}"#,
        )
        .unwrap(),
    );
    assert_eq!(narrow(&off), narrow(&on));
    assert!(on
        .lines()
        .all(|l| l == HEADER || l.contains(",narrow,") || l.contains(",broad,")));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_file_without_a_matrix_is_refused() {
    let d = dir("none");
    std::fs::write(d.join("run.mzML"), no_matrix()).unwrap();
    let prov = run(&d, "{}").unwrap_err();
    assert!(prov.contains("err.pda.none"), "{prov}");
    let _ = std::fs::remove_dir_all(&d);
}
