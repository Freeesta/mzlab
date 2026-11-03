//! A `.wiff` the site sends to the `sciex` module (3200 QTRAP, version-1 blocks) keeps its optical detector and its LC method in the converted mzML:
//! the engine answers `/api/uvspec` and `/api/chrom?kind=pda` from the file and the method (with the gradient) comes back from `mzLab metodo/1`.
//! Synthetic pair of `tools/wiff_riferimento.py` (`dadmetodo`); skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).
#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::itemroutes::{pda_spectrum, pda_wavelength};
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::lettori_mzml::Conversione;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::mzmlwrite::Opzioni;
use serde_json::Value;

fn fixture() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_sciex_dad_metodo_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
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
        eprintln!("SKIP sciex_dad_metodo: python3 not available");
        return None;
    }
    let json: Value =
        serde_json::from_slice(&std::fs::read(out.join("riferimento.json")).unwrap()).unwrap();
    Some((out, json["dadmetodo"].clone()))
}

fn membro(p: &Path, nome: &str) -> Membro {
    let b = std::fs::read(p.join(nome)).unwrap();
    Membro {
        nome: nome.into(),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    }
}

/// The mzML of the pair as `to_mzml` of the module writes it (indexed, no steps, the matrix on).
fn converti(dir: &Path, dad: bool) -> Vec<u8> {
    let fs = FileSet {
        membri: vec![
            membro(dir, "dadmetodo.wiff"),
            membro(dir, "dadmetodo.wiff.scan"),
        ],
    };
    let mut l = mzlab_lettori::apri(fs, &Limiti::sito()).unwrap();
    assert!(
        l.variante(0),
        "version-1 blocks make the pair a variant of the module"
    );
    let conv = Conversione {
        steps: Some(&[]),
        opz: Opzioni {
            dad,
            thread: 1,
            ..Opzioni::default()
        },
        ..Conversione::default()
    };
    conv.scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap()
}

#[test]
fn the_variant_mzml_carries_the_matrix_and_the_method() {
    let Some((dir, want)) = fixture() else { return };
    let bytes = converti(&dir, true);
    let n = bytes.len() as u64;
    let mut m = Mzml::new(bytes, n);
    let dad = m.dad().unwrap().expect("the optical matrix is in the mzML");
    assert_eq!(dad.points(), want["points"].as_u64().unwrap() as usize);
    assert_eq!(
        dad.wavelengths_nm.len(),
        want["nm"][2].as_u64().unwrap() as usize
    );
    assert_eq!(dad.wavelengths_nm[0], want["nm"][0].as_f64().unwrap());
    let spec: Value = serde_json::from_str(&pda_spectrum(&mut m, 0.75, 0.85).unwrap()).unwrap();
    assert!(spec.is_object(), "{spec}");
    let trace: Value =
        serde_json::from_str(&pda_wavelength(&mut m, 250.0, 260.0).unwrap()).unwrap();
    assert!(
        trace["rt"]
            .as_array()
            .is_some_and(|r| r.len() == dad.points()),
        "{trace}"
    );

    let doc: Value = serde_json::from_str(
        &m.metodo("dadmetodo.mzML", 1 << 20)
            .unwrap()
            .expect("metodo/1 in the run"),
    )
    .unwrap();
    assert_eq!(doc["schema"], "metodo/1");
    let grad = doc["lc"]["gradiente"]
        .as_array()
        .expect("the gradient of the time program");
    let got: Vec<(f64, f64)> = grad
        .iter()
        .map(|p| (p["t_min"].as_f64().unwrap(), p["B_pct"].as_f64().unwrap()))
        .collect();
    let exp: Vec<(f64, f64)> = want["gradient"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
        .collect();
    assert_eq!(got, exp);
}

#[test]
fn without_the_option_the_mzml_has_no_matrix_but_keeps_the_method() {
    let Some((dir, _)) = fixture() else { return };
    let bytes = converti(&dir, false);
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("MS:1000804"));
    assert!(text.contains("name=\"mzLab metodo/1\""));
}
