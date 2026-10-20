//! `spectra.ms2` on a synthetic DDA mzML made here: two MS2 scans (one with a wide isolation window), an MS1 scan and a peak under the threshold, then
//! the plan mzML → `spectra.ms2` → `spectra.match` on the library of `flow_ident_tests::fixture`.

use std::path::Path;

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};

use crate::flow_ident_tests::fixture;

fn scan(
    index: usize,
    level: u8,
    rt: f64,
    precursor: Option<f64>,
    peaks: &[(f64, f32)],
) -> Spectrum {
    Spectrum {
        index,
        id: format!("scan={}", index + 1),
        level,
        rt,
        polarity: 1,
        precursor,
        mz: peaks.iter().map(|p| p.0).collect(),
        intensity: peaks.iter().map(|p| p.1).collect(),
        ..Spectrum::default()
    }
}

/// MS1, MS2 of Alpha (window 1 Da, charge 1), MS2 of another ion (window 10 Da, charge 2); each MS2 precursor has an intensity.
fn dda_mzml() -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "dda".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(3).unwrap();
    w.spectrum(&scan(0, 1, 1.0, None, &[(300.1235, 5e5)]))
        .unwrap();
    let alpha = [
        (100.051, 380.0),
        (120.079, 950.0),
        (150.102, 720.0),
        (180.121, 310.0),
        (210.149, 480.0),
        (130.0, 5.0),
    ];
    w.spectrum(&scan(1, 2, 1.1, Some(300.1235), &alpha))
        .unwrap();
    w.spectrum(&scan(
        2,
        2,
        1.2,
        Some(400.1234),
        &[(250.1, 900.0), (280.1, 700.0)],
    ))
    .unwrap();
    w.close_spectra().unwrap();
    let text = String::from_utf8(w.finish().unwrap()).unwrap();
    let cv = |acc: &str, name: &str, v: f64| {
        format!("<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"{v}\"/>")
    };
    // the writer does not say charge, precursor intensity and window offsets: add them by hand
    let mut parts = text
        .split("</isolationWindow>")
        .map(str::to_string)
        .collect::<Vec<_>>();
    let tail = parts.pop().unwrap();
    let mut out = String::new();
    for (k, p) in parts.iter().enumerate() {
        let half = if k == 0 { 0.5 } else { 5.0 };
        out += p;
        out += &(cv("MS:1000828", "isolation window lower offset", half)
            + &cv("MS:1000829", "isolation window upper offset", half));
        out += "</isolationWindow>";
    }
    out += &tail;
    let mut n = 0;
    out.split("</selectedIon>")
        .map(|p| {
            n += 1;
            let extra = if n <= 2 {
                cv("MS:1000041", "charge state", n as f64)
                    + &cv("MS:1000042", "peak intensity", 1e6)
            } else {
                String::new()
            };
            format!("{p}{extra}")
        })
        .collect::<Vec<_>>()
        .join("</selectedIon>")
        .into_bytes()
}

fn plan(params: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"dda.mzML","type":"mzml"}},{{"name":"lib.msp","type":"library"}}],"steps":[
        {{"id":"s","step":"spectra.ms2@1","in":{{"source":["file:dda.mzML"]}},"params":{params}}},
        {{"id":"m","step":"spectra.match@1","in":{{"query":"s.spectra","library":"file:lib.msp"}}}}]}}"#
    )
}

fn run(dir: &Path, params: &str) -> String {
    let (run, prov) =
        crate::flow::run_plan(plan(params).as_bytes(), dir, &dir.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(dir.join("work/s.mgf")).unwrap()
}

/// The MGF has only the MS2 scans, with precursor, intensity, charge, RT and scan; the matcher finds Alpha from it; the bytes are the same on a second run.
#[test]
fn ms2_scans_become_spectra_that_the_matcher_reads() {
    let dir = fixture("ms2");
    std::fs::write(dir.join("dda.mzML"), dda_mzml()).unwrap();
    let mgf = run(&dir, "{}");
    assert_eq!(mgf.matches("BEGIN IONS").count(), 2, "{mgf}");
    assert!(mgf.contains("TITLE=dda.2\nSPECTRUMID=dda.2\nPEPMASS=300.12350 1000000.0\nRTINSECONDS=66.000\nSCANS=2\nMSLEVEL=2\nCHARGE=1+\nIONMODE=Positive\n"), "{mgf}");
    assert!(
        mgf.contains("CHARGE=2+") && mgf.contains("130.00000 5.0"),
        "{mgf}"
    );
    let hits = mzlab_core::json::parse(&std::fs::read(dir.join("work/m.json")).unwrap()).unwrap();
    let hits = hits.get("hits").unwrap().arr();
    let first = &hits[0];
    assert_eq!(first.get("query").unwrap().str(), Some("dda.2"));
    assert_eq!(first.get("name").unwrap().str(), Some("Alpha"));
    assert_eq!(run(&dir, "{}"), mgf);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `min_intensity` drops the weak peak and `max_isolation` the scan with the wide window.
#[test]
fn threshold_and_isolation_window_filter_the_scans() {
    let dir = fixture("ms2f");
    std::fs::write(dir.join("dda.mzML"), dda_mzml()).unwrap();
    let mgf = run(&dir, r#"{"min_intensity":100,"max_isolation":2}"#);
    assert_eq!(mgf.matches("BEGIN IONS").count(), 1, "{mgf}");
    assert!(!mgf.contains("130.00000"), "{mgf}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `docs/schemi/steps-cli-1.json` is the export of the steps that live in this crate (`MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-cli --lib flow_ms2` rewrites it).
#[test]
fn descriptor_export_is_up_to_date() {
    let text = mzlab_core::step::json::registry_json(&[crate::flow_ms2::ms2_step()]);
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/schemi/steps-cli-1.json");
    if std::env::var_os("MZLAB_UPDATE_STEPS").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let on_disk = std::fs::read_to_string(&path)
        .expect("docs/schemi/steps-cli-1.json (MZLAB_UPDATE_STEPS=1 writes it)");
    assert_eq!(
        on_disk, text,
        "stale: MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-cli --lib flow_ms2"
    );
}

/// The caffeine run of `caffeine_mzml` with three MS2 scans added: at the feature (195.08765, 3.0 min), at an ion that is no feature, and at the
/// feature m/z but 1.5 min late.
fn caffeine_dda() -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "caf".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let frag = [(83.06037, 150.0), (110.07127, 600.0), (138.06619, 1000.0)];
    w.open_spectra(123).unwrap();
    let mut k = 0;
    for i in 0..120 {
        let rt = 0.5 + 0.05 * i as f64;
        let a = 3.0e6 * (-0.5 * ((rt - 3.0) / 0.07f64).powi(2)).exp();
        let ms1 = [
            (195.08765, a as f32),
            (196.09100, (a * 0.101) as f32),
            (197.09435, (a * 0.0065) as f32),
        ];
        let ms1: Vec<_> = ms1.into_iter().filter(|p| p.1 > 100.0).collect();
        w.spectrum(&scan(k, 1, rt, None, &ms1)).unwrap();
        k += 1;
        let ms2 = match i {
            50 => Some((195.08765, rt)),
            51 => Some((400.1234, rt)),
            80 => Some((195.08765, rt)),
            _ => None,
        };
        if let Some((prec, rt)) = ms2 {
            w.spectrum(&scan(k, 2, rt, Some(prec), &frag)).unwrap();
            k += 1;
        }
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// `features.table` → `spectra.ms2` (with the table) → `spectra.match`: only the scan at the feature stays, named with the id of the feature, and the
/// matcher gives that id to its hit.
#[test]
fn feature_table_selects_the_scans_and_names_them() {
    let dir = fixture("ms2join");
    std::fs::write(dir.join("caf.mzML"), caffeine_dda()).unwrap();
    std::fs::write(
        dir.join("lib.msp"),
        "Name: Caffeine\nPrecursorMZ: 195.08765\nPrecursor_type: [M+H]+\nIon_mode: Positive\nFormula: C8H10N4O2\nNum Peaks: 3\n83.06037 150\n110.07127 600\n138.06619 1000\n\n",
    )
    .unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"caf.mzML","type":"mzml"},{"name":"lib.msp","type":"library"}],"steps":[
        {"id":"t","step":"features.table@1","in":{"source":["file:caf.mzML"]},"params":{"polarity":1}},
        {"id":"s","step":"spectra.ms2@1","in":{"source":["file:caf.mzML"],"features":"t.report"}},
        {"id":"m","step":"spectra.match@1","in":{"query":"s.spectra","library":"file:lib.msp"}}]}"#;
    let (run, prov) =
        crate::flow::run_plan(plan.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    let csv = std::fs::read_to_string(dir.join("work/t.csv")).unwrap();
    let head: Vec<&str> = csv.lines().next().unwrap().split(',').collect();
    let id = csv
        .lines()
        .nth(1)
        .unwrap()
        .split(',')
        .nth(head.iter().position(|h| *h == "id").unwrap())
        .unwrap()
        .to_string();
    let mgf = std::fs::read_to_string(dir.join("work/s.mgf")).unwrap();
    assert_eq!(mgf.matches("BEGIN IONS").count(), 1, "{mgf}");
    assert!(
        mgf.contains(&format!("SPECTRUMID={id}\n")) && mgf.contains(&format!("FEATURE_ID={id}\n")),
        "{mgf}"
    );
    let hits = mzlab_core::json::parse(&std::fs::read(dir.join("work/m.json")).unwrap()).unwrap();
    let first = &hits.get("hits").unwrap().arr()[0];
    assert_eq!(first.get("query").unwrap().str(), Some(id.as_str()));
    assert_eq!(first.get("name").unwrap().str(), Some("Caffeine"));
    let _ = std::fs::remove_dir_all(&dir);
}
