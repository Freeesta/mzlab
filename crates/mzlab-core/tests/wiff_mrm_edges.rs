//! The edge rows of a dense MRM run (`tools/wiff_mrm_edges.py`): an empty row (size 0) is left out, so the TIC and every transition keep the same
//! times, and a dense block whose first cps is negative noise is read as dense, never as a sparse head; an MRM or full scan method of two periods is
//! `err.file.wiffPeriods`, never folded into Period0. Engine and readers module (`--features sciex`).
//! Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::Path;
use std::process::Command;

use mzlab_core::mzml::{ChromKind, Chromatogram};
use mzlab_core::wiff::{container, samples, Wiff};

fn generate() -> Option<std::path::PathBuf> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_mrmedges_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_mrm_edges.py");
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
        eprintln!("SKIP wiff_mrm_edges: python3 not available");
        return None;
    }
    Some(out)
}

fn open() -> Option<(Wiff, Vec<u8>)> {
    let out = generate()?;
    let ole = container(
        std::fs::read(out.join("mrmedges.wiff")).unwrap(),
        "mrmedges",
    )
    .unwrap();
    let scan = std::fs::read(out.join("mrmedges.wiff.scan")).unwrap();
    let w = Wiff::open(&ole, "mrmedges", Some(scan.len() as u64), samples(&ole)[0]).unwrap();
    Some((w, scan))
}

fn check(c: &[Chromatogram]) {
    let tic = c.iter().find(|c| c.kind == ChromKind::Tic).expect("TIC");
    assert_eq!(tic.time, vec![0.0, 2.0 / 60.0, 3.0 / 60.0]);
    let srm: Vec<&Chromatogram> = c.iter().filter(|c| c.kind == ChromKind::Srm).collect();
    assert_eq!(srm.len(), 3);
    for s in &srm {
        assert_eq!(s.time, tic.time, "{}", s.id);
    }
    assert_eq!(srm[0].intensity, vec![10.0, -2.5, 60.0]);
    assert_eq!(srm[2].intensity, vec![30.0, 50.0, 80.0]);
}

#[test]
fn an_empty_row_is_left_out_and_negative_noise_stays_dense() {
    let Some((w, mut scan)) = open() else {
        return;
    };
    check(&w.run(&mut scan).unwrap().chromatograms);
    let (c, cycles) = w.chromatograms(&mut scan, None).unwrap();
    check(&c);
    assert_eq!((cycles.read, cycles.damaged), (3, 0));
}

// a method of two periods is refused, whatever its scan type: only Period0 would be read, with the cycles laid out wrong
#[test]
fn a_method_of_two_periods_is_refused_never_folded() {
    let Some(out) = generate() else {
        return;
    };
    for name in ["mrmperiods", "fullperiods"] {
        let ole = container(
            std::fs::read(out.join(format!("{name}.wiff"))).unwrap(),
            name,
        )
        .unwrap();
        let len = std::fs::metadata(out.join(format!("{name}.wiff.scan")))
            .unwrap()
            .len();
        let err = Wiff::open(&ole, name, Some(len), samples(&ole)[0])
            .err()
            .expect(name);
        assert_eq!(err.key, "err.file.wiffPeriods", "{name}");
        assert!(
            err.params.iter().any(|(k, v)| *k == "periods" && v == "2"),
            "{name}"
        );
    }
}
