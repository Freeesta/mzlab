//! The readers module (feature `estesi`) isolates a broken unit of a `.wiff` pair as the engine does (mzlab-core `wiff_isolation.rs`): the same
//! per-sample result for a sample with every block damaged, a NaN cps, zeroed Idx rows and a sample at position 0; and a `.wiff` whose OLE start is
//! damaged is the file named by the refusal, not its `.wiff.scan`. Pairs of `tools/wiff_isolamento.py`; skipped without python3.
#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::readers::{FileSet, Limits, Losses, Member, Reader, CYCLES, DAMAGED};

fn pairs(tag: &str) -> Option<PathBuf> {
    let out = std::env::temp_dir().join(format!(
        "mzlab_sciex_isolamento_{tag}_{}",
        std::process::id()
    ));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_isolamento.py");
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
        eprintln!("SKIP sciex_isolation: python3 not available");
        return None;
    }
    Some(out)
}

fn set(dir: &Path, name: &str) -> FileSet {
    let member = |file: String| {
        let b = std::fs::read(dir.join(&file)).unwrap();
        Member {
            nome: file,
            dim: b.len() as u64,
            dati: Box::new(b),
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

fn open(dir: &Path, name: &str) -> Box<dyn Reader> {
    mzlab_lettori::apri(set(dir, name), &Limits::desktop()).unwrap()
}

/// The scans (id, rt, points), chromatograms (id, finite values) and losses of a sample.
type Content = (Vec<(String, f64, usize)>, Vec<(String, Vec<f64>)>, Losses);

/// What sample `c` gives: its refusal key, or its `Content`.
fn result(l: &mut dyn Reader, c: usize) -> Result<Content, &'static str> {
    let n = l.index(c).map_err(|e| e.key)?.len();
    let scans = (0..n)
        .map(|k| l.scan(c, k).map(|s| (s.id, s.rt, s.mz.len())))
        .collect::<Result<_, _>>()
        .map_err(|e| e.key)?;
    let ids: Vec<String> = l
        .chromatograms(c)
        .map_err(|e| e.key)?
        .into_iter()
        .map(|e| e.id)
        .collect();
    let traces = ids
        .iter()
        .map(|id| l.chromatogram(c, id).map(|t| (t.id, t.intensity)))
        .collect::<Result<_, _>>()
        .map_err(|e| e.key)?;
    Ok((scans, traces, l.losses(c).map_err(|e| e.key)?))
}

#[test]
fn the_module_refuses_a_damaged_sample_alone_and_reads_the_others_whole() {
    let Some(d) = pairs("guasto") else { return };
    let (mut whole, mut broken) = (open(&d, "sano"), open(&d, "guasto"));
    assert_eq!(result(broken.as_mut(), 0).err(), Some("err.file.wiff"));
    for c in 1..3 {
        let got = result(broken.as_mut(), c).unwrap();
        assert_eq!(got.0.len(), 3);
        assert_eq!(got, result(whole.as_mut(), c).unwrap());
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_module_leaves_out_a_nan_cycle_and_counts_it() {
    let Some(d) = pairs("nan") else { return };
    let (_, traces, lost) = result(open(&d, "mrm_nan").as_mut(), 0).unwrap();
    assert!(
        !traces.is_empty()
            && traces
                .iter()
                .all(|t| t.1.len() == 9 && t.1.iter().all(|x| x.is_finite()))
    );
    assert_eq!(
        (lost.declared, lost.read, lost.cause, lost.unit),
        (10, 9, DAMAGED, CYCLES)
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_module_gives_no_spectrum_from_zeroed_rows_and_keeps_a_sample_at_position_zero() {
    let Some(d) = pairs("zeri") else { return };
    let (scans, _, lost) = result(open(&d, "zeri").as_mut(), 1).unwrap();
    assert!(scans.is_empty());
    assert_eq!((lost.declared, lost.read, lost.cause), (3, 0, DAMAGED));
    let l = open(&d, "vuoto0");
    let ids: Vec<(u32, usize)> = l.samples().iter().map(|s| (s.id, s.scansioni)).collect();
    assert_eq!(ids, [(1, 0), (2, 2)]);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_wiff_with_a_damaged_start_is_the_file_named() {
    let Some(d) = pairs("magia") else { return };
    let mut fs = set(&d, "sano");
    let mut b = std::fs::read(d.join("sano.wiff")).unwrap();
    b[0] = 0;
    fs.membri[0].dati = Box::new(b);
    let e = mzlab_lettori::apri(fs, &Limits::desktop()).err().unwrap();
    let param = |k: &str| e.params.iter().find(|p| p.0 == k).map(|p| p.1.clone());
    assert_eq!(
        (e.key, param("name").as_deref(), param("ext").as_deref()),
        ("err.file.notThisFormat", Some("sano.wiff"), Some(".wiff"))
    );
    let _ = std::fs::remove_dir_all(&d);
}
