//! The matrix of the scan modes of a QTRAP against the synthetic pairs of `tools/wiff_modi.py`: what the `.wiff` reader does today with each mode, exactly
//! (read with level / precursor / CV type, or refused with `err.file.wiff` and the `scan type code C (experiment 1)` detail), never a panic. Whoever adds
//! a mode changes its entry in `modi.json`. Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn generate() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_modi_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_modi.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_modi: python3 not available");
        return None;
    }
    let dir = out.join("modi");
    let json = serde_json::from_slice(&std::fs::read(dir.join("modi.json")).unwrap()).unwrap();
    Some((dir, json))
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn detail(e: &mzlab_core::error::Error) -> String {
    assert_eq!(e.key, "err.file.wiff");
    e.params
        .iter()
        .find(|(k, _)| *k == "detail")
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

fn set(dir: &Path, name: &str) -> FileSet {
    let member = |file: String| {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        Membro {
            nome: file,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![
            member(format!("{name}.wiff")),
            member(format!("{name}.wiff.scan")),
        ],
    }
}

fn check(dir: &Path, sigla: &str, sample: &Value) {
    let n = sample["n"].as_u64().unwrap() as u32;
    let want = &sample["oggi"];
    let ole = container(
        std::fs::read(dir.join(format!("{sigla}.wiff"))).unwrap(),
        sigla,
    )
    .unwrap();
    let mut scan = std::fs::read(dir.join(format!("{sigla}.wiff.scan"))).unwrap();
    let got = Wiff::open(&ole, sigla, Some(scan.len() as u64), n)
        .and_then(|w| w.run(&mut scan).map(|r| (w.polarity, r)));
    if want["esito"] == "rifiuto" {
        let err = got.err().unwrap_or_else(|| panic!("{sigla}: read"));
        assert_eq!(detail(&err), want["detail"].as_str().unwrap(), "{sigla}");
        return;
    }
    let (polarity, run) = got.unwrap_or_else(|e| panic!("{sigla}: {}", e.key));
    assert_eq!(polarity, 1, "{sigla}: polarity");
    assert_eq!(
        run.spectra.len() as u64,
        want["spettri"].as_u64().unwrap(),
        "{sigla}: spectra"
    );
    if want["spettri"] == 0 {
        let tic = run.chromatograms.iter().filter(|c| c.id == "TIC").count();
        assert_eq!(tic, 1, "{sigla}: TIC");
        assert_eq!(
            run.chromatograms.len() as u64,
            want["cromatogrammi"].as_u64().unwrap(),
            "{sigla}: chromatograms"
        );
        return;
    }
    let (mz, it) = (nums(&want["mz"]), nums(&want["intensita"]));
    for (k, s) in run.spectra.iter().enumerate() {
        assert_eq!(
            s.id,
            format!("sample={n} period=1 cycle={} experiment=1", k + 1)
        );
        assert_eq!(s.level as u64, want["livello"].as_u64().unwrap(), "{sigla}");
        assert_eq!(s.precursor, want["precursore"].as_f64(), "{sigla}");
        if k == 0 {
            assert_eq!(s.mz.len(), mz.len(), "{sigla}: points");
            assert!(
                (0..mz.len()).all(|j| close(s.mz[j], mz[j]) && close(s.intensity_f64(j), it[j])),
                "{sigla}: values"
            );
        }
    }
}

/// The CV type of the spectra in the mzML: only precursor-ion and neutral-loss scans have one today.
fn check_cv(dir: &Path, sigla: &str, cv: Option<&str>) {
    let mut l = lettori::apri(set(dir, sigla), &Limiti::sito()).unwrap();
    let xml = String::from_utf8(
        mzlab_core::lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap(),
    )
    .unwrap();
    for term in ["MS:1000341", "MS:1000326"] {
        assert_eq!(xml.contains(term), cv == Some(term), "{sigla}: {term}");
    }
}

#[test]
fn every_mode_is_read_or_refused_exactly_as_the_matrix_says() {
    let Some((dir, modi)) = generate() else {
        return;
    };
    let modi = modi.as_object().unwrap();
    assert_eq!(modi.len(), 14);
    let mut read = Vec::new();
    for (sigla, m) in modi {
        for sample in m["campioni"].as_array().unwrap() {
            check(&dir, sigla, sample);
        }
        let today = &m["campioni"][0]["oggi"];
        if today["esito"] == "letto" {
            read.push(sigla.as_str());
            if today["spettri"] != 0 {
                check_cv(&dir, sigla, today["tipo"].as_str());
            }
        }
    }
    // the confirmed codes and the readable modes are the same six
    let confirmed: Vec<&str> = modi
        .iter()
        .filter(|(_, m)| m["codice_confermato"] == true)
        .map(|(s, _)| s.as_str())
        .collect();
    assert_eq!(confirmed, read);
    assert_eq!(read, ["EMS", "EPI", "MRM", "NL", "PREC", "Q1"]);
}

/// The refused modes are refused through the registry too (the page's path), with the same detail.
#[test]
fn the_registry_refuses_the_same_modes_with_the_same_detail() {
    let Some((dir, modi)) = generate() else {
        return;
    };
    for (sigla, m) in modi.as_object().unwrap() {
        let today = &m["campioni"][0]["oggi"];
        if today["esito"] == "rifiuto" {
            let err = lettori::apri(set(&dir, sigla), &Limiti::sito())
                .err()
                .unwrap_or_else(|| panic!("{sigla}: opened"));
            assert_eq!(detail(&err), today["detail"].as_str().unwrap(), "{sigla}");
        }
    }
}

/// The mzML of sample `c` (0-based) of the pair `name`, with the file name replaced so two pairs compare.
fn mzml(dir: &Path, name: &str, c: usize) -> String {
    let mut l = lettori::apri(set(dir, name), &Limiti::sito()).unwrap();
    let xml = String::from_utf8(
        mzlab_core::lettori_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}).unwrap(),
    )
    .unwrap()
    .replace(name, "X");
    // the checksums differ: the two files differ in the scan-type code
    let mut parts: Vec<&str> = xml.split("name=\"SHA-1\" value=\"").collect();
    for p in parts.iter_mut().skip(1) {
        *p = &p[p.find('"').unwrap()..];
    }
    parts.concat()
}

/// Q1 (code 0) is read as an EMS level-1 profile scan, also by the samples that take their method from Method1 (a batch run with one method).
#[test]
fn q1_reads_like_ems_and_the_method_one_fallback_serves_every_sample() {
    let Some((dir, modi)) = generate() else {
        return;
    };
    assert_eq!(modi["Q1"]["campioni"].as_array().unwrap().len(), 3);
    for sample in modi["Q1"]["campioni"].as_array().unwrap() {
        check(&dir, "Q1", sample);
    }
    let ole = container(std::fs::read(dir.join("Q1.wiff")).unwrap(), "Q1").unwrap();
    let mut scan = std::fs::read(dir.join("Q1.wiff.scan")).unwrap();
    for n in 1..=3 {
        let w = Wiff::open(&ole, "Q1", Some(scan.len() as u64), n).unwrap();
        assert_eq!(w.experiments.len(), 1, "sample {n}");
        assert_eq!(w.experiments[0].code, Some(0), "sample {n}");
        assert!(
            !w.variante(),
            "sample {n}: a 3200 QTRAP Q1 scan is not a variant"
        );
        let run = w.run(&mut scan).unwrap();
        assert!(run.spectra.iter().all(|s| s.level == 1 && s.profile));
    }
    // same content as EMS apart from the sample ids: header, spectra and TIC
    let (q1, ems) = (mzml(&dir, "Q1", 0), mzml(&dir, "EMS", 0));
    assert_eq!(q1, ems);
    assert_eq!(mzml(&dir, "Q1", 2).matches("<spectrum ").count(), 4);
    assert!(mzml(&dir, "Q1", 2).contains("sample=3 period=1 cycle=4 experiment=1"));
}

/// Q1MI (code 1) stays refused, the message names the code.
#[test]
fn q1mi_is_still_refused_with_its_code() {
    let Some((dir, _)) = generate() else {
        return;
    };
    let err = lettori::apri(set(&dir, "Q1MI"), &Limiti::sito())
        .err()
        .unwrap();
    assert_eq!(detail(&err), "scan type code 1 (experiment 1)");
}
