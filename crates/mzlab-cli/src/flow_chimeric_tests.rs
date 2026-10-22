//! `spectra.chimeric` on a synthetic DDA mzML made here: one MS2 whose window holds a second ion in the MS1 scans around it, one that is alone.
//! The plan mzML → `spectra.ms2` → `spectra.chimeric` flags the first and not the second; the intruders come from a list of feature points.

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_feat::attributio::ms2_engine::chimeric::FeaturePoint;

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

/// MS1 (rt 1.0), MS2 at 300.0 (1.1), MS1 (1.2), MS1 (2.0), MS2 at 500.0 (2.1), MS1 (2.2). Around 300.0 there is an ion of the same height at 300.4;
/// around 500.0 there is none. The windows are ±0.75 Da.
fn dda_mzml() -> Vec<u8> {
    dda_mzml_window(0.75)
}

/// The same file with isolation window offsets of `half` Da.
fn dda_mzml_window(half: f64) -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "chi".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(6).unwrap();
    let busy = [(300.0, 1e6f32), (300.4, 1e6), (700.0, 5e5)];
    let alone = [(500.0, 1e6f32), (700.0, 5e5)];
    let frag = [(100.05, 400.0f32), (120.08, 900.0)];
    w.spectrum(&scan(0, 1, 1.0, None, &busy)).unwrap();
    w.spectrum(&scan(1, 2, 1.1, Some(300.0), &frag)).unwrap();
    w.spectrum(&scan(2, 1, 1.2, None, &busy)).unwrap();
    w.spectrum(&scan(3, 1, 2.0, None, &alone)).unwrap();
    w.spectrum(&scan(4, 2, 2.1, Some(500.0), &frag)).unwrap();
    w.spectrum(&scan(5, 1, 2.2, None, &alone)).unwrap();
    w.close_spectra().unwrap();
    let text = String::from_utf8(w.finish().unwrap()).unwrap();
    let cv = |acc: &str, name: &str| {
        format!(
            "<cvParam cvRef=\"MS\" accession=\"{acc}\" name=\"{name}\" value=\"{}\"/>",
            half
        )
    };
    let mut parts = text
        .split("</isolationWindow>")
        .map(str::to_string)
        .collect::<Vec<_>>();
    let tail = parts.pop().unwrap();
    let mut out = String::new();
    for p in parts {
        out += &p;
        out += &(cv("MS:1000828", "isolation window lower offset")
            + &cv("MS:1000829", "isolation window upper offset"));
        out += "</isolationWindow>";
    }
    (out + &tail).into_bytes()
}

fn header(mgf: &str, title: &str, key: &str) -> Option<String> {
    let block = mgf
        .split("BEGIN IONS")
        .find(|b| b.contains(&format!("TITLE={title}\n")))?;
    block
        .lines()
        .find_map(|l| l.strip_prefix(key))
        .map(str::to_string)
}

#[test]
fn the_ion_beside_the_precursor_makes_the_spectrum_chimeric() {
    let dir = fixture("chimeric");
    std::fs::write(dir.join("chi.mzML"), dda_mzml()).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"chi.mzML","type":"mzml"}],"steps":[
        {"id":"idx","step":"index.mzidx@1","in":{"source":["file:chi.mzML"]}},
        {"id":"s","step":"spectra.ms2@1","in":{"source":["file:chi.mzML"]}},
        {"id":"c","step":"spectra.chimeric@1","in":{"spectra":"s.spectra","index":["idx.index"]}}]}"#;
    let (run, prov) =
        crate::flow::run_plan(plan.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    let mgf = std::fs::read_to_string(dir.join("work/c.mgf")).unwrap();
    assert_eq!(mgf.matches("BEGIN IONS").count(), 2, "{mgf}");
    let purity = |t: &str| header(&mgf, t, "PURITY=").unwrap().parse::<f64>().unwrap();
    assert!((purity("chi.2") - 0.5).abs() < 1e-9, "{mgf}");
    assert!((purity("chi.5") - 1.0).abs() < 1e-9, "{mgf}");
    assert_eq!(header(&mgf, "chi.2", "CHIMERIC=").as_deref(), Some("true"));
    assert_eq!(header(&mgf, "chi.5", "CHIMERIC=").as_deref(), Some("false"));
    assert_eq!(
        header(&mgf, "chi.2", "PURITY_BAND=").as_deref(),
        Some("medium")
    );
    assert_eq!(
        header(&mgf, "chi.5", "PURITY_BAND=").as_deref(),
        Some("high")
    );
    // the peaks are untouched
    assert!(mgf.contains("100.05000 400.0\n120.08000 900.0\n"), "{mgf}");
    // the same bytes on a second run
    let (_, again) =
        crate::flow::run_plan(plan.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
    assert!(!again.contains("\"error\""));
    assert_eq!(
        mgf,
        std::fs::read_to_string(dir.join("work/c.mgf")).unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_ms1_window_algorithm_lists_the_ions_beside_the_precursor() {
    let dir = fixture("chimeric_ms1");
    std::fs::write(dir.join("chi.mzML"), dda_mzml()).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"chi.mzML","type":"mzml"}],"steps":[
        {"id":"idx","step":"index.mzidx@1","in":{"source":["file:chi.mzML"]}},
        {"id":"s","step":"spectra.ms2@1","in":{"source":["file:chi.mzML"]}},
        {"id":"c","step":"spectra.chimeric@1","algorithm":"ms1_window","in":{"spectra":"s.spectra","index":["idx.index"]},"params":{"chim_rel":0.2,"chim_cap":0.5}}]}"#;
    let (_, prov) =
        crate::flow::run_plan(plan.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
    assert!(!prov.contains("\"error\""), "{prov}");
    let mgf = std::fs::read_to_string(dir.join("work/c.mgf")).unwrap();
    let ions = header(&mgf, "chi.2", "MS1_COISOLATED=").unwrap();
    assert!(ions.starts_with("300.4000:"), "{mgf}");
    assert_eq!(
        header(&mgf, "chi.2", "MS1_COISOLATED_N=").as_deref(),
        Some("1")
    );
    assert_eq!(
        header(&mgf, "chi.2", "MS1_CAPPED=").as_deref(),
        Some("true")
    );
    assert_eq!(
        header(&mgf, "chi.5", "MS1_COISOLATED=").as_deref(),
        Some(""),
        "{mgf}"
    );
    assert_eq!(
        header(&mgf, "chi.5", "MS1_CAPPED=").as_deref(),
        Some("false")
    );
    // the purity of the default algorithm is still written
    assert!(header(&mgf, "chi.2", "PURITY=").is_some(), "{mgf}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn library_deconvolution_splits_the_spectrum_among_the_library_spectra_in_the_window() {
    let dir = fixture("chimeric_deconv");
    std::fs::write(dir.join("chi.mzML"), dda_mzml()).unwrap();
    // the MS2 at 300.0 holds 100.05 and 120.08; two library spectra explain one peak each, a third lies outside the window
    let lib = "Name: First\nPrecursorMZ: 300.0\nIonMode: Positive\nNum Peaks: 1\n100.05 100\n\n\
               Name: Second\nPrecursorMZ: 300.4\nIonMode: Positive\nNum Peaks: 1\n120.08 100\n\n\
               Name: Far\nPrecursorMZ: 450.0\nIonMode: Positive\nNum Peaks: 1\n100.05 100\n\n";
    std::fs::write(dir.join("lib.msp"), lib).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"chi.mzML","type":"mzml"},{"name":"lib.msp","type":"library"}],"steps":[
        {"id":"idx","step":"index.mzidx@1","in":{"source":["file:chi.mzML"]}},
        {"id":"s","step":"spectra.ms2@1","in":{"source":["file:chi.mzML"]}},
        {"id":"c","step":"spectra.chimeric@1","algorithm":"library_deconvolution","in":{"spectra":"s.spectra","index":["idx.index"],"library":"file:lib.msp"},"params":{"window":1.5}}]}"#;
    let (_, prov) =
        crate::flow::run_plan(plan.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
    assert!(!prov.contains("\"error\""), "{prov}");
    let mgf = std::fs::read_to_string(dir.join("work/c.mgf")).unwrap();
    let list = header(&mgf, "chi.2", "DECONV=").unwrap();
    assert!(list.contains("First:") && list.contains("Second:"), "{mgf}");
    assert!(!list.contains("Far"), "{mgf}");
    let residual = header(&mgf, "chi.2", "DECONV_RESIDUAL=").unwrap();
    assert!(residual.parse::<f64>().unwrap() < 0.01, "{mgf}");
    // the peaks are untouched
    assert!(mgf.contains("100.05000 400.0\n120.08000 900.0\n"), "{mgf}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_features_inside_the_window_are_listed_nearest_first() {
    let point = |id, mz, rt| FeaturePoint {
        pol: "POS".into(),
        id,
        mz,
        rt,
    };
    let all = [
        point(1, 300.0, 1.1),
        point(2, 300.5, 1.12),
        point(3, 300.2, 1.1),
        point(4, 300.2, 3.0),
        point(5, 302.0, 1.1),
    ];
    let mgf = "BEGIN IONS\nTITLE=a\nPEPMASS=300.0\nRTINSECONDS=66.000\nFEATURE_ID=1\n100.0 5.0\nEND IONS\n";
    let text = crate::flow_chimeric::chimeric_text(
        (1.5, 0.1),
        mgf.as_bytes(),
        (&[None], &[]),
        0.5,
        &[],
        Some(&all),
    );
    assert_eq!(
        header(&text, "a", "INTRUDERS=").as_deref(),
        Some("3:0.2:0;2:0.5:0.02"),
        "{text}"
    );
    assert!(!text.contains("PURITY="));
}

/// PURITY of the MS2 at 300.0 under isolation windows of several widths: the ion at 300.4 counts only when the real window reaches it.
#[test]
fn purity_follows_the_real_isolation_window() {
    let purity_at = |half: f64| {
        let dir = fixture(&format!("chimeric_win{}", (half * 100.0) as u32));
        std::fs::write(dir.join("chi.mzML"), dda_mzml_window(half)).unwrap();
        let plan = r#"{"format":"mzflow/2","inputs":[{"name":"chi.mzML","type":"mzml"}],"steps":[
            {"id":"idx","step":"index.mzidx@1","in":{"source":["file:chi.mzML"]}},
            {"id":"s","step":"spectra.ms2@1","in":{"source":["file:chi.mzML"]}},
            {"id":"c","step":"spectra.chimeric@1","in":{"spectra":"s.spectra","index":["idx.index"]}}]}"#;
        let (_, prov) =
            crate::flow::run_plan(plan.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
        assert!(!prov.contains("\"error\""), "{prov}");
        let mgf = std::fs::read_to_string(dir.join("work/c.mgf")).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        header(&mgf, "chi.2", "PURITY=").map(|v| v.parse::<f64>().unwrap())
    };
    // ±0.2 Da stops short of 300.4; ±0.5 and ±1.5 Da reach it
    assert!((purity_at(0.2).unwrap() - 1.0).abs() < 1e-9);
    assert!((purity_at(0.5).unwrap() - 0.5).abs() < 1e-9);
    assert!((purity_at(1.5).unwrap() - 0.5).abs() < 1e-9);
}

/// `purity_min` moves the chimeric limit (the purity of 0.5 stays at the limit, not below it) and the bands follow it.
#[test]
fn purity_min_is_a_parameter_of_the_flag() {
    let flag = |extra: &str| {
        let dir = fixture(&format!("chimeric_min{}", extra.len()));
        std::fs::write(dir.join("chi.mzML"), dda_mzml()).unwrap();
        let plan = format!(
            r#"{{"format":"mzflow/2","inputs":[{{"name":"chi.mzML","type":"mzml"}}],"steps":[
            {{"id":"idx","step":"index.mzidx@1","in":{{"source":["file:chi.mzML"]}}}},
            {{"id":"s","step":"spectra.ms2@1","in":{{"source":["file:chi.mzML"]}}}},
            {{"id":"c","step":"spectra.chimeric@1","in":{{"spectra":"s.spectra","index":["idx.index"]}},"params":{{{extra}}}}}]}}"#
        );
        let (_, prov) =
            crate::flow::run_plan(plan.as_bytes(), &dir, &dir.join("work"), 1, None).unwrap();
        assert!(!prov.contains("\"error\""), "{prov}");
        let mgf = std::fs::read_to_string(dir.join("work/c.mgf")).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let get = |k: &str| header(&mgf, "chi.2", k).unwrap();
        (get("CHIMERIC="), get("PURITY_BAND="))
    };
    assert_eq!(flag(""), ("true".into(), "medium".into()));
    assert_eq!(flag(r#""purity_min":0.5"#), ("false".into(), "high".into()));
    assert_eq!(
        flag(r#""purity_min":0.9"#),
        ("true".into(), "medium".into())
    );
}
