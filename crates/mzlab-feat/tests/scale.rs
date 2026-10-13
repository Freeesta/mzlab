//! Scale: 50 files of 80 000 features each (the same 80 000 ions seen in every file, with a small drift of RT and m/z) are aligned in both modes.
//! The aligned table (80 000 rows x 50 files) stays under 100 MB; the working memory of the alignment per family (the larger one) is measured on
//! the peak of the process (`VmHWM`, Linux) above what the input tables already hold.

use mzlab_feat::align::{align, AlignParams};
use mzlab_feat::famalign::{align_families, FamAlignParams};
use mzlab_feat::table::{FileFeatures, FLAG_GOOD, ROLE_M};

const FILES: usize = 50;
/// Features per file: the full size runs with `cargo test --release -p mzlab-feat --test scale -- --ignored`.
const IONS: usize = 80_000;
const SMALL: usize = 4_000;

fn file(k: usize, ions: usize) -> FileFeatures {
    let mut f = FileFeatures::default();
    for i in 0..ions {
        // ions 0.01 apart in m/z: far from each other in ppm, each with its own RT (a prime step spreads them over 30 minutes)
        let mz = 100.0 + i as f64 * 0.01 + (k as f64 % 3.0) * 1e-6;
        let rt = (i * 7919 % 30_000) as f32 / 1000.0 + 0.002 * k as f32;
        f.mz.push(mz);
        f.rt.push(rt);
        f.left.push(rt - 0.05);
        f.right.push(rt + 0.05);
        f.area.push(1e6 + i as f64);
        f.height.push(5e5);
        f.flags.push(FLAG_GOOD);
        f.family_id.push(i as u32 + 1);
        f.role.push(ROLE_M);
        f.charge.push(1);
    }
    f
}

fn peak_rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(0.0, |kb| kb / 1024.0)
}

/// Aligns `FILES` files of `ions` features in both modes; returns the peak of memory of the alignment per family above the inputs (MB).
fn run(ions: usize) -> f64 {
    let files: Vec<FileFeatures> = (0..FILES).map(|k| file(k, ions)).collect();
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let before = peak_rss_mb();
    let fam = align_families(&refs, &FamAlignParams::default()).unwrap();
    let peak_fam = peak_rss_mb() - before;
    assert_eq!(fam.len(), ions);
    assert!(
        fam.heap_bytes() <= ions * FILES * 21 + ions * 40,
        "{} bytes",
        fam.heap_bytes()
    );
    drop(fam);
    let ion = align(&refs, &AlignParams::default()).unwrap();
    assert_eq!(ion.len(), ions);
    assert!(
        ion.heap_bytes() <= ions * FILES * 21 + ions * 16,
        "{} bytes",
        ion.heap_bytes()
    );
    eprintln!(
        "table {:.1} MB; peak of the alignment per family above the inputs {peak_fam:.1} MB",
        ion.heap_bytes() as f64 / 1048576.0
    );
    peak_fam
}

#[test]
fn fifty_files_small() {
    run(SMALL);
}

#[test]
#[ignore = "50 files of 80 000 features: run with --release"]
fn fifty_files_of_eighty_thousand_features() {
    let peak = run(IONS);
    if peak > 0.0 {
        assert!(
            peak < 100.0,
            "the alignment per family took {peak:.1} MB above the inputs"
        );
    }
}
