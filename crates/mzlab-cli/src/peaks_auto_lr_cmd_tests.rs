use super::run;
use mzlab_core::area_unit::{lr_auto_json_in, AreaUnit};
use mzlab_core::peakq_lrauto::{lr_auto_integrate, lr_auto_json, LrAutoParams, Split};

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// two touching jagged peaks and one alone, 1.5 s cycle (the low-resolution test traces of mzlab-core)
fn trace() -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..280).map(|i| f64::from(i) * 0.025).collect();
    let g = |t: f64, c: f64, h: f64| h * (-((t - c) / 0.06).powi(2) / 2.0).exp();
    let y = rt
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            g(t, 2.0, 2.0e4)
                + g(t, 2.24, 1.2e4)
                + g(t, 5.0, 8.0e3)
                + 200.0
                + f64::from(i as u32 % 5) * 30.0
        })
        .collect();
    (rt, y)
}

fn write(rt: &[f64], y: &[f64]) -> std::path::PathBuf {
    let f = std::env::temp_dir().join(format!("mzlab_peak_auto_lr_{}.csv", std::process::id()));
    let body: Vec<String> = rt.iter().zip(y).map(|(a, b)| format!("{a},{b}")).collect();
    std::fs::write(&f, format!("rt,y\n{}", body.join("\n"))).unwrap();
    f
}

#[test]
fn the_output_is_the_core_json_byte_for_byte() {
    let (rt, y) = trace();
    let f = write(&rt, &y);
    let p = f.to_str().unwrap();
    let o = run(args(&[p]));
    assert_eq!(o.code, 0);
    let core = lr_auto_integrate(&rt, &y, rt[0], rt[rt.len() - 1], LrAutoParams::default());
    assert_eq!(core.peaks.len(), 3);
    assert_eq!(
        String::from_utf8(o.stdout).unwrap(),
        format!("{}\n", lr_auto_json_in(&rt, &core, AreaUnit::Seconds))
    );
    // the dialog names, in the units of the page (valley in %)
    let o = run(args(&[
        p,
        "--set",
        "lr_half=3",
        "--set",
        "lr_valley=30",
        "--set",
        "lr_sn=4",
        "--set",
        "lr_min_pts=6",
        "--set",
        "lr_despike=0",
        "--set",
        "lr_split=valley",
        "--from",
        "4",
        "--to",
        "6",
    ]));
    let want = LrAutoParams {
        half: 3,
        valley: 0.3,
        snr: 4.0,
        min_pts: 6,
        despike: false,
        split: Split::Valley,
    };
    assert_eq!(
        String::from_utf8(o.stdout).unwrap(),
        format!(
            "{}\n",
            lr_auto_json_in(
                &rt,
                &lr_auto_integrate(&rt, &y, 4.0, 6.0, want),
                AreaUnit::Seconds
            )
        )
    );
    // the old unit stays selectable and is the old bytes
    let o = run(args(&[p, "--set", "area_unit=min"]));
    let core = lr_auto_integrate(&rt, &y, rt[0], rt[rt.len() - 1], LrAutoParams::default());
    assert_eq!(
        String::from_utf8(o.stdout).unwrap(),
        format!("{}\n", lr_auto_json(&rt, &core))
    );
    assert_ne!(run(args(&[p, "--set", "area_unit=h"])).code, 0);
    let _ = std::fs::remove_file(&f);
}

#[test]
fn bad_arguments_are_refused() {
    assert_ne!(run(args(&[])).code, 0);
    assert_ne!(run(args(&["x.csv", "--set", "lr_split=tangent"])).code, 0);
    assert_ne!(run(args(&["x.csv", "--set", "lr_half=-1"])).code, 0);
    assert_ne!(run(args(&["x.csv", "--set", "hr_sn=3"])).code, 0);
    assert_eq!(run(args(&["--help"])).code, 0);
}
