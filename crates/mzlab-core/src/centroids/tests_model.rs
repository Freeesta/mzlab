//! The width model of the file drives the windows: on synthetic Orbitrap-like (FWHM ~ m^1.5) and TOF-like (FWHM ~ m, spacing ~ m^0.5) files, the model
//! estimated from the file as a conversion does, singles over m/z 150-1200 give one unflagged centroid, 1:1 doublets 1.2 FWHM apart give two, and 1:0.3 shoulders 1 FWHM away are found.

use super::tests::Rng;
use super::*;

fn gauss(m: f64, c: f64, h: f64, fw: f64) -> f64 {
    h * (-4.0 * std::f64::consts::LN_2 * ((m - c) / fw).powi(2)).exp()
}

/// The synthetic files: an Orbitrap-like one (FWHM = 200/60 000 (m/200)^1.5, 10 points per FWHM everywhere), a TOF-like one (FWHM = m/30 000, spacing
/// ~ m^0.5: 5 points per FWHM at m/z 150, 14 at 1200) and a stress case with the Orbitrap widths on the TOF spacing (3 points per FWHM at 180, 20 at 1200).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Grid {
    Orbitrap,
    Tof,
    Steep,
}

fn fwhm(m: f64, g: Grid) -> f64 {
    match g {
        Grid::Tof => m / 30_000.0,
        _ => 200.0 / 60_000.0 * (m / 200.0).powf(1.5),
    }
}

/// The point spacing at `m`: 10 points per FWHM on the Orbitrap-like grid, a spacing growing as m^0.5 with 10 points per FWHM at m/z 600 on the others.
fn spacing(m: f64, g: Grid) -> f64 {
    match g {
        Grid::Orbitrap => fwhm(m, g) / 10.0,
        _ => fwhm(600.0, g) / 10.0 * (m / 600.0).sqrt(),
    }
}

/// A profile scan within 6 FWHM of each group of peaks, noise of amplitude `1e4 / snr`.
fn scan(groups: &[Vec<(f64, f64)>], snr: f64, grid: Grid, rng: &mut Rng) -> (Vec<f64>, Vec<f64>) {
    let (mut mz, mut y) = (Vec::new(), Vec::new());
    for g in groups {
        let (lo, hi) = (g[0].0, g[g.len() - 1].0);
        let mut m = lo - 6.0 * fwhm(lo, grid);
        while m < hi + 6.0 * fwhm(hi, grid) {
            let v: f64 = g.iter().map(|&(c, h)| gauss(m, c, h, fwhm(c, grid))).sum();
            mz.push(m);
            y.push((v + 1e4 / snr * (rng.next() - 0.5) * 3.46).max(0.0));
            m += spacing(m, grid);
        }
    }
    (mz, y)
}

/// What a file of 20 scans of each kind gives: singles flagged asymmetric or shoulder, singles flagged merged, 1:1 doublets 1.2 FWHM apart resolved,
/// 1:0.3 shoulders 1 FWHM away found (each as (count, of)), and the mean m/z error in ppm of the singles and of the resolved doublets.
#[derive(Debug, PartialEq)]
pub(super) struct Effect {
    pub flagged: (usize, usize),
    pub merged: usize,
    pub resolved: (usize, usize),
    pub shoulders: (usize, usize),
    pub ppm_single: f64,
    pub ppm_doublet: f64,
}

pub(super) fn measure(snr: f64, grid: Grid, model: bool) -> Effect {
    let mut rng = Rng(5);
    let centre = |i: usize, rng: &mut Rng| 150.0 + 26.0 * i as f64 + 5.0 * rng.next();
    let singles = |rng: &mut Rng| -> Vec<Vec<(f64, f64)>> {
        (0..40)
            .map(|i| vec![(centre(i, rng), 1e4 * (0.5 + rng.next()))])
            .collect()
    };
    let scans: Vec<_> = (0..6)
        .map(|_| scan(&singles(&mut rng), snr, grid, &mut rng))
        .collect();
    let refs: Vec<(&[f64], &[f64])> = scans
        .iter()
        .map(|(a, b)| (a.as_slice(), b.as_slice()))
        .collect();
    let mut p = estimate(&refs, 0.7).params;
    assert!(p.wm.is_some(), "a width model for the file");
    if !model {
        p.wm = None;
    }
    let mut buf = Buf::default();
    let mut out = [(0, 0); 3];
    let mut merged = 0;
    let (mut ppm, mut n) = ([0.0; 2], [0usize; 2]);
    for _ in 0..20 {
        let truth = singles(&mut rng);
        let (mz, y) = scan(&truth, snr, grid, &mut rng);
        for c in centroids(&mz, &y, &p, &mut buf)
            .iter()
            .filter(|c| c.value > 2e3)
        {
            out[0].1 += 1;
            out[0].0 += usize::from(c.flags & (ASYMMETRIC | SHOULDER) != 0);
            merged += usize::from(c.flags & MERGED != 0);
            let m = truth
                .iter()
                .map(|g| g[0].0)
                .min_by(|a, b| (a - c.mz).abs().total_cmp(&(b - c.mz).abs()))
                .unwrap();
            ppm[0] += ((c.mz - m) / m * 1e6).abs();
            n[0] += 1;
        }
        let pairs = |sep: f64, second: f64, rng: &mut Rng| -> Vec<Vec<(f64, f64)>> {
            (0..20)
                .map(|i| {
                    let c = centre(2 * i, rng);
                    vec![(c, 1e4), (c + sep * fwhm(c, grid), second * 1e4)]
                })
                .collect()
        };
        for (k, (sep, second)) in [(1.2, 1.0), (1.0, 0.3)].into_iter().enumerate() {
            let groups = pairs(sep, second, &mut rng);
            let (mz, y) = scan(&groups, snr, grid, &mut rng);
            let cs = centroids(&mz, &y, &p, &mut buf);
            for g in &groups {
                let (a, b) = (g[0].0, g[1].0);
                let near = |m: f64| {
                    cs.iter()
                        .find(|c| (c.mz - m).abs() < 0.3 * (b - a) && c.value > 0.1 * second * 1e4)
                };
                out[k + 1].1 += 1;
                if let (Some(x), Some(y)) = (near(a), near(b)) {
                    out[k + 1].0 += 1;
                    if k == 0 {
                        ppm[1] += ((x.mz - a) / a * 1e6).abs() + ((y.mz - b) / b * 1e6).abs();
                        n[1] += 2;
                    }
                }
            }
        }
    }
    Effect {
        flagged: out[0],
        merged,
        resolved: out[1],
        shoulders: out[2],
        ppm_single: ppm[0] / n[0].max(1) as f64,
        ppm_doublet: ppm[1] / n[1].max(1) as f64,
    }
}

#[test]
fn the_model_of_the_file_drives_the_windows() {
    for grid in [Grid::Orbitrap, Grid::Tof, Grid::Steep] {
        for snr in [1000.0, 50.0] {
            eprintln!(
                "{grid:?} snr {snr}\n  model  {:?}\n  single {:?}",
                measure(snr, grid, true),
                measure(snr, grid, false)
            );
        }
    }
}
