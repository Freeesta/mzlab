//! ADAP builder against the ADAP lists of mzmine 3.9.0 on real files of the private series (HR, `dati/HRMS/Timolol/`). Skipped without the data repository
//! (`MZLAB_DATI`) or without the lists `coordinamento/mzmine39/adap_<file>.csv` (columns of `tests/data/untargeted/mzmine39/adap.csv`, written by
//! `python3 tools/bench/oro_mzmine.py --reale HRMS_DIR DEST`). Threshold: >= 0.97 of the rows of mzmine matched within 1e-4 m/z.
#![cfg(feature = "untargeted")]

mod untargeted_common;

use std::path::PathBuf;
use std::time::Instant;

use mzlab_feat::untargeted::adap::{build, AdapParams};
use untargeted_common::{rows, same, scans_of, shape};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

/// Peak resident memory of the process in MB (Linux), 0 elsewhere.
fn peak_mb() -> f64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        .unwrap_or(0.0)
        / 1024.0
}

#[test]
fn parity_with_mzmine39_on_real_files() {
    let Some(lists) = dati_percorsi::percorso("coordinamento/mzmine39") else {
        eprintln!("SKIP untargeted_adap_dati: no data repository (MZLAB_DATI) or no ADAP list of mzmine in coordinamento/mzmine39");
        return;
    };
    let mut files: Vec<PathBuf> = dati_percorsi::percorso("dati/HRMS/Timolol")
        .into_iter()
        .flat_map(|t| std::fs::read_dir(t).into_iter().flatten().flatten())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "mzML"))
        .collect();
    files.sort();
    files.retain(|f| {
        lists
            .join(format!(
                "adap_{}.csv",
                f.file_stem().unwrap().to_string_lossy()
            ))
            .is_file()
    });
    if files.is_empty() {
        eprintln!("SKIP untargeted_adap_dati: no ADAP list of mzmine in coordinamento/mzmine39");
        return;
    }
    let p = AdapParams::default();
    for f in files.iter().take(2) {
        let stem = f.file_stem().unwrap().to_string_lossy().to_string();
        let t = Instant::now();
        let scans = scans_of(f, &p);
        let ours = build(&scans, &p);
        let secs = t.elapsed().as_secs_f64();
        let refs = rows(&lists.join(format!("adap_{stem}.csv")));
        let mut by_mz: Vec<(usize, f64)> =
            ours.iter().enumerate().map(|(k, c)| (k, c.mz)).collect();
        by_mz.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut matched = 0usize;
        for r in &refs {
            let lo = by_mz.partition_point(|x| x.1 < r.mz - 2e-4);
            let hit = by_mz[lo..]
                .iter()
                .take_while(|x| x.1 <= r.mz + 2e-4)
                .any(|(k, _)| {
                    (shape(&ours[*k], &scans).0 - r.rt).abs() < 1e-3
                        && same(r, &ours[*k], &scans, 1e-4, 1e-3).is_ok()
                });
            matched += hit as usize;
        }
        eprintln!("ADAP real {stem}: {matched}/{} rows of mzmine matched, {} chromatograms of ours, {secs:.1} s, peak memory {:.0} MB", refs.len(), ours.len(), peak_mb());
        assert!(
            matched as f64 >= 0.97 * refs.len() as f64,
            "{stem}: {matched}/{}",
            refs.len()
        );
    }
}
