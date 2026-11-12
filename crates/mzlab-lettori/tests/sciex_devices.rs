//! The device traces of a `.wiff` (`Devices/Device_N/DevData`: column pressure, pump flow, oven temperature) come out of the `sciex` module as chromatograms
//! after the TIC: a pressure in pascal as a `pressure array` (MS:1000821, UO:0000110), any other trace with the unit of the file, a channel without points
//! absent. Oracle: the synthetic pair `dispositivi` of `tools/wiff_riferimento.py` (`tools/wiff_dispositivi.py`), whose values and units are computed there
//! from the inputs; skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).
#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::mzml::{self, ChromKind};
use mzlab_core::mzmlwrite::Options;
use mzlab_core::readers::{FileSet, Limits, Member};
use mzlab_core::readers_mzml::Conversion;
use serde_json::Value;

fn fixture() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_sciex_devices_{}", std::process::id()));
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
        eprintln!("SKIP sciex_devices: python3 not available");
        return None;
    }
    let json: Value =
        serde_json::from_slice(&std::fs::read(out.join("riferimento.json")).unwrap()).unwrap();
    Some((out, json["dispositivi"].clone()))
}

fn member(p: &Path, name: &str) -> Member {
    let b = std::fs::read(p.join(name)).unwrap();
    Member {
        nome: name.into(),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    }
}

fn convert(dir: &Path, stem: &str) -> Vec<u8> {
    let fs = FileSet {
        membri: vec![
            member(dir, &format!("{stem}.wiff")),
            member(dir, &format!("{stem}.wiff.scan")),
        ],
    };
    let mut l = mzlab_lettori::apri(fs, &Limits::sito()).unwrap();
    let conv = Conversion {
        steps: Some(&[]),
        opz: Options {
            thread: 1,
            ..Options::default()
        },
        ..Conversion::default()
    };
    conv.write(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap()
}

fn close(a: &[f64], b: &[Value]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            (x - y.as_f64().unwrap()).abs() <= 1e-9 * y.as_f64().unwrap().abs().max(1.0)
        })
}

#[test]
fn the_device_traces_are_chromatograms_with_their_unit() {
    let Some((dir, want)) = fixture() else { return };
    let bytes = convert(&dir, "dispositivi");
    mzlab_core::mzml_validate::check_written(&bytes[..]).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert_eq!(
        text.matches(r#"accession="MS:1000821" name="pressure array" value="" unitCvRef="UO" unitAccession="UO:0000110" unitName="pascal""#)
            .count(),
        2
    );
    let run = mzml::read(&bytes[..]).unwrap();
    let got: Vec<_> = run
        .chromatograms
        .iter()
        .filter(|c| matches!(c.kind, ChromKind::Pressure | ChromKind::Analog))
        .collect();
    let want = want.as_array().unwrap();
    assert_eq!(
        got.len(),
        want.len(),
        "the channel without points is absent"
    );
    for (c, w) in got.iter().zip(want) {
        let kind = match c.kind {
            ChromKind::Pressure => "pressure",
            _ => "analog",
        };
        assert_eq!(
            (c.id.as_str(), kind, c.unit.as_str()),
            (
                w["id"].as_str().unwrap(),
                w["kind"].as_str().unwrap(),
                w["unit"].as_str().unwrap()
            )
        );
        assert!(
            close(&c.time, w["time"].as_array().unwrap()),
            "{}: time",
            c.id
        );
        assert!(
            close(&c.intensity, w["intensity"].as_array().unwrap()),
            "{}: values",
            c.id
        );
    }
    let tic = run
        .chromatograms
        .iter()
        .position(|c| c.kind == ChromKind::Tic)
        .unwrap();
    let first = run
        .chromatograms
        .iter()
        .position(|c| c.kind == ChromKind::Pressure)
        .unwrap();
    assert!(tic < first, "the traces follow the TIC");
}

#[test]
fn a_pair_without_devices_lists_no_trace() {
    let Some((dir, _)) = fixture() else { return };
    let run = mzml::read(&convert(&dir, "dadmetodo")[..]).unwrap();
    assert!(run
        .chromatograms
        .iter()
        .all(|c| !matches!(c.kind, ChromKind::Pressure | ChromKind::Analog)));
}
