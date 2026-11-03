use super::run;
use mzlab_core::peakq_auto::{auto_integrate, AutoParams};

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// three Gaussians on a flat, slightly noisy baseline: the trace of the wasm test of auto_integrate
fn trace() -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..400).map(|i| f64::from(i) * 0.015).collect();
    let g = |t: f64, c: f64, h: f64| h * (-((t - c) / 0.05).powi(2) / 2.0).exp();
    let y = rt
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            g(t, 1.0, 8.0e4)
                + g(t, 2.5, 2.0e4)
                + g(t, 4.5, 1.0e5)
                + 500.0
                + f64::from(i as u32 % 7) * 20.0
        })
        .collect();
    (rt, y)
}

#[test]
fn peaks_are_the_core_result_and_set_names_are_the_page_ones() {
    let (rt, y) = trace();
    let f = std::env::temp_dir().join("mzlab_peaks_auto_trace.csv");
    let body: Vec<String> = rt.iter().zip(&y).map(|(a, b)| format!("{a},{b}")).collect();
    std::fs::write(&f, format!("rt,y\n{}", body.join("\n"))).unwrap();
    let p = f.to_str().unwrap();
    let o = run(args(&[p]));
    assert_eq!(o.code, 0);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let core = auto_integrate(&rt, &y, rt[0], rt[rt.len() - 1], AutoParams::default());
    let got = v["peaks"].as_array().unwrap();
    assert_eq!(got.len(), 3);
    for (g, k) in got.iter().zip(&core) {
        assert_eq!(g["t0"].as_f64().unwrap(), rt[k.lo]);
        assert_eq!(g["t1"].as_f64().unwrap(), rt[k.hi]);
        assert_eq!(g["area"].as_f64().unwrap(), k.area);
    }
    // the page names are accepted; a prominence above the tallest peak keeps nothing
    let o = run(args(&[
        p,
        "--set",
        "hrSmooth=7",
        "--set",
        "hrSn=5",
        "--set",
        "hrProm=200",
    ]));
    assert_eq!(
        String::from_utf8(o.stdout).unwrap().trim(),
        "{\"peaks\":[]}"
    );
    let o = run(args(&[
        p,
        "--from",
        "2",
        "--to",
        "3",
        "--set",
        "hrProm=1",
        "--set",
        "hrSlope=3",
    ]));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["peaks"].as_array().unwrap().len(), 1);
    assert_ne!(run(args(&[p, "--set", "bogus=1"])).code, 0);
    assert_ne!(run(args(&[p, "--set", "hrSn=-1"])).code, 0);
    let _ = std::fs::remove_file(f);
}

#[test]
fn dialog_params_are_the_defaults_with_two_numbers_changed() {
    let d = AutoParams::default();
    let p = AutoParams::from_dialog(7, 9.0);
    assert_eq!((p.pick.smooth, p.pick.snr), (7, 9.0));
    assert_eq!(
        (p.pick.rel_prominence, p.pick.min_scans, p.pick.local_snr),
        (d.pick.rel_prominence, d.pick.min_scans, d.pick.local_snr)
    );
    assert_eq!(
        (p.half_window, p.edge_sd, p.slope),
        (d.half_window, d.edge_sd, d.slope)
    );
}

#[test]
fn the_dialog_sn_leaves_out_a_weak_peak() {
    // one strong and one weak Gaussian over random noise: S/N 5 keeps both, S/N 200 only the strong one
    let mut seed = 7u64;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5
    };
    let rt: Vec<f64> = (0..600).map(|i| f64::from(i) * 0.01).collect();
    let g = |t: f64, c: f64, h: f64| h * (-((t - c) / 0.05).powi(2) / 2.0).exp();
    let y: Vec<f64> = rt
        .iter()
        .map(|&t| g(t, 2.0, 1.0e5) + g(t, 4.0, 3.0e3) + 1000.0 + 200.0 * rnd())
        .collect();
    let n = |snr: f64| auto_integrate(&rt, &y, 0.0, 6.0, AutoParams::from_dialog(5, snr)).len();
    assert_eq!((n(5.0), n(200.0)), (2, 1));
}
