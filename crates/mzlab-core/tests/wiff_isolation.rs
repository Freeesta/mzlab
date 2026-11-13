//! One broken unit of a `.wiff` pair never takes the healthy ones with it (the engine's reader; the readers module: mzlab-lettori `sciex_isolation.rs`):
//! a sample whose blocks are all damaged is refused alone, a NaN cps leaves out its MRM cycle only, zeroed Idx rows give no spectrum, a sample at
//! position 0 of the sample table stays listed, and every loss is counted. Pairs of `tools/wiff_isolamento.py`; skipped without python3.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::mzml::{Chromatogram, Spectrum};
use mzlab_core::readers::{
    self, FileSet, Limits, Losses, Member, Reader, CYCLES, DAMAGED, TRUNCATED,
};

/// The pairs written into a folder of their own for test `tag` (the tests run at once and each removes its folder).
fn pairs(tag: &str) -> Option<PathBuf> {
    let out = std::env::temp_dir().join(format!(
        "mzlab_wiff_isolamento_{tag}_{}",
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
        eprintln!("SKIP wiff_isolation: python3 not available");
        return None;
    }
    Some(out)
}

fn open(dir: &Path, name: &str) -> Box<dyn Reader> {
    let member = |file: String| {
        let b = std::fs::read(dir.join(&file)).unwrap();
        Member {
            nome: file,
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }
    };
    let fs = FileSet {
        membri: vec![
            member(format!("{name}.wiff")),
            member(format!("{name}.wiff.scan")),
        ],
    };
    readers::apri(fs, &Limits::sito()).unwrap()
}

/// The scans and chromatograms of sample `c`.
fn content(l: &mut dyn Reader, c: usize) -> (Vec<Spectrum>, Vec<Chromatogram>) {
    let n = l.index(c).unwrap().len();
    let scans = (0..n).map(|k| l.scan(c, k).unwrap()).collect();
    let ids: Vec<String> = l
        .chromatograms(c)
        .unwrap()
        .into_iter()
        .map(|e| e.id)
        .collect();
    (
        scans,
        ids.iter()
            .map(|id| l.chromatogram(c, id).unwrap())
            .collect(),
    )
}

#[test]
fn a_sample_with_every_block_damaged_is_refused_alone() {
    let Some(d) = pairs("0") else { return };
    let (mut whole, mut broken) = (open(&d, "sano"), open(&d, "guasto"));
    assert_eq!(broken.samples().len(), 3);
    let e = broken.refused(0).expect("sample 1 is refused");
    assert_eq!(e.key, "err.file.wiff");
    let param = |k: &str| e.params.iter().find(|p| p.0 == k).map(|p| p.1.clone());
    assert_eq!(param("detail").as_deref(), Some("TOF block"));
    assert!(broken.index(0).is_err());
    for c in 1..3 {
        assert!(broken.refused(c).is_none());
        let got = content(broken.as_mut(), c);
        assert_eq!(got.0.len(), 3);
        assert_eq!(got, content(whole.as_mut(), c), "sample {}", c + 1);
        assert_eq!(broken.losses(c).unwrap(), Losses::complete(3));
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_nan_cps_leaves_out_its_cycle_only_and_counts_it() {
    let Some(d) = pairs("1") else { return };
    let (mut whole, mut broken) = (open(&d, "mrm"), open(&d, "mrm_nan"));
    let (_, traces) = content(broken.as_mut(), 0);
    let (_, want) = content(whole.as_mut(), 0);
    assert_eq!(traces.len(), want.len());
    for (t, w) in traces.iter().zip(&want) {
        assert!(
            t.intensity.iter().chain(&t.time).all(|x| x.is_finite()),
            "{}",
            t.id
        );
        // cycle 5 (0.0417 min) is the one left out
        let keep: Vec<usize> = (0..10).filter(|&k| k != 4).collect();
        assert_eq!(
            t.time,
            keep.iter().map(|&k| w.time[k]).collect::<Vec<_>>(),
            "{}",
            t.id
        );
        assert_eq!(
            t.intensity,
            keep.iter().map(|&k| w.intensity[k]).collect::<Vec<_>>(),
            "{}",
            t.id
        );
    }
    let l = broken.losses(0).unwrap();
    assert_eq!(
        (l.declared, l.read, l.cause, l.unit),
        (10, 9, DAMAGED, CYCLES)
    );
    assert_eq!(content(broken.as_mut(), 1), content(whole.as_mut(), 1));
    assert_eq!(broken.losses(1).unwrap(), Losses::complete(0));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_cut_mrm_sample_counts_its_cycles() {
    let Some(d) = pairs("2") else { return };
    let mut l = open(&d, "mrm_taglio");
    let lost = l.losses(0).unwrap();
    assert_eq!(
        (lost.declared, lost.read, lost.cause, lost.unit),
        (10, 6, TRUNCATED, CYCLES)
    );
    let r = lost.report(Path::new("x.wiff")).unwrap();
    assert!(r.contains("6 of 10 MRM cycles read"), "{r}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn zeroed_index_rows_give_no_spectrum_and_are_counted() {
    let Some(d) = pairs("3") else { return };
    let mut l = open(&d, "zeri");
    assert_eq!(l.index(0).unwrap().len(), 2);
    assert!(
        l.index(1).unwrap().is_empty(),
        "no spectrum from a zeroed row"
    );
    let lost = l.losses(1).unwrap();
    assert_eq!((lost.declared, lost.read, lost.cause), (3, 0, DAMAGED));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_sample_at_position_zero_of_the_table_stays_listed_empty() {
    let Some(d) = pairs("4") else { return };
    let mut l = open(&d, "vuoto0");
    let ids: Vec<(u32, usize)> = l.samples().iter().map(|s| (s.id, s.scansioni)).collect();
    assert_eq!(ids, [(1, 0), (2, 2)]);
    assert!(l.index(0).unwrap().is_empty() && l.chromatograms(0).unwrap().is_empty());
    assert_eq!(l.losses(0).unwrap(), Losses::complete(0));
    assert_eq!(l.index(1).unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_container_past_a_fixed_limit_is_told_as_such() {
    let Some(d) = pairs("5") else { return };
    let b = std::fs::read(d.join("sano.wiff")).unwrap();
    let limits = Limits {
        byte: 1024,
        ..Limits::sito()
    };
    let e = mzlab_core::wiff::container_within(b, "sano.wiff", &limits)
        .err()
        .unwrap();
    assert_eq!(e.key, "error.fixedLimit");
    let _ = std::fs::remove_dir_all(&d);
}
