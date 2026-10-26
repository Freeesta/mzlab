//! Several MRM experiments in one sample, one of them scheduled (sMRM): one TIC per experiment, the target RT and window in the ids of the scheduled
//! transitions, and the optional trimming to the window (`programmati` of `tools/wiff_riferimento.py`). Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

#![cfg(feature = "sciex")]

use std::path::Path;
use std::process::Command;

use mzlab_core::mzml::{ChromKind, Chromatogram};
use mzlab_core::wiff::{container, samples, Wiff};

fn chromatograms(trim: bool) -> Option<Vec<Chromatogram>> {
    let out = std::env::temp_dir().join(format!(
        "mzlab_wiff_programmati_{}_{trim}",
        std::process::id()
    ));
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
        eprintln!("SKIP wiff_programmati: python3 not available");
        return None;
    }
    let ole = container(
        std::fs::read(out.join("programmati.wiff")).unwrap(),
        "programmati",
    )
    .unwrap();
    let mut scan = std::fs::read(out.join("programmati.wiff.scan")).unwrap();
    let mut w = Wiff::open(
        &ole,
        "programmati",
        Some(scan.len() as u64),
        samples(&ole)[0],
    )
    .unwrap();
    assert_eq!(w.experiments[1].window_s, Some(60));
    assert_eq!(w.experiments[0].window_s, None);
    w.trim_scheduled = trim;
    Some(w.run(&mut scan).unwrap().chromatograms)
}

fn find<'a>(c: &'a [Chromatogram], part: &str) -> &'a Chromatogram {
    c.iter()
        .find(|c| c.id.contains(part))
        .unwrap_or_else(|| panic!("{part}"))
}

#[test]
fn each_experiment_has_its_tic_and_a_scheduled_transition_its_target_and_window() {
    let Some(c) = chromatograms(false) else {
        return;
    };
    let tics: Vec<_> = c.iter().filter(|c| c.kind == ChromKind::Tic).collect();
    assert_eq!(tics.len(), 2);
    assert_eq!(tics[0].id, "TIC sample=1 period=1 experiment=1");
    assert_eq!(tics[1].id, "TIC sample=1 period=1 experiment=2");
    assert_eq!(tics[1].intensity, vec![10.0, 20.0, 30.0, 40.0]);
    let s0 = find(&c, "experiment=2 transition=0");
    assert!(
        s0.id.contains(" rt=0.5 window=60 ") && !s0.id.contains("dwell="),
        "{}",
        s0.id
    );
    assert_eq!(s0.intensity, vec![10.0, 20.0, 5.0]);
    assert_eq!(s0.time.len(), 3);
    assert!(find(&c, "experiment=1 transition=0")
        .id
        .contains("dwell=50"));
}

#[test]
fn trimming_keeps_the_points_inside_the_window_and_is_off_by_default() {
    let Some(c) = chromatograms(true) else { return };
    assert_eq!(
        find(&c, "experiment=2 transition=0").intensity,
        vec![10.0, 20.0]
    );
    assert_eq!(
        find(&c, "experiment=2 transition=2").intensity,
        vec![50.0, 60.0]
    );
    assert_eq!(find(&c, "experiment=1 transition=0").intensity.len(), 4);
}
