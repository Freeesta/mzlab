//! The sixth chain, a kinetic LR campaign (`docs/agenti/recipes.md`): a synthetic unit-resolution series of two conditions, a shared t0 and a blank, written as
//! vendor files (mzXML), taken from the sample listing to the kinetics of every product per condition with the plan template of
//! `docs/recipes/`. The call count is asserted, the target table of the plan is the bytes of `quant targets`, one cell is the answer of
//! `trace xic` into `peak auto-lr`, and every command, step and `--set` the recipe cites exists.

use std::collections::HashMap;
use std::path::Path;

use mzlab_core::json::{parse, Value};

use super::kinetic_series::{series, FILES, OFF};
use super::{dir, s, Chain};

const PLAN: &str = include_str!("../../../docs/recipes/kinetic-lr.mzflow.json");
const TARGETS: &str = include_str!("../../../docs/recipes/kinetic-lr.targets.csv");
const PARENT: &str = include_str!("../../../docs/recipes/kinetic-lr.parent.json");
const RECIPE: &str = include_str!("../../../docs/agenti/recipes.md");

fn text(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::str)
        .unwrap_or_default()
        .to_string()
}

/// The rows of a CSV as maps of its header.
fn rows(csv: &str) -> Vec<HashMap<String, String>> {
    let mut l = csv.lines();
    let head: Vec<&str> = l.next().unwrap().split(',').collect();
    l.map(|r| {
        head.iter()
            .map(|h| h.to_string())
            .zip(r.split(',').map(String::from))
            .collect()
    })
    .collect()
}

#[test]
fn kinetic_lr_campaign() {
    let d = dir("wf_kinetic");
    let (_, vendor) = series(&d);
    let (mzml, run) = (d.join("mzml"), d.join("run"));
    let mut c = Chain::new();
    let converted = to_mzml(&mut c, &vendor, &mzml);
    roles(&mut c, &converted, &mzml);
    run_plan(&mut c, &d, &mzml, &run);
    let cell = evidence(&run);
    by_hand(&mut c, &d, &mzml, &cell);
    c.done(9);
    kinetics(&run);
    shortcut_is_the_flow_table(&d, &mzml, &converted, &run);
    let _ = std::fs::remove_dir_all(&d);
}

/// Calls 1-3: the samples of the vendor files, then one mzML per sample named after it (a dry run first, which writes nothing).
fn to_mzml(c: &mut Chain, vendor: &[String], mzml: &Path) -> Vec<String> {
    let mut args = vec!["sample", "list"];
    args.extend(vendor.iter().map(String::as_str));
    args.push("--json");
    let listed = c.json(&args, "sample-list/1");
    assert_eq!(listed.get("files").unwrap().arr().len(), 8);
    let mut conv = vec!["file", "convert"];
    conv.extend(vendor.iter().map(String::as_str));
    conv.extend(["--out", s(mzml), "--name", "sample", "--json"]);
    let mut dry = conv.clone();
    dry.push("--dry-run");
    let plan = c.json(&dry, "convert/1");
    assert_eq!(plan.get("items").unwrap().arr().len(), 8);
    assert!(!mzml.join("t0.mzML").exists(), "a dry run writes nothing");
    c.json(&conv, "convert/1");
    FILES
        .iter()
        .map(|f| s(&mzml.join(format!("{}.mzML", f.0))).to_string())
        .collect()
}

/// Call 4: roles, times and series proposed from the names, each logged on stderr (`auto:` lines), never silent; the t0 has no series, so
/// both conditions share it.
fn roles(c: &mut Chain, converted: &[String], mzml: &Path) {
    let mut table = vec!["sample", "table"];
    table.extend(converted.iter().map(String::as_str));
    let samples = c.call(&table);
    assert_eq!(
        samples.stderr.matches("auto: ").count(),
        8,
        "{}",
        samples.stderr
    );
    std::fs::write(mzml.join("samples.json"), &samples.stdout).unwrap();
    let roles = parse(&samples.stdout).unwrap();
    let got: Vec<(String, String)> = roles
        .get("samples")
        .unwrap()
        .arr()
        .iter()
        .map(|x| (text(x, "kind"), text(x, "series")))
        .collect();
    assert!(
        got.contains(&("blank".into(), String::new()))
            && got.contains(&("t0".into(), String::new())),
        "{got:?}"
    );
    let series: Vec<&str> = got
        .iter()
        .filter(|g| g.0 == "sample")
        .map(|g| g.1.as_str())
        .collect();
    assert_eq!(series, ["Fe", "Fe", "Fe", "Cu", "Cu", "Cu"], "{got:?}");
}

/// Calls 5-7: the plan template checked, dry-run on the inputs, run.
fn run_plan(c: &mut Chain, d: &Path, mzml: &Path, run: &Path) {
    std::fs::write(mzml.join("targets.csv"), TARGETS).unwrap();
    std::fs::write(mzml.join("parent.json"), PARENT).unwrap();
    let plan = d.join("kinetic-lr.mzflow.json");
    std::fs::write(&plan, PLAN).unwrap();
    let check = c.json(&["flow", "check", s(&plan), "--json"], "flowcheck/1");
    assert_eq!(check.get("valid").and_then(Value::bool), Some(true));
    let base = [
        "flow",
        "run",
        s(&plan),
        "--in",
        s(mzml),
        "--out",
        s(run),
        "--json",
    ];
    let mut dry = base.to_vec();
    dry.push("--dry-run");
    c.json(&dry, "flowdry/1");
    let done = c.json(&base, "flowrun/1");
    assert_eq!(done.get("code").and_then(Value::f64), Some(0.0));
}

/// Calls 8-9: the cell of P229 in `Fe_t15` by hand, `trace xic` at the measured offset into `peak auto-lr` with the step's settings.
fn by_hand(c: &mut Chain, d: &Path, mzml: &Path, cell: &HashMap<String, String>) {
    let f = mzml.join("Fe_t15.mzML");
    let off = &cell["mz_offset_da"];
    let xic = c.call(&[
        "trace",
        "xic",
        s(&f),
        "--mz",
        "229.1",
        "--da",
        "0.5",
        "--polarity",
        "+",
        "--mz-offset",
        off,
        "--format",
        "csv",
    ]);
    let trace = d.join("P229_Fe_t15.csv");
    std::fs::write(&trace, &xic.stdout).unwrap();
    let lr = c.call(&[
        "peak",
        "auto-lr",
        s(&trace),
        "--from",
        "6.6",
        "--to",
        "7.6",
        "--set",
        "sn=0",
        "--set",
        "min_pts=2",
    ]);
    same_cell(cell, &parse(&lr.stdout).unwrap());
}

/// The target table: the states per condition and the offset measured on the parent, with its evidence and choice in the log; returns the
/// row of P229 in `Fe_t15` that the agent checks by hand.
fn evidence(run: &Path) -> HashMap<String, String> {
    let qt = std::fs::read_to_string(run.join("qt.csv")).unwrap();
    let log: serde_json::Value =
        serde_json::from_slice(&std::fs::read(run.join("qt.log.json")).unwrap()).unwrap();
    let choice = log["choices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["key"] == "offset_reference.+")
        .cloned();
    assert_eq!(
        choice.as_ref().map(|c| &c["value"]),
        Some(&serde_json::json!("parent")),
        "the automatic choice is logged: {}",
        log["choices"]
    );
    let ev = &log["files"][2]["offsets"][0]["evidence"];
    assert!(
        ev["status"] == "ok" && ev["n_scans"].as_u64().unwrap() > 3,
        "the offset of Fe_t5 has its evidence: {ev}"
    );
    assert!(qt.starts_with("file,id,area,height,sn,rt,flag,state,points,shape,start_rt,end_rt,peaks_in_window,smooth_half,mz_offset_da\n"));
    let r = rows(&qt);
    assert_eq!(r.len(), 8 * 3, "one row per file and target, never hidden");
    let state = |f: &str, id: &str| {
        r.iter()
            .find(|x| x["file"] == f && x["id"] == id)
            .map(|x| x["state"].clone())
            .unwrap()
    };
    assert_eq!(state("Blank_01.mzML", "parent"), "absent");
    assert_eq!(state("t0.mzML", "parent"), "detected");
    assert_eq!(state("t0.mzML", "P305"), "absent");
    for f in ["Fe_t5", "Fe_t15", "Fe_t30", "Cu_t5", "Cu_t15", "Cu_t30"] {
        assert_eq!(state(&format!("{f}.mzML"), "P305"), "detected", "{f}");
        let p229 = if f.starts_with("Fe") {
            "detected"
        } else {
            "absent"
        };
        assert_eq!(state(&format!("{f}.mzML"), "P229"), p229, "{f}");
    }
    let off: f64 = r.iter().find(|x| x["file"] == "Fe_t15.mzML").unwrap()["mz_offset_da"]
        .parse()
        .unwrap();
    assert!(
        (off - OFF).abs() < 0.05,
        "offset measured on the parent: {off}"
    );
    r.into_iter()
        .find(|x| x["file"] == "Fe_t15.mzML" && x["id"] == "P229")
        .unwrap()
}

/// `peak auto-lr` over the `trace xic` of the cell gives the numbers of the table (the step rounds area and height to 1 decimal, S/N to 2).
fn same_cell(cell: &HashMap<String, String>, lr: &Value) {
    let p = lr.get("peaks").unwrap().arr();
    let top = p
        .iter()
        .max_by(|a, b| num(a, "height").total_cmp(&num(b, "height")))
        .expect("a peak");
    for (k, tol) in [
        ("area", 0.05),
        ("height", 0.05),
        ("sn", 0.005),
        ("rt", 1e-9),
    ] {
        let want: f64 = cell[k].parse().unwrap();
        assert!(
            (num(top, k) - want).abs() <= tol,
            "{k}: peak auto-lr {} vs table {want}",
            num(top, k)
        );
    }
}

fn num(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::f64).unwrap()
}

/// The Dig: one run over both conditions with the shared t0, the evidence and the kinetics per condition, the parent's rate per condition; the
/// LR table exported with its samples.
fn kinetics(run: &Path) {
    let dig: serde_json::Value =
        serde_json::from_slice(&std::fs::read(run.join("dig.json")).unwrap()).unwrap();
    let conds = dig["groups"]["conditions"].as_array().unwrap();
    let ids: Vec<&str> = conds.iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["Fe", "Cu"], "conditions in the order of the files");
    for c in conds {
        assert_eq!(
            c["controls"]["t0"].as_array().unwrap().len(),
            1,
            "the shared t0: {c}"
        );
        assert_eq!(c["controls"]["blank"].as_array().unwrap().len(), 1, "{c}");
    }
    // the first-order rate of the parent per condition is the one of the series (0.035 and 0.015 per min) within 15 %
    for (id, k) in [("Fe", 0.035), ("Cu", 0.015)] {
        let p = &conds.iter().find(|c| c["id"] == id).unwrap()["parent"];
        let got = p["k"].as_f64().unwrap();
        assert!((got / k - 1.0).abs() < 0.15, "{id}: k {got} vs {k}: {p}");
    }
    let rows = dig["groups"]["rows"].as_array().unwrap();
    let near = |mz: f64| {
        rows.iter()
            .find(|r| (r["mz"].as_f64().unwrap() - mz).abs() < 0.5)
    };
    let p305 = near(305.0 + OFF).expect("P305 is a row");
    assert_eq!(p305["in_all"], true, "{p305}");
    let p229 = near(229.1 + OFF).expect("P229 is a row");
    assert_eq!(p229["present_in"], serde_json::json!(["Fe"]), "{p229}");
    for r in [p305, p229] {
        for c in r["cond"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["present"] == true)
        {
            assert_eq!(
                c["kin"].as_array().unwrap().len(),
                3,
                "three times per condition: {c}"
            );
        }
    }
    let csv = std::fs::read_to_string(run.join("exp.csv")).unwrap();
    assert!(csv.lines().count() > 3, "the LR table has rows: {csv}");
}

/// The one-call CLI route of the target step (`quant targets`) writes the bytes of the Flow table and log for the same files and settings.
fn shortcut_is_the_flow_table(d: &Path, mzml: &Path, files: &[String], run: &Path) {
    let (csv, log) = (d.join("qt.csv"), d.join("qt.log.json"));
    let t = s(&mzml.join("targets.csv")).to_string();
    let mut a: Vec<&str> = vec![
        "quant",
        "targets",
        "--targets",
        &t,
        "--offset",
        "auto",
        "--blank-pattern",
        "blank*",
    ];
    a.extend(files.iter().map(String::as_str));
    a.extend(["--out", s(&csv), "--log", s(&log)]);
    let o = crate::execute(&a.iter().map(|x| x.to_string()).collect::<Vec<_>>());
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        std::fs::read(&csv).unwrap() == std::fs::read(run.join("qt.csv")).unwrap(),
        "quant targets CSV = Flow qt.csv"
    );
    assert!(
        std::fs::read(&log).unwrap() == std::fs::read(run.join("qt.log.json")).unwrap(),
        "quant targets log = Flow qt.log.json"
    );
}

/// Every `mzlab-cli ...` of the recipe is a command of the table, every `id@N` a step of the binary, and every `--set INSTANCE.NAME` a
/// parameter of that instance's step in the plan template.
#[test]
fn the_recipe_cites_only_what_exists() {
    let cat =
        String::from_utf8(crate::execute(&["commands".into(), "--json".into()]).stdout).unwrap();
    let cat = parse(cat.as_bytes()).unwrap();
    let paths: Vec<String> = cat
        .get("commands")
        .unwrap()
        .arr()
        .iter()
        .map(|c| text(c, "path"))
        .collect();
    let mut cited = 0;
    for (n, line) in RECIPE.lines().enumerate() {
        for cmd in line.split("mzlab-cli ").skip(1) {
            let words: Vec<&str> = cmd
                .split(|ch: char| !(ch.is_ascii_lowercase() || ch == '-' || ch == ' '))
                .next()
                .unwrap()
                .split(' ')
                .collect();
            let ok = (1..=words.len()).any(|k| paths.contains(&words[..k].join(" ")));
            assert!(
                ok,
                "recipes.md:{}: `mzlab-cli {}` is not a command",
                n + 1,
                words.join(" ")
            );
            cited += 1;
        }
    }
    assert!(cited >= 10, "the recipe names its commands ({cited})");
    let plan: serde_json::Value = serde_json::from_str(PLAN).unwrap();
    for st in plan["steps"].as_array().unwrap() {
        let (step, id) = (st["step"].as_str().unwrap(), st["id"].as_str().unwrap());
        assert!(
            RECIPE.contains(step),
            "recipes.md names the step {step} of the template"
        );
        let o = crate::execute(&[
            "flow".into(),
            "steps".into(),
            "--json".into(),
            "--id".into(),
            step.into(),
        ]);
        assert_eq!(o.code, 0, "{step}: {}", o.stderr);
        let desc: serde_json::Value =
            serde_json::from_slice(o.stdout.split(|b| *b == b'\n').next().unwrap()).unwrap();
        let props = &desc["params"]["properties"];
        for key in sets() {
            if let Some(name) = key.strip_prefix(&format!("{id}.")) {
                assert!(
                    props.get(name).is_some(),
                    "recipes.md: --set {key} is not a parameter of {step}"
                );
            }
        }
    }
    let ids: Vec<&str> = plan["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|st| st["id"].as_str().unwrap())
        .collect();
    for key in sets().into_iter().filter(|k| k.contains('.')) {
        assert!(
            ids.contains(&key.split('.').next().unwrap()),
            "recipes.md: --set {key} names no instance of the template"
        );
    }
}

/// The keys of every `--set KEY=VALUE` of the recipe.
fn sets() -> Vec<&'static str> {
    RECIPE
        .split("--set ")
        .skip(1)
        .map(|set| set.split(['=', '`', ' ']).next().unwrap())
        .collect()
}
