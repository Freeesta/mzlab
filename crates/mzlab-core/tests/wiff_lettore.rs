//! The `.wiff` pair read through the `Lettore` contract gives the same spectra and chromatograms as `Wiff::run`, and is recognised by the registry.
//! Synthetic pairs need python3 (`tools/wiff_riferimento.py`), the real ones the data repository (`MZLAB_DATI`); both skip when missing.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::wiff::{container, samples, Wiff};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn set(dir: &Path, name: &str) -> FileSet {
    let member = |file: String| {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        Membro {
            nome: file,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
        }
    };
    FileSet {
        membri: vec![
            member(format!("{name}.wiff")),
            member(format!("{name}.wiff.scan")),
        ],
    }
}

fn same(dir: &Path, name: &str) -> usize {
    let mut l = lettori::apri(set(dir, name), &Limiti::sito()).unwrap();
    assert_eq!(l.formato().id(), "sciex-wiff");
    let ole = container(
        std::fs::read(dir.join(format!("{name}.wiff"))).unwrap(),
        name,
    )
    .unwrap();
    let mut scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
    let ids = samples(&ole);
    assert_eq!(l.campioni().len(), ids.len(), "{name}: samples");
    let mut checked = 0;
    for (c, s) in ids.into_iter().enumerate() {
        let w = Wiff::open(&ole, name, Some(scan.len() as u64), s).unwrap();
        let run = w.run(&mut scan).unwrap();
        let index = l.indice(c).unwrap().to_vec();
        assert_eq!(index.len(), run.spectra.len(), "{name}/{s}: index");
        for (k, want) in run.spectra.iter().enumerate() {
            assert_eq!(
                (index[k].id.as_str(), index[k].rt, index[k].livello),
                (want.id.as_str(), want.rt, want.level),
                "{name}/{s} row {k}"
            );
            assert!(index[k]
                .posizione
                .is_some_and(|p| p.membro == 1 && p.lunghezza > 0));
            let got = l.scansione(c, k).unwrap();
            assert_eq!(
                (
                    &got.id,
                    got.level,
                    got.polarity,
                    got.profile,
                    got.precursor,
                    got.rt
                ),
                (
                    &want.id,
                    want.level,
                    want.polarity,
                    want.profile,
                    want.precursor,
                    want.rt
                )
            );
            assert_eq!(
                (&got.mz, &got.wide),
                (&want.mz, &want.wide),
                "{name}/{s}: {}",
                want.id
            );
            checked += 1;
        }
        let listed = l.cromatogrammi(c).unwrap();
        assert_eq!(
            listed.len(),
            run.chromatograms.len(),
            "{name}/{s}: chromatograms"
        );
        for want in &run.chromatograms {
            assert_eq!(&l.cromatogramma(c, &want.id).unwrap(), want);
        }
        let mut direct = Vec::new();
        w.to_mzml(&mut scan, &mut direct, &format!("{name}.wiff"))
            .unwrap();
        let via = mzlab_core::lettori_mzml::scrivi(&mut *l, c, Vec::new(), &mut |_, _| {}).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&via),
            String::from_utf8_lossy(&direct),
            "{name}/{s}: mzML"
        );
        let doc = l.metodo(c).unwrap();
        assert!(
            doc.contains("sciex-wiff") && doc.contains(&format!("{name}.wiff")),
            "{doc}"
        );
    }
    checked
}

fn dati() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    for up in root().canonicalize().ok()?.ancestors().skip(1).take(3) {
        cands.push(up.join("mzlab-dati"));
    }
    cands.into_iter().find(|p| p.join("3200 qtrap").is_dir())
}

#[test]
fn the_synthetic_pairs_read_the_same_through_the_registry() {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_lettore_{}", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/wiff_riferimento.py"))
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_lettore: python3 not available");
        return;
    }
    let n: usize = ["one", "two", "serie"]
        .iter()
        .map(|name| same(&out, name))
        .sum();
    assert!(n > 0);
}

#[test]
fn the_real_files_read_the_same_through_the_registry() {
    let Some(d) = dati() else {
        eprintln!("SKIP wiff_lettore: no data repository (MZLAB_DATI)");
        return;
    };
    let folder = d.join("3200 qtrap/wiff inquinanti 25-26 B");
    let n: usize = ["FullMass", "MS2", "MRM"]
        .iter()
        .filter(|n| folder.join(format!("{n}.wiff")).exists())
        .map(|n| same(&folder, n))
        .sum();
    assert!(n > 0);
}

#[test]
fn a_wiff_alone_is_not_sure_and_not_a_wiff_is_refused() {
    let wiff = |bytes: Vec<u8>, name: &str| FileSet {
        membri: vec![Membro {
            nome: name.into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
        }],
    };
    let mut ole_head = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    ole_head.resize(600, 0);
    let mut alone = wiff(ole_head, "x.wiff");
    assert_eq!(
        lettori::riconosci(&mut alone).map(|v| v.id),
        Some("sciex-wiff")
    );
    assert_eq!(
        lettori::apri(alone, &Limiti::sito()).err().unwrap().key,
        "err.file.wiffNoScan"
    );
    let mut text = wiff(b"hello world, not an OLE container".to_vec(), "x.wiff");
    assert!(lettori::riconosci(&mut text).is_none());
}
