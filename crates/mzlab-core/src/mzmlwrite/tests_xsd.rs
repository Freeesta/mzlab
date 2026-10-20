//! The indexed file against the PSI schema `mzML1.1.2_idx.xsd` (`xmllint`). The schema files are fetched once into a cache folder; without `xmllint`,
//! `curl` or network the test says so and passes, so only a machine that can check really checks.
use super::tests_indice::{indexed_header, indexed_sample};
use crate::mzmlwrite::Header;
use crate::mzmlwrite::ProcStep;
use std::path::PathBuf;
use std::process::Command;

const BASE: &str = "https://raw.githubusercontent.com/HUPO-PSI/mzML/master/schema/schema_1.1/";

/// The folder with `mzML1.1.2_idx.xsd` and `mzML1.1.0.xsd`, or why it cannot be had.
fn schemas() -> Result<PathBuf, String> {
    let dir = std::env::var_os("MZLAB_XSD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("mzlab-psi-xsd"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for n in ["mzML1.1.2_idx.xsd", "mzML1.1.0.xsd"] {
        let f = dir.join(n);
        if std::fs::metadata(&f).map_or(true, |m| m.len() < 1000) {
            let ok = Command::new("curl")
                .args(["-fsS", "-m", "30", "-o"])
                .arg(&f)
                .arg(format!("{BASE}{n}"))
                .status()
                .map_err(|e| format!("curl: {e}"))?
                .success();
            if !ok {
                return Err(format!("cannot fetch {n}"));
            }
        }
    }
    Ok(dir)
}

#[test]
fn the_indexed_file_validates_against_the_psi_schema() {
    let dir = match schemas() {
        Ok(d) => d,
        Err(e) => return eprintln!("skipped: {e}"),
    };
    let step = ProcStep {
        accession: "MS:1000035".into(),
        name: "peak picking".into(),
        software: "features.detect@1".into(),
        version: "1".into(),
        params: vec![("min_height".into(), "1000".into())],
    };
    // a header with no source file and no component, which must leave out both lists
    let bare = indexed_header(Header {
        run: "x".into(),
        ..Default::default()
    })
    .1;
    let full = [Vec::new(), vec![step]].map(|steps| indexed_sample(steps).1);
    for idx in std::iter::once(bare).chain(full) {
        let file = dir.join(format!("sample-{}.mzML", std::process::id()));
        std::fs::write(&file, &idx).unwrap();
        let out = Command::new("xmllint")
            .args(["--noout", "--huge", "--schema"])
            .arg(dir.join("mzML1.1.2_idx.xsd"))
            .arg(&file)
            .output();
        std::fs::remove_file(&file).ok();
        match out {
            Err(e) => return eprintln!("skipped: xmllint: {e}"),
            Ok(o) => assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr)),
        }
    }
}
