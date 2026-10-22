//! `convert.mzml` and its `peaks` parameter on a synthetic profile mzML (numpress, the one open format the step takes): `--set c.peaks=mzlab` gives our
//! centroids, the default keeps the profile and warns in `prov.json` and `corsa.json`, also when the step is taken again from the cache.

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Opzioni};

use crate::flow_run_tests::{flow_run, p};

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[{"name":"a.mzML","type":"raw"}],"steps":[
{"id":"c","step":"convert.mzml@1","in":{"raw":"file:a.mzML"}}]}"#;

fn profile_mzml() -> Vec<u8> {
    let mut w = MzmlWriter::con_opzioni(
        Vec::new(),
        &Header::default(),
        Opzioni {
            numpress: true,
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(4).unwrap();
    for k in 0..4 {
        let (mz, y): (Vec<f64>, Vec<f64>) = (0..400)
            .map(|i| {
                let m = 100.0 + f64::from(i) * 0.01;
                let g = |c: f64| 1000.0 * (-((m - c) / 0.2).powi(2)).exp();
                (m, g(101.0) + g(102.5) + g(104.0) + 5.0)
            })
            .unzip();
        w.spectrum(&Spectrum {
            index: k,
            id: format!("scan={}", k + 1),
            level: 1,
            rt: k as f64,
            polarity: 1,
            profile: true,
            mz,
            intensity: y.iter().map(|&v| v as f32).collect(),
            ..Spectrum::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

fn run(tag: &str, set: &[&str]) -> (std::path::PathBuf, String) {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_picchi_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data/a.mzML"), profile_mzml()).unwrap();
    std::fs::write(dir.join("plan.json"), PLAN).unwrap();
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let mut a = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
    for s in set {
        a.extend(["--set", s]);
    }
    let o = flow_run(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let mzml = std::fs::read_to_string(out.join("c-a.mzML")).unwrap();
    (out, mzml)
}

#[test]
fn convert_peaks_mzlab_writes_our_centroids() {
    let (out, mzml) = run("mzlab", &["c.peaks=mzlab"]);
    assert!(mzml.contains("MS:1000035") && !mzml.contains("MS:1000128"));
    let prov = std::fs::read_to_string(out.join("prov.json")).unwrap();
    assert!(!prov.contains("warn.flow.profile"), "{prov}");
    let _ = std::fs::remove_dir_all(out.parent().unwrap());
}

#[test]
fn convert_peaks_file_keeps_the_profile_and_warns() {
    let (out, mzml) = run("file", &[]);
    assert!(mzml.contains("MS:1000128") && !mzml.contains("MS:1000035"));
    for f in ["prov.json", "corsa.json"] {
        let t = std::fs::read_to_string(out.join(f)).unwrap();
        assert!(
            t.contains("\"warnings\":[\"warn.flow.profile\"]"),
            "{f}: {t}"
        );
    }
    let _ = std::fs::remove_dir_all(out.parent().unwrap());
}

#[test]
fn convert_peaks_profile_writes_the_profile_without_a_warning() {
    let (out, mzml) = run("profile", &["c.peaks=profile"]);
    assert!(mzml.contains("MS:1000128"));
    let prov = std::fs::read_to_string(out.join("prov.json")).unwrap();
    assert!(!prov.contains("warn.flow.profile"), "{prov}");
    let _ = std::fs::remove_dir_all(out.parent().unwrap());
}
