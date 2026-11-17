//! `ticbg` on a synthetic unit-resolution run of known truth (`synth`): six background lines with drift and noise, random chemical noise,
//! two real peaks of which one shares two m/z with background lines. Acceptance of `docs/agenti/tic-fondo.md`.
use crate::ticbg::*;

const N: usize = 1200; // 20 min, 1 s per scan
const LINES: [(f64, f64); 6] = [
    (141.0, 30e3),
    (149.0, 80e3),
    (158.0, 25e3),
    (198.0, 40e3),
    (214.0, 20e3),
    (236.0, 50e3),
];

struct Rng(u64);
impl Rng {
    fn u(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    fn g(&mut self) -> f64 {
        (-2.0 * self.u().ln()).sqrt() * (std::f64::consts::TAU * self.u()).cos()
    }
}

/// A peak: apex (min), sigma (min), its ions (m/z, height).
type Peak = (f64, f64, Vec<(f64, f64)>);

fn rt() -> Vec<f64> {
    (0..N).map(|j| j as f64 / 60.0).collect()
}

/// (scan, m/z, intensity) sorted by m/z, and the true TIC of the peaks per scan. `lines` = with the background, `hr` = m/z jitter of ppm.
fn synth(peaks: &[Peak], lines: bool, seed: u64) -> (Vec<(usize, f64, f64)>, Vec<f64>) {
    let mut r = Rng(seed);
    let mut pts = Vec::new();
    let mut truth = vec![0.0; N];
    for (j, t) in rt().into_iter().enumerate() {
        if lines {
            for &(mz, h) in &LINES {
                let drift = 1.0 + 0.6 * t / 20.0;
                pts.push((
                    j,
                    mz + 0.1 * (r.u() - 0.5),
                    h * drift * (1.0 + 0.05 * r.g()).max(0.0),
                ));
            }
        }
        for _ in 0..8 {
            pts.push((
                j,
                (100.0 + 300.0 * r.u()).round() + 0.2 * (r.u() - 0.5),
                500.0 * r.u(),
            ));
        }
        for (apex, s, ions) in peaks {
            let g = (-0.5 * ((t - apex) / s).powi(2)).exp();
            for &(mz, h) in ions {
                if g > 1e-6 {
                    pts.push((j, mz + 0.1 * (r.u() - 0.5), h * g));
                    truth[j] += h * g;
                }
            }
        }
    }
    pts.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    (pts, truth)
}

fn run(pts: &[(usize, f64, f64)], p: &Params, blank: Option<&BlankProfile>) -> Cleaned {
    let mut feed =
        |f: &mut dyn FnMut(usize, f64, f64)| pts.iter().for_each(|&(s, m, y)| f(s, m, y));
    let pres = presence(N, p, &mut feed);
    let flags = flagged(N, p, &pres, blank);
    clean(&rt(), p, &flags, blank, &mut feed)
}

fn peaks() -> Vec<Peak> {
    vec![
        (6.0, 0.08, vec![(250.0, 60e3), (272.0, 20e3)]),
        (
            14.3,
            0.08,
            vec![(198.0, 50e3), (158.0, 30e3), (305.0, 40e3)],
        ),
    ]
}

fn area(y: &[f64], apex: f64) -> f64 {
    rt().iter()
        .zip(y)
        .filter(|(t, _)| (**t - apex).abs() <= 0.4)
        .map(|(_, v)| v)
        .sum()
}

/// Apex over the noise floor outside the peaks (median + 3 robust sigma of the trace away from both peaks).
fn peak_to_baseline(y: &[f64], apex: f64) -> f64 {
    let t = rt();
    let out: Vec<f64> = t
        .iter()
        .zip(y)
        .filter(|(t, _)| (**t - 6.0).abs() > 0.6 && (**t - 14.3).abs() > 0.6)
        .map(|(_, v)| *v)
        .collect();
    let med = crate::tracestat::median_of(&out);
    let mad = crate::tracestat::median_of(&out.iter().map(|v| (v - med).abs()).collect::<Vec<_>>());
    let top = t
        .iter()
        .zip(y)
        .filter(|(t, _)| (**t - apex).abs() < 0.1)
        .map(|(_, v)| *v)
        .fold(f64::MIN, f64::max);
    top / (med.abs() + 3.0 * 1.4826 * mad)
}

#[test]
fn automatic_finds_the_six_lines_and_recovers_the_peaks() {
    let (pts, truth) = synth(&peaks(), true, 1);
    let c = run(&pts, &Params::default(), None);
    let mut found: Vec<f64> = c.ions.iter().map(|i| i.mz.round()).collect();
    found.sort_by(f64::total_cmp);
    assert_eq!(
        found,
        LINES.iter().map(|l| l.0).collect::<Vec<_>>(),
        "{:?}",
        c.ions
    );
    assert!(c
        .ions
        .iter()
        .all(|i| i.persistence > 0.99 && i.confidence > 0.8 && i.share > 0.0));
    let raw: Vec<f64> = (0..N)
        .map(|j| pts.iter().filter(|p| p.0 == j).map(|p| p.2).sum())
        .collect();
    for apex in [6.0, 14.3] {
        let (want, got) = (area(&truth, apex), area(&c.tic, apex));
        assert!(
            (got - want).abs() / want < 0.05,
            "area at {apex}: {got} vs {want}"
        );
        let (before, after) = (peak_to_baseline(&raw, apex), peak_to_baseline(&c.tic, apex));
        assert!(
            after >= 3.0 * before,
            "peak/baseline at {apex}: {before} -> {after}"
        );
    }
    assert!(c.removed > 0.8 && c.removed < 1.0, "{}", c.removed);
}

#[test]
fn a_peak_on_a_background_mz_keeps_its_area_under_subtraction_and_loses_it_under_exclusion() {
    let shared: Vec<Peak> = vec![(14.3, 0.08, vec![(198.0, 50e3), (158.0, 30e3)])];
    let (pts, truth) = synth(&shared, true, 2);
    let want = area(&truth, 14.3);
    let sub = run(&pts, &Params::default(), None);
    assert!(
        area(&sub.tic, 14.3) >= 0.9 * want,
        "{} of {want}",
        area(&sub.tic, 14.3)
    );
    let ex = run(
        &pts,
        &Params {
            action: Action::Exclude,
            ..Params::default()
        },
        None,
    );
    assert!(
        area(&ex.tic, 14.3) < 0.1 * want,
        "{} of {want}",
        area(&ex.tic, 14.3)
    );
}

#[test]
fn windows_keep_quiet_and_blank() {
    let (pts, truth) = synth(&peaks(), true, 3);
    let want = area(&truth, 14.3);
    // a manual window takes the whole ion, background and peak
    let w = run(
        &pts,
        &Params {
            windows: vec![(305.0, 0.5)],
            ..Params::default()
        },
        None,
    );
    assert!(
        (area(&w.tic, 14.3)
            - (want
                - area(
                    &synth(&[(14.3, 0.08, vec![(305.0, 40e3)])], false, 3).1,
                    14.3
                )))
        .abs()
            / want
            < 0.06
    );
    assert!(w.windows > 0.0 && w.windows < w.removed);
    // a kept ion stays raw and leaves the list
    let k = run(
        &pts,
        &Params {
            keep: vec![149.0],
            ..Params::default()
        },
        None,
    );
    assert_eq!(k.ions.len(), 5);
    assert!(k.ions.iter().all(|i| i.mz.round() != 149.0));
    // the quiet window at the start: a constant baseline leaves the drift (the end of the run sits higher), the peak stays
    let q = run(
        &pts,
        &Params {
            mode: Mode::Quiet,
            quiet: Some((0.5, 2.0)),
            ..Params::default()
        },
        None,
    );
    assert!(q.tic[N - 1] > q.tic[60] + 50e3);
    assert!(area(&q.tic, 14.3) > want);
    // the blank: the same lines without peaks name the six ions and give their levels
    let (bl, _) = synth(&[], true, 4);
    let p = Params {
        mode: Mode::Blank,
        ..Params::default()
    };
    let prof = blank_profile(N, &p, &mut |f| bl.iter().for_each(|&(s, m, y)| f(s, m, y)));
    assert_eq!(prof.len(), 6);
    let b = run(&pts, &p, Some(&prof));
    assert_eq!(b.ions.len(), 6);
    assert!(area(&b.tic, 14.3) > 0.9 * want);
}

#[test]
fn high_resolution_bins_merge_a_line_split_by_an_edge() {
    let p = Params {
        hr: true,
        ..Params::default()
    };
    assert!(p.bin(300.001) - p.bin(299.999) <= 1);
    let mut pts = Vec::new();
    for j in 0..N {
        let m = if j % 2 == 0 { 299.999 } else { 300.001 };
        pts.push((j, m, 1e4));
    }
    let c = run(&pts, &Params { floor: false, ..p }, None);
    assert_eq!(c.ions.len(), 1, "{:?}", c.ions);
    assert!((c.ions[0].mz - 300.0).abs() < 0.002);
    assert!(c.tic.iter().all(|v| v.abs() < 1e-6));
}

#[test]
fn the_rolling_baseline_follows_a_ramp_and_ignores_a_narrow_peak() {
    let x: Vec<f64> = (0..1000)
        .map(|j| 100.0 + j as f64 + if (480..520).contains(&j) { 5000.0 } else { 0.0 })
        .collect();
    let b = rolling_baseline(&x, &Params::default());
    assert!((b[500] - 600.0).abs() < 30.0, "{}", b[500]);
    assert!((b[100] - 200.0).abs() < 30.0, "{}", b[100]);
}
