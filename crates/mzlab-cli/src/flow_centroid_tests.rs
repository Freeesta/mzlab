//! `spectra.centroid@1` on synthetic profile mzML: centres of known peaks, bytes of the archive conversion, a centroid file passes through untouched,
//! the later steps give the same table as without the step, `--set` reaches the parameters, same bytes twice.

use std::path::{Path, PathBuf};

use mzlab_core::mzml::{self, Spectrum};
use mzlab_core::mzmlwrite::{Header, MzmlWriter};

use crate::tests::mzml as centroid_mzml;

/// Gaussian peaks (centre, height) at unit resolution on a 0.1 Da grid, `n` scans of 280 to 320.
fn profile(n: usize, peaks: &[(f64, f64)]) -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "prof".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(n).unwrap();
    for i in 0..n {
        let mz: Vec<f64> = (0..400).map(|k| 280.0 + 0.1 * k as f64).collect();
        let y: Vec<f64> = mz
            .iter()
            .enumerate()
            .map(|(k, &m)| {
                let s: f64 = peaks
                    .iter()
                    .map(|&(c, h)| h * (-0.5 * ((m - c) / 0.3).powi(2)).exp())
                    .sum();
                // a fixed noise of a few intensity units, not whole counts: the continuous mode of the centroider
                let noise = (((i * 400 + k) as u64).wrapping_mul(2_654_435_761) >> 7) % 1000;
                (s + 5.0 + noise as f64 * 0.006) * 1.37
            })
            .collect();
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: 0.5 + 0.05 * i as f64,
            polarity: 1,
            profile: true,
            filter: "+ p ESI Q1MS [280.000-320.000]".into(),
            precursor: None,
            mz: mz.clone(),
            intensity: y.iter().map(|&v| v as f32).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

fn dir(tag: &str, files: &[(&str, Vec<u8>)]) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_centroid_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for (n, b) in files {
        std::fs::write(d.join(n), b).unwrap();
    }
    d
}

fn run(d: &Path, plan: &str, work: &str) -> (mzlab_core::flow::Run, String) {
    std::fs::create_dir_all(d.join(work)).unwrap();
    crate::flow::run_plan(plan.as_bytes(), d, &d.join(work), 1, None).unwrap()
}

fn plan(params: &str) -> String {
    format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"p.mzML","type":"raw"}}],"steps":[
        {{"id":"c","step":"spectra.centroid@1","in":{{"source":"file:p.mzML"}},"params":{params}}}]}}"#
    )
}

fn centres(bytes: &[u8]) -> Vec<Vec<f64>> {
    let run = mzml::read(bytes).unwrap();
    assert!(run.spectra.iter().all(|s| !s.profile));
    run.spectra.iter().map(|s| s.mz.clone()).collect()
}

const PEAKS: [(f64, f64); 3] = [(290.0, 4000.0), (300.2, 9000.0), (310.0, 2500.0)];

#[test]
fn a_profile_file_gets_the_centres_of_its_peaks_and_the_bytes_of_the_archive_conversion() {
    let src = profile(30, &PEAKS);
    let d = dir("a", &[("p.mzML", src.clone())]);
    let (run, prov) = run(&d, &plan("{}"), "w");
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    let got = std::fs::read(d.join("w/c-p.mzML")).unwrap();
    let mut want = Vec::new();
    mzlab_core::centroidi::mzml_to_centroids(
        || Ok(src.as_slice()),
        &mut want,
        "p.mzML",
        &mut |_, _| {},
    )
    .unwrap();
    assert_eq!(got, want, "the step is the conversion of the archive");
    for scan in centres(&got) {
        for c in [290.0, 300.2, 310.0] {
            assert!(
                scan.iter().any(|&m| (m - c).abs() < 0.05),
                "{c} in {scan:?}"
            );
        }
    }
    // the values it measured are reported, and the same plan gives the same bytes
    assert!(
        prov.contains("\"fwhm\"") && prov.contains("\"threshold\""),
        "{prov}"
    );
    run_again(&d, &got);
    let _ = std::fs::remove_dir_all(&d);
}

fn run_again(d: &Path, first: &[u8]) {
    run(d, &plan("{}"), "w2");
    assert_eq!(std::fs::read(d.join("w2/c-p.mzML")).unwrap(), first);
}

#[test]
fn the_parameters_reach_the_centroider() {
    let d = dir("b", &[("p.mzML", profile(20, &PEAKS))]);
    run(&d, &plan("{}"), "w");
    // a threshold no peak reaches leaves no centroid: the value of --set is the one used
    run(&d, &plan(r#"{"threshold":1e9}"#), "hi");
    let strict = centres(&std::fs::read(d.join("hi/c-p.mzML")).unwrap());
    assert!(strict.iter().all(Vec::is_empty), "{strict:?}");
    let wide = run(&d, &plan(r#"{"fwhm":0.9}"#), "wide").1;
    assert!(wide.contains("0.9"), "{wide}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_centroid_file_goes_through_and_the_table_is_the_same_as_without_the_step() {
    let files: Vec<(String, Vec<u8>)> = [0.0, 10.0, 30.0]
        .iter()
        .map(|&t| (format!("s_t{:03}.mzML", t as u32), centroid_mzml(t)))
        .collect();
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    let d = dir("c", &refs);
    let inputs: Vec<String> = files
        .iter()
        .map(|(n, _)| format!(r#"{{"name":"{n}","type":"raw"}}"#))
        .collect();
    let refs_in: Vec<String> = files
        .iter()
        .map(|(n, _)| format!(r#""file:{n}""#))
        .collect();
    let chain = |src: &str, centroid: bool| {
        let first = if centroid {
            format!(
                r#"{{"id":"cen","step":"spectra.centroid@1","in":{{"source":[{}]}}}},"#,
                refs_in.join(",")
            )
        } else {
            String::new()
        };
        format!(
            r#"{{"format":"mzflow/2","inputs":[{}],"steps":[{first}
            {{"id":"idx","step":"index.mzidx@1","in":{{"source":[{src}]}}}},
            {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
            {{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}}]}}"#,
            inputs.join(",")
        )
    };
    run(&d, &chain(&refs_in.join(","), false), "plain");
    run(&d, &chain(r#""cen.mzml""#, true), "cen");
    for (n, b) in &files {
        let k = n.trim_end_matches(".mzML");
        assert_eq!(
            std::fs::read(d.join(format!("cen/cen-{k}.mzML"))).unwrap(),
            *b,
            "{n} passes through"
        );
    }
    assert_eq!(
        std::fs::read(d.join("cen/ali.mzfeat")).unwrap(),
        std::fs::read(d.join("plain/ali.mzfeat")).unwrap(),
        "the table of a centroid series does not change"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_index_after_the_step_holds_centroids_and_the_one_without_it_profile() {
    let d = dir("d", &[("p.mzML", profile(10, &PEAKS))]);
    let plain = r#"{"format":"mzflow/2","inputs":[{"name":"p.mzML","type":"raw"}],"steps":[
        {"id":"i","step":"index.mzidx@1","in":{"source":"file:p.mzML"}}]}"#;
    let after = r#"{"format":"mzflow/2","inputs":[{"name":"p.mzML","type":"raw"}],"steps":[
        {"id":"c","step":"spectra.centroid@1","in":{"source":"file:p.mzML"}},
        {"id":"i","step":"index.mzidx@1","in":{"source":"c.mzml"}}]}"#;
    run(&d, plain, "a");
    run(&d, after, "b");
    let profile_of = |w: &str| {
        let f = std::fs::File::open(d.join(w).join("i-p.mzidx")).unwrap();
        let size = f.metadata().unwrap().len();
        let idx = mzlab_core::idxread::Index::open(f, size).unwrap();
        idx.scans.iter().all(|s| s.profile)
    };
    assert!(profile_of("a"));
    assert!(!profile_of("b"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_progress_is_monotone_ends_at_the_total_and_leaves_the_bytes_alone() {
    let src = profile(300, &PEAKS);
    let mut plain = Vec::new();
    mzlab_core::centroidi::mzml_to_centroids(
        || Ok(src.as_slice()),
        &mut plain,
        "p.mzML",
        &mut |_, _| {},
    )
    .unwrap();
    let (mut seen, mut with) = (Vec::new(), Vec::new());
    {
        let mut sink = |d, t| seen.push((d, t));
        let mut pace = mzlab_core::pace::Pace::new(&mut sink);
        mzlab_core::centroidi::mzml_to_centroids(
            || Ok(src.as_slice()),
            &mut with,
            "p.mzML",
            &mut |d, t| pace.at(d, t),
        )
        .unwrap();
    }
    assert_eq!(plain, with, "the callback does not change a byte");
    assert!(seen.len() <= 101 && seen.windows(2).all(|w| w[0].0 < w[1].0));
    assert_eq!(seen.last(), Some(&(300, 300)));
}

#[test]
fn two_aligns_on_the_same_peaks_give_the_family_table_and_the_ion_table() {
    let files: Vec<(String, Vec<u8>)> = [0.0, 10.0]
        .iter()
        .map(|&t| (format!("s_t{:03}.mzML", t as u32), centroid_mzml(t)))
        .collect();
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
    let d = dir("fi", &refs);
    let inputs: Vec<String> = files
        .iter()
        .map(|(n, _)| format!(r#"{{"name":"{n}","type":"raw"}}"#))
        .collect();
    let sources: Vec<String> = files
        .iter()
        .map(|(n, _)| format!(r#""file:{n}""#))
        .collect();
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{}],"steps":[
        {{"id":"idx","step":"index.mzidx@1","in":{{"source":[{}]}}}},
        {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
        {{"id":"fam","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}},
        {{"id":"ion","step":"features.align@1","in":{{"peaks":["det.peaks"]}},"params":{{"mode":"ion"}}}}]}}"#,
        inputs.join(","),
        sources.join(",")
    );
    run(&d, &plan, "w");
    // the detection is shared; the family table keeps the members of each row, the ion table has one row per ion
    let fam = std::fs::read(d.join("w/fam.mzfeat")).unwrap();
    let ion = std::fs::read(d.join("w/ion.mzfeat")).unwrap();
    assert!(!fam.is_empty() && !ion.is_empty() && fam != ion);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_mixed_series_centroids_the_profile_files_only_and_the_record_says_which() {
    let cen = centroid_mzml(10.0);
    let d = dir(
        "mix",
        &[
            ("p.mzML", profile(20, &PEAKS)),
            ("s_t010.mzML", cen.clone()),
        ],
    );
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"p.mzML","type":"raw"},{"name":"s_t010.mzML","type":"raw"}],"steps":[
        {"id":"c","step":"spectra.centroid@1","in":{"source":["file:p.mzML","file:s_t010.mzML"]}}]}"#;
    let (run, prov) = run(&d, plan, "w");
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    assert_eq!(std::fs::read(d.join("w/c-s_t010.mzML")).unwrap(), cen);
    assert!(!centres(&std::fs::read(d.join("w/c-p.mzML")).unwrap())[0].is_empty());
    let files = prov.split("\"files\":").nth(1).expect(&prov);
    let (p, s) = (
        files.find("\"p.mzML\"").unwrap(),
        files.find("\"s_t010.mzML\"").unwrap(),
    );
    assert!(
        files[p..s.max(p)].contains("\"via\":\"centroided\""),
        "{files}"
    );
    assert!(files[s..].contains("\"via\":\"passthrough\""), "{files}");
    // a second run takes both from the cache and says the same
    let again = run_cached(&d, plan);
    assert!(
        again.contains("\"via\":\"centroided\"") && again.contains("\"via\":\"passthrough\""),
        "{again}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

fn run_cached(d: &Path, plan: &str) -> String {
    run(d, plan, "w").1
}

/// The profile files of `tools/dati_sintetici.py` (`P_FullMass-t*.mzML`) in a new folder; `None` without python3 and numpy.
fn sintetici_profilo() -> Option<PathBuf> {
    let d = std::env::temp_dir().join(format!("mzlab_centroid_p_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools");
    let script = "import sys; from pathlib import Path; import numpy as np; sys.path.insert(0, sys.argv[1]); import dati_sintetici as d; o = Path(sys.argv[2]); \
                  r = np.random.default_rng(5); [d.full_scan_profile(o / f'P_FullMass-t{t}.mzML', t, r) for t in (0, 15, 60)]";
    let ok = std::process::Command::new("python3")
        .args(["-c", script])
        .arg(&tools)
        .arg(&d)
        .status()
        .is_ok_and(|s| s.success());
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return None;
    }
    Some(d)
}

/// The browser calls `mzml_to_centroids` (engine of Settings, no parameters) and the CLI step runs `mzml_to_centroids_with` with none left out: the bytes of every low-resolution profile file are the same.
#[test]
fn the_step_and_the_engine_call_of_the_browser_give_the_same_bytes_on_the_synthetic_profile_files()
{
    let Some(d) = sintetici_profilo() else { return };
    for t in [0, 15, 60] {
        let name = format!("P_FullMass-t{t}.mzML");
        let src = std::fs::read(d.join(&name)).unwrap();
        let plan = plan("{}").replace("p.mzML", &name);
        let (run, prov) = run(&d, &plan, &format!("w{t}"));
        assert!(
            run.skipped.is_empty() && !prov.contains("\"error\""),
            "{prov}"
        );
        let got = std::fs::read(d.join(format!("w{t}/c-{name}"))).unwrap();
        let mut want = Vec::new();
        let did = mzlab_core::centroidi::mzml_to_centroids(
            || Ok(src.as_slice()),
            &mut want,
            &name,
            &mut |_, _| {},
        )
        .unwrap();
        assert!(did, "{name} is a profile file");
        assert_eq!(got, want, "{name}");
    }
    let _ = std::fs::remove_dir_all(&d);
}
