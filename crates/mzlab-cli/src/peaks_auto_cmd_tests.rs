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
fn peaks_are_the_core_result_and_set_names_are_accepted() {
    let (rt, y) = trace();
    let f = std::env::temp_dir().join("mzlab_peaks_auto_trace.csv");
    let body: Vec<String> = rt.iter().zip(&y).map(|(a, b)| format!("{a},{b}")).collect();
    std::fs::write(&f, format!("rt,y\n{}", body.join("\n"))).unwrap();
    let p = f.to_str().unwrap();
    let o = run(args(&[p, "--baseline", "edges"]));
    assert_eq!(o.code, 0);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let core = auto_integrate(&rt, &y, rt[0], rt[rt.len() - 1], AutoParams::default());
    let got = v["peaks"].as_array().unwrap();
    assert_eq!(got.len(), 3);
    for (g, k) in got.iter().zip(&core) {
        assert_eq!(g["t0"].as_f64().unwrap(), rt[k.lo]);
        assert_eq!(g["t1"].as_f64().unwrap(), rt[k.hi]);
        assert_eq!(g["area"].as_f64().unwrap(), k.area * 60.0);
        assert_eq!(g["baseline"], "edges");
    }
    // the page names are accepted; a prominence above the tallest peak keeps nothing
    let o = run(args(&[
        p,
        "--set",
        "hr_smooth=7",
        "--set",
        "hr_sn=5",
        "--set",
        "hr_prom=200",
    ]));
    assert_eq!(
        String::from_utf8(o.stdout).unwrap().trim(),
        "{\"schema\":\"peak-auto/1\",\"peaks\":[],\"choices\":[{\"key\":\"b0_b1\",\"choice\":\"omitted\",\"reason\":\"baseline=zero: the area is measured from 0, so the detection baseline ends are not used\"}]}"
    );
    let o = run(args(&[
        p,
        "--from",
        "2",
        "--to",
        "3",
        "--set",
        "hr_prom=1",
        "--set",
        "hr_slope=3",
    ]));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["peaks"].as_array().unwrap().len(), 1);
    assert_eq!(run(args(&[p, "--set", "bogus=1"])).code, 2);
    assert_eq!(run(args(&[p, "--set", "hr_sn=-1"])).code, 2);
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

#[test]
fn the_baseline_mode_applies_to_every_peak() {
    use mzlab_core::baseline::integrate;
    let (rt, y) = trace();
    let f = std::env::temp_dir().join("mzlab_peaks_auto_trace_base.csv");
    let body: Vec<String> = rt.iter().zip(&y).map(|(a, b)| format!("{a},{b}")).collect();
    std::fs::write(&f, body.join("\n")).unwrap();
    let p = f.to_str().unwrap();
    // no mode named = zero
    assert_eq!(
        run(args(&[p, "--baseline", "zero"])).stdout,
        run(args(&[p])).stdout
    );
    let core = auto_integrate(&rt, &y, rt[0], rt[rt.len() - 1], AutoParams::default());
    let o = run(args(&[p]));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let got = v["peaks"].as_array().unwrap();
    assert_eq!(got.len(), core.len());
    for (g, k) in got.iter().zip(&core) {
        // same boundaries, the area now from zero (the background of about 560 counts inside it); the detection
        // baseline ends are not what that area is measured from, so b0/b1 are left out
        assert_eq!(g["t0"].as_f64().unwrap(), rt[k.lo]);
        assert!(g.get("b0").is_none() && g.get("b1").is_none(), "{g}");
        let z = integrate(&rt, &y, k.lo, k.hi, |_| 0.0);
        assert_eq!(
            (g["area"].as_f64().unwrap(), g["below"].as_f64().unwrap()),
            (z.area * 60.0, 0.0)
        );
        assert_eq!(g["baseline"], "zero");
        assert!(z.area > k.area);
    }
    let o = run(args(&[
        p,
        "--baseline",
        "line",
        "--set",
        "base_y0=1000",
        "--set",
        "base_t1=6",
        "--set",
        "base_y1=1000",
    ]));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert!(v["peaks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|g| g["baseline"] == "line" && g["below"].as_f64().unwrap() > 0.0));
    // the line's own ends are written with a line, the omission is logged with zero, and base_* without a line contradict the mode
    assert!(v["choices"].as_array().unwrap().is_empty());
    assert!(v["peaks"][0]["b0"].as_f64().unwrap() > 0.0, "{v}");
    let z: serde_json::Value = serde_json::from_slice(&run(args(&[p])).stdout).unwrap();
    assert_eq!(z["choices"][0]["key"], "b0_b1", "{z}");
    assert_eq!(run(args(&[p])).stderr, "baseline=zero: b0/b1 omitted\n");
    assert!(run(args(&[p, "--baseline", "edges"])).stderr.is_empty());
    let bad = run(args(&[p, "--set", "base_y0=5"]));
    assert_eq!(bad.code, 2, "{}", bad.stderr);
    assert_eq!(
        run(args(&[p, "--baseline", "edges", "--set", "base_y0=5"])).code,
        2
    );
    assert_eq!(run(args(&[p, "--baseline", "valley"])).code, 2);
}

#[test]
fn area_unit_min_keeps_the_old_counts_x_min() {
    let (rt, y) = trace();
    let f = std::env::temp_dir().join("mzlab_peaks_auto_unit.csv");
    let body: Vec<String> = rt.iter().zip(&y).map(|(a, b)| format!("{a},{b}")).collect();
    std::fs::write(&f, format!("rt,y\n{}", body.join("\n"))).unwrap();
    let p = f.to_str().unwrap();
    let core = auto_integrate(&rt, &y, rt[0], rt[rt.len() - 1], AutoParams::default());
    let o = run(args(&[p, "--baseline", "edges", "--area-unit", "min"]));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    for (g, k) in v["peaks"].as_array().unwrap().iter().zip(&core) {
        assert_eq!(g["area"].as_f64().unwrap(), k.area);
    }
    assert_eq!(run(args(&[p, "--area-unit", "s"])).code, 0);
    assert_eq!(run(args(&[p, "--area-unit", "h"])).code, 2);
}

/// The keys of the output are exactly those `docs/schemi/peak-auto-1.json` documents, for each baseline mode.
#[test]
fn the_output_keys_are_the_schemas() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../docs/schemi/peak-auto-1.json")).unwrap();
    let props = |v: &serde_json::Value| -> Vec<String> {
        v["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    };
    let (rt, y) = trace();
    let f = std::env::temp_dir().join("mzlab_peaks_auto_schema.csv");
    let body: Vec<String> = rt.iter().zip(&y).map(|(a, b)| format!("{a},{b}")).collect();
    std::fs::write(&f, body.join("\n")).unwrap();
    let p = f.to_str().unwrap();
    let item = &schema["properties"]["peaks"]["items"];
    for mode in ["zero", "edges", "line"] {
        let mut a = vec![p, "--baseline", mode];
        if mode == "line" {
            a.extend([
                "--set",
                "base_y0=1000",
                "--set",
                "base_t1=6",
                "--set",
                "base_y1=1000",
            ]);
        }
        let v: serde_json::Value = serde_json::from_slice(&run(args(&a)).stdout).unwrap();
        let text = String::from_utf8(run(args(&a)).stdout).unwrap();
        assert!(
            text.starts_with("{\"schema\":\"peak-auto/1\","),
            "schema is the first key"
        );
        let top: Vec<&String> = v.as_object().unwrap().keys().collect();
        assert_eq!(v["schema"], schema["properties"]["schema"]["const"]);
        for k in top {
            assert!(props(&schema).contains(k), "{k}");
        }
        for pk in v["peaks"].as_array().unwrap() {
            for k in pk.as_object().unwrap().keys() {
                assert!(props(item).contains(k), "{mode}: {k}");
            }
            for r in item["required"].as_array().unwrap() {
                assert!(pk.get(r.as_str().unwrap()).is_some(), "{mode}: {r}");
            }
        }
    }
    let _ = std::fs::remove_file(f);
}
