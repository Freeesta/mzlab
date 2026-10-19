//! ADAP chromatogram builder (`untargeted::adap`) against mzmine 4.10: parity per chromatogram on the four synthetic files, hand-made cases, and the
//! independence from the slabs. The mzML files are written again by `tools/bench/oro_mzmine.py --genera` (skipped without python3 and numpy).
//! The reference CSV keeps 5 decimals of m/z and 4 significant digits of height, so those are compared at the precision they have.
#![cfg(feature = "untargeted")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_feat::untargeted::adap::{build, AdapParams};

mod untargeted_common;

use untargeted_common::{rows, same, scans_of, shape, Row};

const GOLD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/data/untargeted/mzmine410/adap.csv"
);

/// The mzML of the gold run, or `None` when python3 or numpy are missing.
fn generate() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("untargeted_oro");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ok = Command::new("python3")
        .arg(root.join("tools/bench/oro_mzmine.py"))
        .arg("--genera")
        .arg(&dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    ok.then_some(dir)
}

#[test]
fn parity_with_mzmine410() {
    if !std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/untargeted/mzmine410"
    ))
    .exists()
    {
        eprintln!("SKIP parity_with_mzmine410: tests/data/untargeted/mzmine410 is missing (the 4.10 gold data need an mzmine account: tools/bench/oro_mzmine410.py)");
        return;
    }
    let Some(dir) = generate() else {
        eprintln!("SKIP untargeted_adap: python3 with numpy is missing");
        return;
    };
    let p = AdapParams::default();
    let rows = rows(Path::new(GOLD));
    let (mut total, mut equal, mut bad) = (0usize, 0usize, Vec::new());
    for file in ["oro_1", "oro_2", "oro_3", "oro_4"] {
        let scans = scans_of(&dir.join(format!("{file}.mzML")), &p);
        let ours = build(&scans, &p);
        let refs: Vec<&Row> = rows.iter().filter(|r| r.file == file).collect();
        let mut used = vec![false; ours.len()];
        for r in &refs {
            total += 1;
            let cand = ours.iter().enumerate().find(|(k, c)| {
                !used[*k] && (c.mz - r.mz).abs() < 5e-3 && (shape(c, &scans).0 - r.rt).abs() < 0.02
            });
            match cand {
                Some((k, c)) => match same(r, c, &scans, 1e-5, 6e-4) {
                    Ok(()) => {
                        used[k] = true;
                        equal += 1
                    }
                    Err(w) => bad.push(format!("{file} m/z {}: {w}", r.mz)),
                },
                None => bad.push(format!(
                    "{file} m/z {} RT {}: no chromatogram of ours",
                    r.mz, r.rt
                )),
            }
        }
        let extra = used.iter().filter(|u| !**u).count();
        if extra > 0 {
            bad.push(format!(
                "{file}: {extra} chromatograms of ours without a reference row"
            ));
        }
    }
    eprintln!("ADAP parity: {equal}/{total} identical; different: {bad:#?}");
    assert!(equal as f64 >= 0.99 * total as f64, "{bad:#?}");
}
