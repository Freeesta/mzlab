//! What the `probe` of the `sciex` module tells the page (`motivi`): a scheduled-MRM pair with no Log (`programmati` of `tools/wiff_riferimento.py`) and a pair
//! whose second sample only is a variant (`tools/wiff_misto_pi.py`) are `"variante"`, so the page hands them to the module and the engine never reads their blocks.
//! Synthetic pairs; skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).
#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::lettori::{FileSet, Lettore, Limiti, Membro};

fn genera(tool: &str) -> Option<PathBuf> {
    let out = std::env::temp_dir().join(format!("mzlab_sciex_sonda_{tool}_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools")
        .join(tool);
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
        eprintln!("SKIP sciex_sonda: python3 not available");
        return None;
    }
    Some(out)
}

fn apri(dir: &Path, stem: &str) -> Box<dyn Lettore> {
    let membro = |nome: String| {
        let b = std::fs::read(dir.join(&nome)).unwrap();
        Membro {
            nome,
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }
    };
    let fs = FileSet {
        membri: vec![
            membro(format!("{stem}.wiff")),
            membro(format!("{stem}.wiff.scan")),
        ],
    };
    mzlab_lettori::apri(fs, &Limiti::sito()).unwrap()
}

#[test]
fn a_scheduled_mrm_pair_without_a_log_is_a_variant() {
    let Some(dir) = genera("wiff_riferimento.py") else {
        return;
    };
    let mut l = apri(&dir, "programmati");
    assert!(
        l.variante(0),
        "its sparse MRM blocks are read by the module only"
    );
    assert_eq!(mzlab_lettori::motivi(&mut *l, 0), vec!["variante"]);
}

#[test]
fn a_pair_is_a_variant_when_any_of_its_samples_is() {
    let Some(dir) = genera("wiff_misto_pi.py") else {
        return;
    };
    let mut l = apri(&dir, "mistopi");
    assert_eq!(l.campioni().len(), 2);
    assert!(!l.variante(0) && l.variante(1));
    assert!(mzlab_lettori::motivi(&mut *l, 0).contains(&"variante"));
}
