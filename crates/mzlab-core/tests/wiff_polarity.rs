//! The SRM chromatograms of a `.wiff` say their polarity: in the id (`polarity=`, read back by the channels) and as the scan polarity term of the mzML,
//! from the experiment header (which wins over the ion spray voltage), and nothing when the sample has no method. Checked on `tools/wiff_polarity.py`
//! (an independent writer) through `Wiff::run`, the `Lettore` (what the site reads) and the mzML. Skipped without python3.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::mzml::ChromKind;
use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_polarity_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_polarity.py");
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
        eprintln!("SKIP wiff_polarity: python3 not available");
        return None;
    }
    let json = serde_json::from_slice(&std::fs::read(out.join("polarita.json")).unwrap()).unwrap();
    Some((out, json))
}

fn set(dir: &Path) -> FileSet {
    let member = |file: &str| {
        let bytes = std::fs::read(dir.join(file)).unwrap();
        Member {
            nome: file.into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![member("polarita.wiff"), member("polarita.wiff.scan")],
    }
}

#[test]
fn srm_chromatograms_carry_the_polarity_of_their_experiment() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let ole = container(
        std::fs::read(dir.join("polarita.wiff")).unwrap(),
        "polarita.wiff",
    )
    .unwrap();
    let mut scan = std::fs::read(dir.join("polarita.wiff.scan")).unwrap();
    let mut l = readers::apri(set(&dir), &Limits::sito()).unwrap();
    let width = want["transitions"].as_u64().unwrap() as usize;
    for (c, n) in [1u32, 2, 3].into_iter().enumerate() {
        let pol = want[n.to_string()].as_str();
        let w = Wiff::open(&ole, "polarita.wiff", Some(scan.len() as u64), n).unwrap();
        let srm: Vec<String> = w
            .run(&mut scan)
            .unwrap()
            .chromatograms
            .into_iter()
            .filter(|x| x.kind == ChromKind::Srm)
            .map(|x| x.id)
            .collect();
        assert_eq!(srm.len(), width, "sample {n}");
        for id in &srm {
            let term = id
                .split_whitespace()
                .find_map(|x| x.strip_prefix("polarity="));
            assert_eq!(term, pol, "sample {n}: {id}");
        }
        let site: Vec<String> = l
            .chromatograms(c)
            .unwrap()
            .into_iter()
            .filter(|x| x.tipo == ChromKind::Srm)
            .map(|x| x.id)
            .collect();
        assert_eq!(site, srm, "sample {n}: the site reads the same ids");
        let mut mzml = Vec::new();
        let conv = mzlab_core::readers_mzml::Conversion {
            steps: Some(&[]),
            ..Default::default()
        };
        conv.write(&mut *l, c, &mut mzml, &mut |_, _| {}).unwrap();
        let text = String::from_utf8(mzml).unwrap();
        let count = |acc: &str| {
            text.split("<chromatogram ")
                .skip(1)
                .filter(|x| x.contains("selected reaction monitoring") && x.contains(acc))
                .count()
        };
        let (neg, posi) = (count("\"MS:1000129\""), count("\"MS:1000130\""));
        let expected = match pol {
            Some("negative") => (width, 0),
            Some("positive") => (0, width),
            _ => (0, 0),
        };
        assert_eq!(
            (neg, posi),
            expected,
            "sample {n}: polarity terms of the SRM chromatograms in the mzML"
        );
    }
}
