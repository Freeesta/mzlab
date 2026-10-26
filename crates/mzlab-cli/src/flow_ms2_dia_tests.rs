//! `spectra.ms2` with `dia_pseudo` on a DIA mzML made here (two windows, three precursors, a constant background ion in each window) and a feature table:
//! the MGF has one record per feature with exactly its fragments; without `dia_pseudo` the output does not change.

use std::path::Path;

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};

use crate::flow_ident_tests::fixture;

fn scan(
    index: usize,
    level: u8,
    rt: f64,
    precursor: Option<f64>,
    peaks: &[(f64, f64)],
) -> Spectrum {
    Spectrum {
        index,
        id: format!("scan={}", index + 1),
        level,
        rt,
        polarity: 1,
        precursor,
        mz: peaks.iter().map(|p| p.0).collect(),
        intensity: peaks.iter().map(|p| p.1 as f32).collect(),
        ..Spectrum::default()
    }
}

/// (window centre, precursor m/z, apex in seconds, fragments)
const PRECS: [(f64, f64, f64, [f64; 3]); 3] = [
    (150.0, 130.0, 30.0, [70.0123, 90.0456, 110.0789]),
    (150.0, 170.0, 60.0, [60.0111, 80.0222, 120.0333]),
    (250.0, 250.0, 45.0, [100.0555, 150.0666, 200.0777]),
];
const BACKGROUND: [f64; 2] = [85.5, 285.5];

fn gauss(t: f64, apex: f64) -> f64 {
    (-0.5 * ((t - apex) / 3.0).powi(2)).exp()
}

/// 90 cycles of one MS1 and one MS2 per window (isolation 100 Da wide around the centre).
fn dia_mzml() -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "dia".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(270).unwrap();
    let mut k = 0;
    for t in 0..90 {
        let sec = t as f64;
        let ms1: Vec<(f64, f64)> = PRECS
            .iter()
            .map(|p| (p.1, 1e6 * gauss(sec, p.2)))
            .filter(|p| p.1 > 1.0)
            .collect();
        w.spectrum(&scan(k, 1, sec / 60.0, None, &ms1)).unwrap();
        k += 1;
        for (c, bg) in [(150.0, BACKGROUND[0]), (250.0, BACKGROUND[1])] {
            let mut peaks = vec![(bg, 2000.0)];
            for p in PRECS.iter().filter(|p| p.0 == c) {
                for (n, f) in p.3.iter().enumerate() {
                    peaks.push((*f, 5e5 * (n + 1) as f64 * gauss(sec + 0.3, p.2)));
                }
            }
            peaks.retain(|p| p.1 > 1.0);
            peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
            w.spectrum(&scan(k, 2, (sec + 0.3) / 60.0, Some(c), &peaks))
                .unwrap();
            k += 1;
        }
    }
    w.close_spectra().unwrap();
    let text = String::from_utf8(w.finish().unwrap()).unwrap();
    let cv = |acc: &str, name: &str| {
        format!("<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"50\"/>")
    };
    let mut parts = text
        .split("</isolationWindow>")
        .map(str::to_string)
        .collect::<Vec<_>>();
    let tail = parts.pop().unwrap();
    let mut out = String::new();
    for p in &parts {
        out += p;
        out += &(cv("MS:1000828", "isolation window lower offset")
            + &cv("MS:1000829", "isolation window upper offset"));
        out += "</isolationWindow>";
    }
    out += &tail;
    out.into_bytes()
}

const TABLE: &str = "id,mz,rt\nf1,130,0.5\nf2,170,1.0\nf3,250,0.75\n";

fn plan(params: &str, table: bool) -> String {
    let (input, link) = if table {
        (
            r#",{"name":"feat.csv","type":"report"}"#,
            r#","features":"file:feat.csv""#,
        )
    } else {
        ("", "")
    };
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"dia.mzML","type":"mzml"}}{input}],"steps":[
        {{"id":"s","step":"spectra.ms2@1","in":{{"source":["file:dia.mzML"]{link}}},"params":{params}}}]}}"#
    )
}

fn run(dir: &Path, params: &str, table: bool) -> String {
    let (run, prov) = crate::flow::run_plan(
        plan(params, table).as_bytes(),
        dir,
        &dir.join("work"),
        1,
        None,
    )
    .unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(dir.join("work/s.mgf")).unwrap()
}

/// The peaks of the record with `TITLE`.
fn peaks_of(mgf: &str, title: &str) -> Vec<String> {
    let rec = mgf
        .split("BEGIN IONS\n")
        .find(|r| r.starts_with(&format!("TITLE={title}\n")))
        .unwrap_or_else(|| panic!("{title} in {mgf}"));
    rec.lines()
        .filter(|l| l.starts_with(|c: char| c.is_ascii_digit()) && !l.contains('='))
        .map(|l| l.split(' ').next().unwrap().to_string())
        .collect()
}

/// One record per feature with exactly its fragments (no background ion, none of the other precursors), apex as RT and the m/z of the feature.
#[test]
fn dia_pseudo_spectra_have_the_fragments_of_each_feature() {
    let dir = fixture("ms2dia");
    std::fs::write(dir.join("dia.mzML"), dia_mzml()).unwrap();
    std::fs::write(dir.join("feat.csv"), TABLE).unwrap();
    let mgf = run(&dir, r#"{"dia_pseudo":true}"#, true);
    assert_eq!(mgf.matches("BEGIN IONS").count(), 3, "{mgf}");
    for (n, p) in PRECS.iter().enumerate() {
        let want: Vec<String> = p.3.iter().map(|f| format!("{f:.5}")).collect();
        assert_eq!(
            peaks_of(&mgf, &format!("dia.f{}.pseudo", n + 1)),
            want,
            "{mgf}"
        );
    }
    assert!(
        mgf.contains("SPECTRUMID=f1\nPEPMASS=130.00000\nRTINSECONDS=30.300\n"),
        "{mgf}"
    );
    assert_eq!(run(&dir, r#"{"dia_pseudo":true}"#, true), mgf);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Without `dia_pseudo` the DIA scans are written as they always were: the same bytes with the parameter absent or false.
#[test]
fn without_dia_pseudo_the_output_is_unchanged() {
    let dir = fixture("ms2dia_off");
    std::fs::write(dir.join("dia.mzML"), dia_mzml()).unwrap();
    let base = run(&dir, "{}", false);
    assert_eq!(run(&dir, r#"{"dia_pseudo":false}"#, false), base);
    assert!(!base.contains(".pseudo"), "{base}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dia_pseudo_needs_the_feature_table() {
    let dir = fixture("ms2dia_miss");
    std::fs::write(dir.join("dia.mzML"), dia_mzml()).unwrap();
    let res = crate::flow::run_plan(
        plan(r#"{"dia_pseudo":true}"#, false).as_bytes(),
        &dir,
        &dir.join("work"),
        1,
        None,
    );
    let text = match res {
        Ok((_, prov)) => prov,
        Err(e) => format!("{e:?}"),
    };
    assert!(
        text.contains("err.flow.port.missing") && text.contains("features"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The DDA output of `spectra.ms2` is the same with `dia_pseudo` absent or false (the pinned SHA-256 is that of the output before the parameter existed).
#[test]
fn dda_output_has_the_same_hash() {
    let dir = fixture("ms2dia_dda");
    std::fs::write(dir.join("dda.mzML"), super::dda_mzml()).unwrap();
    let plan = |params: &str| {
        format!(
            r#"{{"format":"mzflow/2","inputs":[{{"name":"dda.mzML","type":"mzml"}}],"steps":[{{"id":"s","step":"spectra.ms2@1","in":{{"source":["file:dda.mzML"]}},"params":{params}}}]}}"#
        )
    };
    for params in ["{}", r#"{"dia_pseudo":false}"#] {
        crate::flow::run_plan(plan(params).as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
        let mgf = std::fs::read(dir.join("work/s.mgf")).unwrap();
        assert_eq!(
            mzlab_core::budget::sha256_hex(&mgf),
            "62ef91a3ce953e3c7f2ee56ff666c612ebf0e1b6c179367af1ed873db0a6982c",
            "{params}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
