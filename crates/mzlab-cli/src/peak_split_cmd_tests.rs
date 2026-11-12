//! `mzlab-cli peak-split` on the SIM `.wiff` of `tools/wiff_sim.py` (skipped without python3): one window over the isomer pair of ionC in
//! sample B finds one valley between the two apexes the writer put; cut there it gives two `drop` rows whose areas add up to the whole window,
//! the bytes of `manual-table` for the same window with `splits`; bad settings fail.

use std::path::Path;

use mzlab_core::json::parse;

use crate::execute;

fn run(file: &Path, set: &[&str]) -> crate::Outcome {
    let mut a = vec![
        "peak-split".to_string(),
        file.to_string_lossy().into_owned(),
    ];
    for s in set {
        a.push("--set".into());
        a.push(s.to_string());
    }
    execute(&a)
}

fn text(o: &crate::Outcome) -> String {
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout.clone()).unwrap()
}

const PAIR: [&str; 4] = ["file=2", "tr=2", "a=6.33", "b=7.05"];

#[test]
fn the_isomer_pair_splits_once_and_its_parts_add_up() {
    let d = std::env::temp_dir().join(format!("mzlab_cli_valley_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_sim.py");
    let made = std::process::Command::new("python3")
        .arg(script)
        .arg(&d)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if made.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP peak-split: python3 not available");
        return;
    }
    let wiff = d.join("sim3.wiff");
    let truth = parse(&std::fs::read(d.join("sim3.json")).unwrap()).unwrap();
    let apex = |k: usize| {
        truth.get("picchi").unwrap().arr()[2].arr()[2].arr()[k]
            .f64()
            .unwrap()
    };
    let v = text(&run(&wiff, &PAIR));
    let at: Vec<f64> = parse(v.as_bytes())
        .unwrap()
        .get("splits")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.f64().unwrap())
        .collect();
    assert_eq!(at.len(), 1, "{v}");
    assert!(
        at[0] > apex(0).min(apex(1)) && at[0] < apex(0).max(apex(1)),
        "{v}"
    );
    let cut = format!("split={}", at[0]);
    let parts = text(&run(&wiff, &[&PAIR[..], &[cut.as_str()]].concat()));
    assert_eq!(
        parts,
        text(&run(&wiff, &[&PAIR[..], &["split=auto"]].concat())),
        "auto cuts at the valleys found"
    );
    let rows: Vec<Vec<&str>> = parts
        .lines()
        .skip(1)
        .map(|l| l.split(',').collect())
        .collect();
    assert_eq!(rows.len(), 2, "{parts}");
    assert!(
        rows.iter()
            .all(|r| r[10] == "drop" && r[11].parse::<f64>().unwrap() > 1.0),
        "{parts}"
    );
    let w = d.join("w.json");
    std::fs::write(&w, r#"[{"file":2,"tr":2,"a":6.33,"b":7.05}]"#).unwrap();
    let whole = text(&execute(&[
        "manual-table".into(),
        wiff.to_string_lossy().into_owned(),
        "--set".into(),
        format!("windows={}", w.display()),
    ]));
    let area = |r: &Vec<&str>| r[6].parse::<f64>().unwrap();
    let total: f64 = whole
        .lines()
        .nth(1)
        .map(|l| area(&l.split(',').collect()))
        .unwrap();
    let sum: f64 = rows.iter().map(area).sum();
    assert!((sum - total).abs() <= 1e-4 * total, "{sum} vs {total}");
    std::fs::write(
        &w,
        format!(
            r#"[{{"file":2,"tr":2,"a":6.33,"b":7.05,"splits":[{}]}}]"#,
            at[0]
        ),
    )
    .unwrap();
    let table = text(&execute(&[
        "manual-table".into(),
        wiff.to_string_lossy().into_owned(),
        "--set".into(),
        format!("windows={}", w.display()),
    ]));
    assert_eq!(
        parts, table,
        "the bytes of manual-table for the same window"
    );
    assert_eq!(
        text(&run(&wiff, &[&PAIR[..], &["depth=1"]].concat())),
        "{\"splits\":[]}"
    );
    for bad in [
        &["tr=2", "a=6.33"][..],
        &[&PAIR[..], &["depth=2"]].concat(),
        &[&PAIR[..], &["baseline=valley"]].concat(),
        &[&PAIR[..], &["line=1,2"]].concat(),
        &["file=9", "tr=0", "a=1", "b=2"],
    ] {
        assert_ne!(run(&wiff, bad).code, 0, "{bad:?}");
    }
    let _ = std::fs::remove_dir_all(&d);
}
