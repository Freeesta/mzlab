//! A `.wiff` pair cut short at every length (`one` and `two` of `tools/wiff_riferimento.py`) gives either one keyed error or the scans it holds whole:
//! a block cut by the end of the `.wiff.scan` is counted by `missing`, never read short, and the engine (`Wiff::run`) gives the same spectra as the reader.
//! Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::mzml::Spectrum;

/// One folder per test: the tests run in parallel and the script rewrites its folder.
fn reference(tag: &str) -> Option<PathBuf> {
    let out =
        std::env::temp_dir().join(format!("mzlab_wiff_troncato_{}_{tag}", std::process::id()));
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
        eprintln!("SKIP wiff_troncato: python3 not available");
        return None;
    }
    Some(out)
}

fn pair(wiff: &[u8], scan: &[u8]) -> FileSet {
    let member = |nome: &str, b: &[u8]| Membro {
        nome: nome.into(),
        dim: b.len() as u64,
        dati: Box::new(b.to_vec()),
        sha1: None,
    };
    FileSet {
        membri: vec![member("x.wiff", wiff), member("x.wiff.scan", scan)],
    }
}

/// The scans of every sample (`Err` = the key of the error at open) and how many blocks each sample misses.
fn read(wiff: &[u8], scan: &[u8]) -> Result<Vec<(Vec<Spectrum>, usize)>, &'static str> {
    let mut l = lettori::apri(pair(wiff, scan), &Limiti::sito()).map_err(|e| e.key)?;
    let mut out = Vec::new();
    for c in 0..l.samples().len() {
        let n = l.index(c).map_err(|e| e.key)?.len();
        let scans = (0..n)
            .map(|k| l.scan(c, k))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.key)?;
        out.push((scans, l.missing(c)));
    }
    Ok(out)
}

/// The spectra of the engine route for every sample.
fn engine(wiff: &[u8], scan: &[u8]) -> Vec<Vec<Spectrum>> {
    let ole = mzlab_core::wiff::container(wiff.to_vec(), "x.wiff").unwrap();
    let mut scan = scan.to_vec();
    let len = scan.len() as u64;
    mzlab_core::wiff::samples(&ole)
        .into_iter()
        .map(|s| {
            mzlab_core::wiff::Wiff::open(&ole, "x.wiff", Some(len), s)
                .unwrap()
                .run(&mut scan)
                .unwrap()
                .spectra
        })
        .collect()
}

fn same(a: &Spectrum, b: &Spectrum) -> bool {
    a.id == b.id && a.mz == b.mz && a.intensity == b.intensity
}

#[test]
fn a_pair_cut_short_gives_a_keyed_error_or_whole_scans_and_the_missing_count() {
    let Some(dir) = reference("scan") else { return };
    for name in ["one", "two"] {
        let wiff = std::fs::read(dir.join(format!("{name}.wiff"))).unwrap();
        let scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
        let full = read(&wiff, &scan).unwrap();
        assert!(
            full.iter().all(|(_, m)| *m == 0),
            "{name}: the whole pair misses nothing"
        );
        for cut in 0..scan.len() {
            let got = read(&wiff, &scan[..cut]);
            let Ok(got) = got else {
                let key = got.unwrap_err();
                assert!(
                    key.starts_with("err."),
                    "{name}.wiff.scan cut at {cut}: {key}"
                );
                continue;
            };
            assert_eq!(
                got.len(),
                full.len(),
                "{name}.wiff.scan cut at {cut}: samples"
            );
            for (c, ((have, missing), (want, _))) in got.iter().zip(&full).enumerate() {
                // every scan given is whole and in place; a scan left out is counted
                assert!(have.len() <= want.len());
                assert!(
                    have.iter().zip(want).all(|(a, b)| same(a, b)),
                    "{name} cut at {cut}: sample {c} has a scan read short"
                );
                assert!(
                    have.len() == want.len() || *missing > 0,
                    "{name} cut at {cut}: sample {c} lost a scan unsaid"
                );
            }
            let eng = engine(&wiff, &scan[..cut]);
            for (c, (e, (have, _))) in eng.iter().zip(&got).enumerate() {
                assert!(
                    e.len() == have.len() && e.iter().zip(have).all(|(a, b)| same(a, b)),
                    "{name} cut at {cut}: engine and reader differ on sample {c}"
                );
            }
        }
    }
}

#[test]
fn a_wiff_cut_short_is_refused_with_its_own_notice() {
    let Some(dir) = reference("wiff") else { return };
    let wiff = std::fs::read(dir.join("one.wiff")).unwrap();
    let scan = std::fs::read(dir.join("one.wiff.scan")).unwrap();
    let mut told = 0;
    // whole sectors of the container missing: the samples would read as none
    for cut in (512..wiff.len() - 512).step_by(64) {
        match read(&wiff[..cut], &scan) {
            Err("err.file.wiffTruncated") => told += 1,
            Err(key) => assert!(key.starts_with("err."), "one.wiff cut at {cut}: {key}"),
            Ok(s) => assert!(
                !s.is_empty(),
                "one.wiff cut at {cut}: opened with no sample and no notice"
            ),
        }
    }
    assert!(told > 0, "no cut of the container told it is cut short");
}
