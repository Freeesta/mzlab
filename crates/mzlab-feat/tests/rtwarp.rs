//! RT correction between files (`rtwarp`) on a synthetic series with a known drift: the RT of every file moves by an amount larger than the RT gap
//! of the alignment, so without correction the compounds split in several rows; with piecewise, LOESS and RANSAC every compound is one row seen in
//! all files, and the curve of a file follows the drift put in. RANSAC also holds when some anchors have a wrong RT.

use mzlab_feat::famalign::{align_ions, FamAlignParams};
use mzlab_feat::rtmap::RtParams;
use mzlab_feat::rtwarp::{warp_map, Method, WarpParams};
use mzlab_feat::table::{Aligned, FileFeatures, CELL_DETECTED, FLAG_GOOD};

const FILES: usize = 6;
const COMPOUNDS: usize = 150;

fn mz(c: usize) -> f64 {
    200.0 + 3.1 * c as f64
}

fn rt0(c: usize) -> f64 {
    1.0 + 18.0 * ((c * 37) % COMPOUNDS) as f64 / COMPOUNDS as f64
}

/// Drift of file `k` at the true RT `t`: a shift per file plus a slow wave, 0.15 min per file, 0.75 over the series (the gap of the alignment is 0.1).
fn drift(k: usize, t: f64) -> f64 {
    0.15 * k as f64 + 0.01 * k as f64 * (t / 2.5).sin()
}

/// A line only, for the RANSAC test.
fn line_drift(k: usize, t: f64) -> f64 {
    k as f64 * (0.03 + 0.004 * t)
}

fn file(k: usize, d: fn(usize, f64) -> f64, wrong: &dyn Fn(usize, usize) -> bool) -> FileFeatures {
    let mut ions: Vec<(f64, f32)> = (0..COMPOUNDS)
        .map(|c| {
            let t = rt0(c);
            let shift = if wrong(c, k) { 0.35 } else { d(k, t) };
            (mz(c), (t + shift) as f32)
        })
        .collect();
    ions.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut f = FileFeatures::default();
    for (m, rt) in ions {
        f.mz.push(m);
        f.rt.push(rt);
        f.left.push(rt - 0.05);
        f.right.push(rt + 0.05);
        f.area.push(1e7);
        f.height.push(5e6);
        f.flags.push(FLAG_GOOD);
        f.role.push(0);
        f.charge.push(1);
        f.family_id.push(0);
    }
    f
}

fn series(d: fn(usize, f64) -> f64, wrong: &dyn Fn(usize, usize) -> bool) -> Vec<FileFeatures> {
    (0..FILES).map(|k| file(k, d, wrong)).collect()
}

fn params(method: Method) -> FamAlignParams {
    let mut p = FamAlignParams::default();
    p.warp.method = method;
    p.warp.coarse_gap = 1.0;
    p
}

fn align(files: &[FileFeatures], method: Method) -> Aligned {
    let refs: Vec<&FileFeatures> = files.iter().collect();
    align_ions(&refs, &params(method), &RtParams::default()).unwrap()
}

fn complete_rows(al: &Aligned) -> usize {
    (0..al.len())
        .filter(|&r| (0..FILES).all(|k| al.state[r * FILES + k] == CELL_DETECTED))
        .count()
}

#[test]
fn without_correction_the_drift_splits_the_compounds() {
    let files = series(drift, &|_, _| false);
    let al = align(&files, Method::Off);
    assert!(al.len() > COMPOUNDS + 20, "{} rows", al.len());
}

#[test]
fn every_method_gives_one_row_per_compound_seen_in_all_files() {
    let files = series(drift, &|_, _| false);
    for m in [Method::Piecewise, Method::Loess, Method::Ransac] {
        // RANSAC fits a line: the wave of the drift is too much for the default distance
        let mut p = params(m);
        p.warp.inlier = 0.06;
        let refs: Vec<&FileFeatures> = files.iter().collect();
        let al = align_ions(&refs, &p, &RtParams::default()).unwrap();
        assert_eq!(al.len(), COMPOUNDS, "{m:?}");
        assert_eq!(complete_rows(&al), COMPOUNDS, "{m:?}");
    }
}

#[test]
fn the_curve_follows_the_drift_put_in() {
    let files = series(drift, &|_, _| false);
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let p = FamAlignParams::default();
    for m in [Method::Piecewise, Method::Loess] {
        let w = WarpParams {
            method: m,
            coarse_gap: 1.0,
            ..WarpParams::default()
        };
        let map = warp_map(&refs, &p.ion, &p.rt, &w).unwrap();
        assert!(map.ok(), "{m:?}");
        let mut worst = 0.0f64;
        for c in (0..COMPOUNDS).step_by(7) {
            let t = rt0(c);
            // the reference is the median over the files of the apex RT
            let mut apex: Vec<f64> = (0..FILES).map(|k| t + drift(k, t)).collect();
            apex.sort_by(f64::total_cmp);
            let reference = (apex[FILES / 2 - 1] + apex[FILES / 2]) / 2.0;
            for k in 0..FILES {
                let want = t + drift(k, t) - reference;
                worst = worst.max((map.at(k, reference) - want).abs());
            }
        }
        assert!(worst < 0.02, "{m:?}: {worst}");
    }
}

#[test]
fn ransac_ignores_the_anchors_with_a_wrong_rt() {
    // one compound in five has the wrong RT in file 3
    let wrong = |c: usize, k: usize| k == 3 && c.is_multiple_of(5);
    let files = series(line_drift, &wrong);
    let refs: Vec<&FileFeatures> = files.iter().collect();
    let p = FamAlignParams::default();
    let w = WarpParams {
        method: Method::Ransac,
        coarse_gap: 1.0,
        ..WarpParams::default()
    };
    let map = warp_map(&refs, &p.ion, &p.rt, &w).unwrap();
    assert!(map.ok());
    let t = 10.0;
    let mut apex: Vec<f64> = (0..FILES).map(|k| t + line_drift(k, t)).collect();
    apex.sort_by(f64::total_cmp);
    let reference = (apex[FILES / 2 - 1] + apex[FILES / 2]) / 2.0;
    let want = t + line_drift(3, t) - reference;
    assert!((map.at(3, reference) - want).abs() < 0.02);
}

#[test]
fn a_series_without_anchors_gives_an_empty_map() {
    let files = series(drift, &|_, _| false)[..2].to_vec();
    let few: Vec<FileFeatures> = files
        .iter()
        .map(|f| {
            let mut g = FileFeatures::default();
            for i in 0..2 {
                g.push_lean(f, i);
            }
            g
        })
        .collect();
    let refs: Vec<&FileFeatures> = few.iter().collect();
    let p = FamAlignParams::default();
    let w = WarpParams {
        method: Method::Loess,
        ..WarpParams::default()
    };
    let map = warp_map(&refs, &p.ion, &p.rt, &w).unwrap();
    assert!(!map.ok());
}
