//! `mzlab-cli table compare` on a small invented mzmine "full" table (a reference, two treated times, a blank): JSON and CSV agree, `--dry-run` computes no row,
//! the exit codes (2, 4, 5, 6), the Flow step `table.compare@1` writes the bytes the command prints, and on the `tools/lr_synthetic.py` files the CSV of a run and
//! its `.mzfeat` give the same rows (python3 and numpy needed; `MZLAB_GOLDEN_STRICT=1` fails instead of skipping).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::json::{parse, Value};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};

use crate::execute;

fn run(args: &[&str]) -> (u8, String, String) {
    let o = execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    (o.code, String::from_utf8(o.stdout).unwrap(), o.stderr)
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_cmp_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A table in the shape of the mzmine "full" CSV: id, m/z, RT, then area, height and state of every file.
const FILES: [&str; 5] = ["s_t0", "s_t5", "s_t10", "s_t20", "MQ_blank"];
const ROWS: [(f64, f64, [f64; 5]); 5] = [
    (301.1, 5.0, [1e6, 1e6, 1e6, 1e6, 0.0]), // present everywhere: unchanged
    (317.1, 6.5, [0.0, 8e5, 2e6, 3e6, 0.0]), // absent at t0, rises: candidate
    (333.1, 7.0, [0.0, 2e4, 3e4, 2e4, 2e4]), // as high in the blank: blank artefact
    (349.1, 8.0, [0.0, 500.0, 800.0, 600.0, 500.0]), // below the pseudocount
    (365.1, 9.0, [1e6, 2e5, 1e5, 5e4, 0.0]), // falls
];

fn csv() -> String {
    let mut s = String::from("id,mz,rt,rt_range:min,rt_range:max,charge");
    for f in FILES {
        s += &format!(",datafile:{f}:area,datafile:{f}:height,datafile:{f}:feature_state");
    }
    s.push('\n');
    for (i, (mz, rt, a)) in ROWS.iter().enumerate() {
        s += &format!("{},{mz},{rt},{},{},0", i + 1, rt - 0.1, rt + 0.1);
        for v in a {
            if *v > 0.0 {
                s += &format!(",{v},{},DETECTED", v / 10.0);
            } else {
                s += ",,,UNKNOWN";
            }
        }
        s.push('\n');
    }
    s
}

/// The same invented areas as a `.mzfeat`, the table type of a plan.
fn mzfeat() -> Vec<u8> {
    let mut t = Aligned::with_capacity(FILES.len(), ROWS.len());
    for (mz, rt, areas) in ROWS {
        t.mz.push(mz);
        t.rt.push(rt);
        for a in areas {
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state
                .push(if a > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    let origin = Origin {
        inputs: FILES
            .iter()
            .map(|n| (n.to_string(), String::new()))
            .collect(),
        ..Origin::default()
    };
    write_table(&t, &Recipe::default(), &origin)
}

/// The file the instance `id` of a plan wrote in `work`.
fn made(work: &Path, id: &str) -> PathBuf {
    std::fs::read_dir(work)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.file_stem().is_some_and(|s| s.to_string_lossy() == id))
        .unwrap_or_else(|| panic!("no file of {id} in {work:?}"))
}

fn table(tag: &str) -> (PathBuf, String) {
    let d = dir(tag);
    let p = d.join("feats.csv");
    std::fs::write(&p, csv()).unwrap();
    (d, p.to_str().unwrap().to_string())
}

const SEL: [&str; 6] = ["--ref", "s_t0", "--blank", "MQ*", "--pseudocount", "1000"];

fn doc(args: &[&str]) -> Value {
    let (code, out, err) = run(args);
    assert_eq!(code, 0, "{err}");
    parse(out.as_bytes()).unwrap()
}

fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).unwrap().str().unwrap()
}

#[test]
fn json_and_csv_say_the_same() {
    let (_, t) = table("same");
    let j = doc(&[&["table", "compare", &t][..], &SEL].concat());
    assert_eq!(text(&j, "schema"), "compare/1");
    let rows = j.get("rows").unwrap().arr();
    assert_eq!(rows.len(), ROWS.len());
    let class = |k: usize| text(&rows[k], "class").to_string();
    assert_eq!(class(0), "unchanged");
    assert_eq!(class(1), "candidate");
    assert_eq!(class(2), "blank_artefact");
    assert_eq!(class(3), "no_signal");
    assert_eq!(class(4), "decreasing");
    assert_eq!(text(&rows[1], "shape"), "rises");
    let (code, out, err) = run(&[&["table", "compare", &t, "--format", "csv"][..], &SEL].concat());
    assert_eq!(code, 0, "{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), rows.len() + 1);
    let head: Vec<&str> = lines[0].split(',').collect();
    let at = |name: &str| head.iter().position(|h| *h == name).unwrap();
    for (k, line) in lines[1..].iter().enumerate() {
        let c: Vec<&str> = line.split(',').collect();
        assert_eq!(c[at("id")], text(&rows[k], "id"));
        assert_eq!(c[at("class")], text(&rows[k], "class"));
        let reasons: Vec<&str> = rows[k]
            .get("reasons")
            .unwrap()
            .arr()
            .iter()
            .map(|r| r.str().unwrap())
            .collect();
        assert_eq!(c[at("reasons")], reasons.join(";"));
        let ratio: f64 = c[at("ratio_ref")].parse().unwrap();
        assert!((ratio - rows[k].get("ratio_ref").unwrap().f64().unwrap()).abs() < 1e-9);
    }
}

#[test]
fn a_dry_run_prints_the_plan_and_no_row() {
    let (_, t) = table("dry");
    let (code, out, _) = run(&[&["table", "compare", &t, "--dry-run"][..], &SEL].concat());
    assert_eq!(code, 0);
    let v = parse(out.as_bytes()).unwrap();
    assert_eq!(text(&v, "schema"), "compare-plan/1");
    assert!(v.get("rows").is_none());
    let (_, one, _) = run(&[&["table", "compare", &t][..], &SEL].concat());
    // the plan names the same columns the run reads
    let real = parse(one.as_bytes()).unwrap();
    assert_eq!(
        v.get("roles")
            .unwrap()
            .get("treated")
            .unwrap()
            .get("columns")
            .unwrap()
            .arr()
            .len(),
        real.get("roles")
            .unwrap()
            .get("treated")
            .unwrap()
            .get("columns")
            .unwrap()
            .arr()
            .len()
    );
}

#[test]
fn usage_and_input_errors_have_their_exit_codes() {
    let (d, t) = table("exit");
    let code = |a: &[&str]| run(a).0;
    assert_eq!(
        code(&["table", "compare", &t, "--ref", "s_t0"]),
        2,
        "no --blank"
    );
    assert_eq!(
        code(&["table", "compare", &t, "--ref", "s_t*", "--blank", "s_t5"]),
        2,
        "a column in both lists"
    );
    assert_eq!(
        code(&[
            "table",
            "compare",
            &t,
            "--ref",
            "role:reference",
            "--blank",
            "MQ*"
        ]),
        2,
        "role: needs --samples"
    );
    assert_eq!(
        code(&["table", "compare", &t, "--ref", "s_t0", "--blank", "MQ*", "--format", "xml"]),
        2
    );
    assert_eq!(
        code(&[
            "table",
            "compare",
            d.join("none.csv").to_str().unwrap(),
            "--ref",
            "s_t0",
            "--blank",
            "MQ*"
        ]),
        4
    );
    assert_eq!(
        code(&[
            "table",
            "compare",
            &t,
            "--ref",
            "s_t0",
            "--blank",
            "MQ*",
            "--samples",
            d.join("none.json").to_str().unwrap()
        ]),
        4
    );
}

#[test]
fn a_selector_that_matches_nothing_and_an_unwritable_out_have_their_codes() {
    let (d, t) = table("exit_sel");
    let code = |a: &[&str]| run(a).0;
    assert_eq!(
        code(&["table", "compare", &t, "--ref", "s_t0", "--blank", "nothing*"]),
        5
    );
    let (c, _, err) = run(&[
        "table", "compare", &t, "--ref", "nothing*", "--blank", "MQ*",
    ]);
    assert_eq!(c, 5);
    assert!(
        err.contains("err.compare.selector") && err.contains("s_t0"),
        "{err}"
    );
    let bad = d.join("no_folder").join("out.json");
    assert_eq!(
        code(
            &[
                &["table", "compare", &t, "--out", bad.to_str().unwrap()][..],
                &SEL
            ]
            .concat()
        ),
        6
    );
    // --out writes the same bytes the standard output carries
    let out = d.join("out.json");
    assert_eq!(
        code(
            &[
                &["table", "compare", &t, "--out", out.to_str().unwrap()][..],
                &SEL
            ]
            .concat()
        ),
        0
    );
    let (_, stdout, _) = run(&[&["table", "compare", &t][..], &SEL].concat());
    assert_eq!(
        std::fs::read_to_string(&out).unwrap().trim_end(),
        stdout.trim_end()
    );
}

#[test]
fn the_flow_step_writes_the_bytes_of_the_command() {
    let (d, t) = table("step");
    std::fs::write(d.join("feats.mzfeat"), mzfeat()).unwrap();
    let m = d.join("feats.mzfeat");
    let (_, cli, _) = run(&[&["table", "compare", m.to_str().unwrap()][..], &SEL].concat());
    // the .mzfeat and the CSV of the same areas give the same classes
    let (_, from_csv, _) = run(&[&["table", "compare", &t][..], &SEL].concat());
    let classes = |j: &str| -> Vec<String> {
        parse(j.as_bytes())
            .unwrap()
            .get("rows")
            .unwrap()
            .arr()
            .iter()
            .map(|r| text(r, "class").to_string())
            .collect()
    };
    assert_eq!(classes(&cli), classes(&from_csv));
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"feats.mzfeat","type":"table"}],"steps":[
        {"id":"c","step":"table.compare@1","in":{"table":"file:feats.mzfeat"},"params":{"ref":"s_t0","blank":"MQ*","pseudocount":1000}}]}"#;
    let work = d.join("work");
    std::fs::create_dir_all(&work).unwrap();
    crate::flow::run_plan(plan.as_bytes(), &d, &work, 2, None)
        .unwrap_or_else(|e| panic!("{} {:?}", e.key, e.params));
    assert_eq!(
        std::fs::read_to_string(made(&work, "c")).unwrap(),
        cli.trim_end()
    );
}

#[test]
fn a_ref_that_matches_nothing_in_a_plan_is_the_selector_error() {
    let (d, _) = table("plan_err");
    std::fs::write(d.join("feats.mzfeat"), mzfeat()).unwrap();
    let plan = r#"{"format":"mzflow/2","inputs":[{"name":"feats.mzfeat","type":"table"}],"steps":[
        {"id":"c","step":"table.compare@1","in":{"table":"file:feats.mzfeat"},"params":{"ref":"zz*","blank":"MQ*"}}]}"#;
    let work = d.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let r = crate::flow::run_plan(plan.as_bytes(), &d, &work, 2, None);
    let text = match r {
        Ok((_, prov)) => prov,
        Err(e) => format!("{} {:?}", e.key, e.params),
    };
    assert!(text.contains("err.compare.selector"), "{text}");
}

fn python_files(tag: &str) -> Option<PathBuf> {
    let d = dir(tag);
    let tool = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/lr_synthetic.py");
    let ok = Command::new("python3")
        .arg(tool)
        .arg(&d)
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return None;
    }
    Some(d)
}

/// The same files as a flow run in `d`: its folder holds `gap.mzfeat` and the CSV (`csv.csv`) exported from it.
fn flow_run(d: &Path, names: &[&str]) -> PathBuf {
    let raw: Vec<String> = names
        .iter()
        .map(|n| format!(r#"{{"name":"{n}","type":"raw"}}"#))
        .collect();
    let src: Vec<String> = names.iter().map(|n| format!(r#""file:{n}""#)).collect();
    let full: Vec<String> = names
        .iter()
        .map(|n| d.join(n).to_str().unwrap().to_string())
        .collect();
    let (code, samples, err) = run(&[
        &["sample", "table"][..],
        &full.iter().map(String::as_str).collect::<Vec<_>>(),
    ]
    .concat());
    assert_eq!(code, 0, "{err}");
    std::fs::write(d.join("samples.json"), samples).unwrap();
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{},{{"name":"samples.json","type":"samples"}}],"steps":[
        {{"id":"idx","step":"index.mzidx@1","in":{{"source":[{}]}}}},
        {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
        {{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}},
        {{"id":"gap","step":"features.gapfill@1","in":{{"table":"ali.table","index":["idx.index"]}}}},
        {{"id":"csv","step":"export.table_csv@1","in":{{"table":"gap.table","samples":"file:samples.json"}}}}]}}"#,
        raw.join(","),
        src.join(",")
    );
    std::fs::write(d.join("plan.json"), &plan).unwrap();
    let out = d.join("run");
    let (code, _, err) = run(&[
        "flow",
        "run",
        d.join("plan.json").to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{err}");
    out
}

/// The CSV the `features` command writes on the synthetic files compares like the `.mzfeat` of a flow run on the same files: the CSV is rounded, so the ids,
/// classes, reasons and shapes are asserted exactly and the numbers within the rounding of the export.
#[test]
fn a_features_csv_and_the_mzfeat_of_the_same_run_give_the_same_rows() {
    let Some(d) = python_files("synth") else {
        return;
    };
    let names = [
        "LR_bench-noise-s1.mzML",
        "LR_bench-blank-s1.mzML",
        "LR_bench-s1.mzML",
    ];
    // the smoke of the command on the CSV of `features`
    let args: Vec<String> = ["features"]
        .into_iter()
        .map(String::from)
        .chain(
            names
                .iter()
                .map(|n| d.join(n).to_str().unwrap().to_string()),
        )
        .collect();
    let o = execute(&args);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let feats = d.join("features.csv");
    std::fs::write(&feats, &o.stdout).unwrap();
    let sel = ["--ref", "LR_bench-noise*", "--blank", "LR_bench-blank*"];
    let j = doc(&[&["table", "compare", feats.to_str().unwrap()][..], &sel].concat());
    assert!(!j.get("rows").unwrap().arr().is_empty());
    let out = flow_run(&d, &names);
    let a = doc(&[
        &["table", "compare", out.join("csv.csv").to_str().unwrap()][..],
        &sel,
    ]
    .concat());
    let b = doc(&[
        &["table", "compare", out.join("gap.mzfeat").to_str().unwrap()][..],
        &sel,
    ]
    .concat());
    let (ra, rb) = (a.get("rows").unwrap().arr(), b.get("rows").unwrap().arr());
    assert_eq!(ra.len(), rb.len());
    assert_eq!(a.get("counts").unwrap().entries().len(), 5);
    for (x, y) in ra.iter().zip(rb) {
        assert_eq!(text(x, "class"), text(y, "class"));
        assert_eq!(
            x.get("reasons").unwrap().arr().len(),
            y.get("reasons").unwrap().arr().len()
        );
        assert_eq!(
            x.get("shape").and_then(|s| s.str()),
            y.get("shape").and_then(|s| s.str())
        );
        for k in ["ref_level", "blank_level", "max_treated"] {
            let (p, q) = (
                x.get(k).unwrap().f64().unwrap(),
                y.get(k).unwrap().f64().unwrap(),
            );
            assert!((p - q).abs() <= 1e-6 * q.abs().max(1.0), "{k}: {p} {q}");
        }
    }
}

/// The output on the invented `.mzfeat` is the golden file `tests/golden/compare_lr.json` (`MZLAB_UPDATE_COMPARE=1` rewrites it).
#[test]
fn the_output_on_the_invented_table_is_the_golden_file() {
    let d = dir("golden");
    std::fs::write(d.join("feats.mzfeat"), mzfeat()).unwrap();
    let (code, out, err) = run(&[
        &["table", "compare", d.join("feats.mzfeat").to_str().unwrap()][..],
        &SEL,
    ]
    .concat());
    assert_eq!(code, 0, "{err}");
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/compare_lr.json");
    if std::env::var_os("MZLAB_UPDATE_COMPARE").is_some() {
        std::fs::write(&golden, &out).unwrap();
    }
    assert_eq!(
        out,
        std::fs::read_to_string(&golden).unwrap(),
        "run with MZLAB_UPDATE_COMPARE=1 to rewrite it"
    );
}
