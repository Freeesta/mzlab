//! Common gradient mode G(t) for the broad-band pass: 50 synthetic traces share a drift that reads as two wide bands on each of them.
//! Without G(t) every narrow-only trace gives false broad bands; with it none, and the real bands (a tenth of the traces) are still found.

use mzlab_feat::broad::{detect_broad, BroadParams};
use mzlab_feat::broad_gt::{share, TraceBank};

const N: usize = 1200;
const TRACES: usize = 50;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self, sd: f64) -> f64 {
        let mut s = 0.0;
        for _ in 0..12 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            s += (self.0 >> 11) as f64 / (1u64 << 53) as f64;
        }
        (s - 6.0) * sd
    }
}

fn gauss(t: f64, t0: f64, sd: f64, h: f64) -> f64 {
    h * libm::exp(-0.5 * (t - t0) * (t - t0) / (sd * sd))
}

/// The shared drift: two humps of the gradient and a ramp.
fn drift(t: f64) -> f64 {
    400.0 + 0.8 * t + gauss(t, 350.0, 50.0, 2500.0) + gauss(t, 850.0, 70.0, 3000.0)
}

/// Trace `i`: its own share of the drift, two narrow peaks and, every tenth trace, a real band at 600 s.
struct Case {
    rt: Vec<f64>,
    y: Vec<f64>,
    narrow: Vec<(usize, usize)>,
    banded: bool,
}

fn trace(i: usize) -> Case {
    let mut rng = Lcg(11 + i as u64);
    let c = 0.6 + 0.8 * (i % 7) as f64 / 6.0;
    let banded = i % 10 == 3;
    let rt: Vec<f64> = (0..N).map(|t| t as f64).collect();
    let y = rt
        .iter()
        .map(|&t| {
            c * drift(t)
                + gauss(t, 150.0 + 3.0 * i as f64, 4.0, 9000.0)
                + gauss(t, 1000.0, 5.0, 9000.0)
                + if banded {
                    gauss(t, 600.0, 45.0, 3500.0)
                } else {
                    0.0
                }
                + rng.next(40.0)
        })
        .collect();
    let a = 150 + 3 * i;
    Case {
        rt,
        y,
        narrow: vec![(a - 20, a + 20), (975, 1025)],
        banded,
    }
}

fn run(gt: bool) -> (f64, f64) {
    let mut bank = TraceBank::new(N, 8);
    for i in 0..TRACES {
        bank.add(0, &trace(i).y).unwrap();
    }
    let p = BroadParams {
        enabled: true,
        gt,
        common: share(bank.finish().unwrap()),
        ..BroadParams::default()
    };
    let (mut fp, mut narrow_only, mut hit, mut banded) = (0, 0, 0, 0);
    for i in 0..TRACES {
        let c = trace(i);
        let mz = vec![300.0; N];
        let found = detect_broad(&c.rt, &mz, &c.y, &c.narrow, &p).unwrap();
        if c.banded {
            banded += 1;
            hit += found.iter().any(|b| (b.rt_apex - 600.0).abs() <= 40.0) as usize;
        } else {
            narrow_only += 1;
            fp += found.len();
        }
    }
    (fp as f64 / narrow_only as f64, hit as f64 / banded as f64)
}

#[test]
fn common_mode_removes_the_shared_drift_and_keeps_the_bands() {
    let (fp_off, _) = run(false);
    let (fp_on, recall) = run(true);
    eprintln!("false bands per narrow-only trace: {fp_off} -> {fp_on}; recall {recall}");
    assert!(fp_off > 1.0, "without G(t): {fp_off}");
    assert!(fp_on <= 0.05, "with G(t): {fp_on}");
    assert!(recall >= 0.8, "recall {recall}");
}

#[test]
fn few_traces_give_no_common_mode() {
    let mut bank = TraceBank::new(N, 8);
    bank.add(0, &trace(0).y).unwrap();
    assert!(bank.finish().unwrap().is_none());
    bank.add(0, &vec![1.0; 100]).unwrap();
    assert!(bank.finish().unwrap().is_none());
}

#[test]
fn offer_then_push_row_is_add() {
    // 3000 traces of every length and start: the bank doubles its stride several times, and `offer` must count exactly as `add` does
    let mut rng = Lcg(3);
    let (mut a, mut b) = (TraceBank::new(N, 8), TraceBank::new(N, 8));
    let mut built = 0usize;
    for i in 0..3000usize {
        let len = [30, 399, 400, 500, 1200, 1300][i % 6];
        let scan0 = (i * 37) % 900;
        let y: Vec<f64> = (0..len)
            .map(|k| 1e5 + 1e4 * (k as f64 / 80.0).sin() + rng.next(1e3))
            .collect();
        a.add(scan0, &y).unwrap();
        if b.offer(scan0, len) {
            built += 1;
            b.push_row(scan0, &y).unwrap();
        }
    }
    assert!(
        built < 1200,
        "{built}: the stride must spare most of the traces"
    );
    let (ga, gb) = (a.finish().unwrap(), b.finish().unwrap());
    assert!(ga.is_some());
    assert!(ga == gb);
}
