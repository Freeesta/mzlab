//! `mzlab-cli background` on the synthetic run of `ticbg_synth`: the six lines found, the peak kept, the options, a blank, the errors.
use crate::execute;

fn write(dir: &std::path::Path, name: &str, blank: bool) -> String {
    let p = dir.join(name);
    std::fs::write(&p, mzlab_core::ticbg_synth::mzml(600, if blank { 9 } else { 1 }, blank)).unwrap();
    p.to_string_lossy().into_owned()
}

fn json(args: &[&str]) -> mzlab_core::json::Value {
    let o = execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    assert_eq!(o.code, 0, "{}", o.stderr);
    mzlab_core::json::parse(&o.stdout).unwrap()
}

#[test]
fn background_of_a_synthetic_run() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_background_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (run, blank) = (write(&dir, "run.mzML", false), write(&dir, "blank.mzML", true));
    let v = json(&["background", &run]);
    let mut ions: Vec<i64> = v.get("ions").unwrap().arr().iter().map(|i| i.get("mz").unwrap().f64().unwrap().round() as i64).collect();
    ions.sort();
    assert_eq!(ions, [141, 149, 158, 198, 214, 236]);
    let (y, raw) = (v.get("y").unwrap().arr(), v.get("raw").unwrap().arr());
    assert_eq!(y.len(), 600);
    assert!(raw[0].f64().unwrap() > 2e5 && y[0].f64().unwrap().abs() < 2e4);
    // the peak at 14.3 min (scan 429) keeps its 120 000 counts of apex
    assert!(y[429].f64().unwrap() > 1e5, "{:?}", y[429]);
    assert_eq!(v.get("params").unwrap().get("mode").unwrap().str(), Some("auto"));
    // options by --set, with or without an instance prefix
    let q = json(&["background", &run, "--set", "mode=quiet", "--set", "background.rt0=1", "--set", "rt1=3", "--set", "kind=bpc"]);
    assert_eq!(q.get("kind").unwrap().str(), Some("bpc"));
    assert_eq!(q.get("params").unwrap().get("quiet").unwrap().arr().len(), 2);
    let b = json(&["background", &run, "--blank", &blank, "--set", "mode=blank"]);
    assert_eq!(b.get("ions").unwrap().arr().len(), 6);
    // errors: an unknown name, a bad value, a missing blank
    let bad = |a: &[&str]| execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>()).code;
    assert_eq!(bad(&["background", &run, "--set", "nope=1"]), 2);
    assert_ne!(bad(&["background", &run, "--set", "mode=nope"]), 0);
    assert_ne!(bad(&["background", &run, "--set", "mode=blank"]), 0);
    assert_eq!(bad(&["background", "--help"]), 0);
    let _ = std::fs::remove_dir_all(&dir);
}
