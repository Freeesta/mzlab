//! Pseudo MS2 spectra of `dia::pseudo_ms2` on a synthetic SWATH run made here (fixed-seed LCG, no file): 600 s, one MS1 and four MS2 windows per
//! second, 40 precursors with 3-8 fragments, co-eluting interferers, constant background ions and isolated spikes.

use mzlab_feat::dia::{pseudo_ms2, DiaParams, DiaScan, Prec, Pseudo};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
    fn log_range(&mut self, a: f64, b: f64) -> f64 {
        (a.ln() + (b.ln() - a.ln()) * self.next()).exp()
    }
}

const WINDOWS: [(f64, f64); 4] = [
    (100.0, 200.0),
    (200.0, 300.0),
    (300.0, 400.0),
    (400.0, 500.0),
];
const NOISE: f64 = 1e3;

#[derive(Clone)]
struct Spec {
    win: usize,
    mz: f64,
    apex: f64,
    sigma: f64,
    h: f64,
    frags: Vec<(f64, f64)>,
}

type RawScan = (f64, u8, Option<(f64, f64)>, Vec<f64>, Vec<f64>);

struct Run {
    // (rt, level, window, mz, intensity)
    scans: Vec<RawScan>,
}

fn spec(r: &mut Lcg, win: usize, used: &mut Vec<f64>) -> Spec {
    let (lo, _) = WINDOWS[win];
    let mz = lo + r.range(10.0, 90.0);
    let mut frags = Vec::new();
    for _ in 0..(3 + (r.next() * 6.0) as usize) {
        let m = loop {
            let m = (r.range(50.0, mz - 1.0) * 1e4).round() / 1e4;
            if used.iter().all(|u| (u - m).abs() > 0.1) {
                break m;
            }
        };
        used.push(m);
        frags.push((m, r.range(0.05, 1.0)));
    }
    used.push(mz);
    Spec {
        win,
        mz,
        apex: r.range(60.0, 540.0),
        sigma: r.range(2.5, 4.0),
        h: r.log_range(1e4, 1e6),
        frags,
    }
}

fn gauss(s: &Spec, t: f64) -> f64 {
    (-0.5 * ((t - s.apex) / s.sigma).powi(2)).exp()
}

/// The run of the precursors `specs`, with `bg` constant ions and `spikes` single-cycle peaks per window.
fn build(r: &mut Lcg, specs: &[Spec], secs: usize, bg: usize, spikes: usize) -> Run {
    let bgs: Vec<Vec<(f64, f64)>> = (0..4)
        .map(|w| {
            (0..bg)
                .map(|_| (WINDOWS[w].0 + r.range(1.0, 99.0), r.range(1e3, 3e3)))
                .collect()
        })
        .collect();
    let spk: Vec<Vec<(f64, usize, f64)>> = (0..4)
        .map(|w| {
            (0..spikes)
                .map(|_| {
                    (
                        WINDOWS[w].0 + r.range(1.0, 99.0),
                        (r.next() * secs as f64) as usize,
                        r.range(1e4, 1e5),
                    )
                })
                .collect()
        })
        .collect();
    let mut scans = Vec::new();
    let noisy = |r: &mut Lcg, v: f64| v * (1.0 + 0.05 * (2.0 * r.next() - 1.0));
    for t in 0..secs {
        let t0 = t as f64;
        let mut ms1: Vec<(f64, f64)> = Vec::new();
        for s in specs {
            let v = noisy(r, s.h * gauss(s, t0));
            if v >= 1.0 {
                ms1.push((s.mz, v));
            }
        }
        ms1.sort_by(|a, b| a.0.total_cmp(&b.0));
        scans.push((
            t0,
            1,
            None,
            ms1.iter().map(|p| p.0).collect(),
            ms1.iter().map(|p| p.1).collect(),
        ));
        for (w, win) in WINDOWS.iter().enumerate() {
            let tw = t0 + 0.2 * (w + 1) as f64;
            let mut peaks: Vec<(f64, f64)> = Vec::new();
            for s in specs.iter().filter(|s| s.win == w) {
                for (m, rel) in &s.frags {
                    let v = noisy(r, s.h * rel * gauss(s, tw));
                    if v >= 1.0 {
                        peaks.push((*m, v));
                    }
                }
            }
            for (m, v) in &bgs[w] {
                peaks.push((*m, noisy(r, *v) * (1.0 + 0.1 * (2.0 * r.next() - 1.0))));
            }
            for (m, at, v) in &spk[w] {
                if *at == t {
                    peaks.push((*m, *v));
                }
            }
            peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
            scans.push((
                tw,
                2,
                Some(*win),
                peaks.iter().map(|p| p.0).collect(),
                peaks.iter().map(|p| p.1).collect(),
            ));
        }
    }
    Run { scans }
}

fn view(run: &Run) -> Vec<DiaScan<'_>> {
    run.scans
        .iter()
        .map(|s| DiaScan {
            rt_s: s.0,
            level: s.1,
            iso: s.2,
            mz: &s.3,
            int: &s.4,
        })
        .collect()
}

fn prec(id: usize, s: &Spec) -> Prec {
    Prec {
        id: id.to_string(),
        mz: s.mz,
        rt_lo_s: s.apex - 5.0 * s.sigma,
        rt_hi_s: s.apex + 5.0 * s.sigma,
    }
}

fn hits(p: &Pseudo, s: &Spec) -> usize {
    let near = |a: f64, b: f64| (a - b).abs() <= b * 10e-6;
    p.peaks
        .iter()
        .filter(|k| s.frags.iter().any(|f| near(k.0, f.0)))
        .count()
}

#[test]
fn recall_and_precision_on_the_synthetic_run() {
    let mut r = Lcg(20240607);
    let mut used = Vec::new();
    let mut targets = Vec::new();
    let mut all = Vec::new();
    for w in 0..4 {
        for k in 0..10 {
            let t = spec(&mut r, w, &mut used);
            if k < 8 {
                // an interferer in the same window that elutes 6-12 s away
                let mut i = spec(&mut r, w, &mut used);
                let side = if r.next() < 0.5 { -1.0 } else { 1.0 };
                i.apex = (t.apex + side * r.range(6.0, 12.0)).clamp(30.0, 570.0);
                all.push((targets.len(), i, true));
            }
            targets.push((t.win, k < 8));
            all.push((targets.len() - 1, t, false));
        }
    }
    let specs: Vec<Spec> = all.iter().map(|a| a.1.clone()).collect();
    let run = build(&mut r, &specs, 600, 30, 10);
    let scans = view(&run);
    let tg: Vec<(usize, &Spec, bool)> = all
        .iter()
        .filter(|a| !a.2)
        .map(|a| (a.0, &a.1, targets[a.0].1))
        .collect();
    let precs: Vec<Prec> = tg.iter().map(|t| prec(t.0, t.1)).collect();
    let out = pseudo_ms2(&scans, &precs, &DiaParams::default());
    let (mut strong, mut found, mut reported, mut good) = (0, 0, 0, 0);
    let (mut c_rep, mut c_good) = (0, 0);
    for (id, s, paired) in &tg {
        let p = out.iter().find(|p| p.id == id.to_string());
        let near = |a: f64, b: f64| (a - b).abs() <= b * 10e-6;
        for f in s.frags.iter().filter(|f| s.h * f.1 >= 3.0 * NOISE) {
            strong += 1;
            if p.is_some_and(|p| p.peaks.iter().any(|k| near(k.0, f.0))) {
                found += 1;
            }
        }
        if let Some(p) = p {
            reported += p.peaks.len();
            good += hits(p, s);
            if *paired {
                c_rep += p.peaks.len();
                c_good += hits(p, s);
            }
        }
    }
    let (recall, precision, coel) = (
        found as f64 / strong as f64,
        good as f64 / reported as f64,
        c_good as f64 / c_rep as f64,
    );
    println!("recall={recall:.3} precision={precision:.3} coelute_precision={coel:.3}");
    assert!(recall >= 0.90, "recall {recall}");
    assert!(precision >= 0.95, "precision {precision}");
    assert!(coel >= 0.85, "coelute precision {coel}");
}

/// Isomers with the same apex cannot be told apart by their traces: the fragments of each end in the spectrum of both.
#[test]
fn perfectly_coeluting_precursors_share_their_fragments() {
    let mut r = Lcg(7);
    let mut used = Vec::new();
    let a = spec(&mut r, 1, &mut used);
    let mut b = spec(&mut r, 1, &mut used);
    b.apex = a.apex;
    b.sigma = a.sigma;
    b.h = a.h;
    let run = build(&mut r, &[a.clone(), b.clone()], 300, 0, 0);
    let scans = view(&run);
    let pa = prec(0, &a);
    let pseudo = pseudo_ms2(&scans, &[pa], &DiaParams::default());
    assert_eq!(pseudo.len(), 1);
    assert_eq!(hits(&pseudo[0], &a), a.frags.len());
    assert_eq!(
        hits(&pseudo[0], &b),
        b.frags.len(),
        "the isomer's fragments are in too"
    );
}

#[test]
fn no_cycles_gives_nothing() {
    let none: Vec<DiaScan> = Vec::new();
    let p = Prec {
        id: "x".into(),
        mz: 150.0,
        rt_lo_s: 0.0,
        rt_hi_s: 10.0,
    };
    assert!(pseudo_ms2(&none, &[p], &DiaParams::default()).is_empty());
}
