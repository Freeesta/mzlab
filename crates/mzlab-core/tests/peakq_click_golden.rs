//! The golden of the click: one synthetic trace and the exact JSON of five clicks on it (kept, refused, forced, no MRM), checked in. The same two files are read by the
//! tests of `mzlab-wasm` (`lr_click`, the page) and `mzlab-cli` (`peak edges --set click=1`), so a change of the engine, or a route that drifts from it, fails in the
//! ordinary `cargo test` jobs. Regenerate after a deliberate engine change: `MZLAB_BLESS=1 cargo test -p mzlab-core --test peakq_click_golden`.
use mzlab_core::peakq_click::{lr_click, ClickGuard};
use mzlab_core::peakq_lr::LrParams;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data");
/// (click time min, MRM trace, force): a big peak, a flat stretch (refused, then forced), a small peak, the same big peak read as a non-MRM trace.
const CASES: [(&str, bool, bool); 5] = [
    ("1.02", true, false),
    ("1.6", true, false),
    ("1.6", true, true),
    ("2.2", true, false),
    ("1.02", false, false),
];

/// 0.025 min sampling over 3 min, a flat 200 count level, two Gaussians and deterministic noise (an LCG): nothing real.
fn trace() -> (Vec<f64>, Vec<f64>) {
    let mut seed = 20240607u64;
    let mut rnd = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 33) as f64 / (1u64 << 31) as f64) - 0.5
    };
    let xs: Vec<f64> = (0..121).map(|i| i as f64 * 0.025).collect();
    let g = |t: f64, c: f64, h: f64, s: f64| h * (-((t - c) / s).powi(2) / 2.0).exp();
    let ys = xs
        .iter()
        .map(|&t| 200.0 + 60.0 * rnd() + g(t, 1.0, 5000.0, 0.05) + g(t, 2.2, 120.0, 0.04))
        .collect();
    (xs, ys)
}

fn csv(xs: &[f64], ys: &[f64]) -> String {
    let mut s = String::from("rt,y\n");
    for (x, y) in xs.iter().zip(ys) {
        s += &format!("{x:.6},{y:.3}\n");
    }
    s
}

/// The trace as the CSV file holds it (what the CLI and the page read), so every route starts from the same numbers.
fn rounded(csv: &str) -> (Vec<f64>, Vec<f64>) {
    csv.lines()
        .skip(1)
        .map(|l| {
            let (a, b) = l.split_once(',').unwrap();
            (a.parse::<f64>().unwrap(), b.parse::<f64>().unwrap())
        })
        .unzip()
}

#[test]
fn the_golden_is_the_engine() {
    let (xs, ys) = trace();
    let text = csv(&xs, &ys);
    let (rx, ry) = rounded(&text);
    let mut golden = String::new();
    for (at, mrm, force) in CASES {
        let g = ClickGuard {
            force,
            ..ClickGuard::default()
        };
        let c = lr_click(&rx, &ry, at.parse().unwrap(), mrm, &LrParams::default(), &g);
        golden += &format!(
            "at={at} mrm={} force={} ok={}\n{}\n",
            mrm as u8,
            force as u8,
            c.ok(),
            c.to_json()
        );
    }
    if std::env::var("MZLAB_BLESS").is_ok() {
        std::fs::write(format!("{DIR}/click_golden.csv"), &text).unwrap();
        std::fs::write(format!("{DIR}/click_golden.txt"), &golden).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(format!("{DIR}/click_golden.csv")).unwrap(),
        text,
        "the fixture changed: MZLAB_BLESS=1"
    );
    assert_eq!(
        std::fs::read_to_string(format!("{DIR}/click_golden.txt")).unwrap(),
        golden,
        "the engine changed: MZLAB_BLESS=1"
    );
    // the golden shows the three outcomes: kept, refused with its reason, forced past it
    assert!(
        golden.contains("ok=true")
            && golden.contains("ok=false")
            && golden.contains("\"forced\":true"),
        "{golden}"
    );
}
