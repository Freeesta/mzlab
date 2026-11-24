//! `peak table --traces`: the `trace xic` answers of two demo full-scan files give the same table as the page's list built from the same
//! points, the page's list passes unchanged, and a JSON of another shape is refused instead of giving an empty table.

use std::path::{Path, PathBuf};

use crate::execute;

fn run(args: &[&str]) -> crate::Outcome {
    execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn demo(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mzlab/web/esempi")
        .join(name);
    p.display().to_string()
}

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_traces_in_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn xic_answers_of_many_files_are_integrated_in_one_call() {
    let d = dir();
    let mut sets = Vec::new();
    for f in ["FullScan_t0.mzML", "FullScan_t5.mzML"] {
        let out = d.join(format!("{f}.json"));
        let o = run(&[
            "trace",
            "xic",
            &demo(f),
            "--mz",
            "364.4,194.2",
            "--da",
            "0.3",
            "--out",
            &out.display().to_string(),
        ]);
        assert_eq!(o.code, 0, "{}", o.stderr);
        sets.push(out.display().to_string());
    }
    let auto = run(&[
        "peak", "table", "--traces", &sets[0], "--traces", &sets[1], "--auto",
    ]);
    assert_eq!(auto.code, 0, "{}", auto.stderr);
    let v: serde_json::Value = serde_json::from_slice(&auto.stdout).unwrap();
    let windows = v["windows"].as_array().unwrap();
    assert!(
        windows.iter().any(|w| w["file"] == 1),
        "peaks of the second file: {v}"
    );
    // the page's own list with the same points gives the same bytes
    let mut page = Vec::new();
    for s in &sets {
        let t: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(s).unwrap()).unwrap();
        let tr: Vec<serde_json::Value> = t["traces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| {
                let pts = x["points"].as_array().unwrap();
                serde_json::json!({"rt": pts.iter().map(|p| p[0].clone()).collect::<Vec<_>>(),
                    "y": pts.iter().map(|p| p[1].clone()).collect::<Vec<_>>(),
                    "name": x["id"], "q1": x["evidence"]["mz"]})
            })
            .collect();
        page.push(serde_json::json!({"name": t["file"], "transitions": tr}));
    }
    let list = d.join("page.json");
    std::fs::write(&list, serde_json::to_string(&page).unwrap()).unwrap();
    let same = run(&[
        "peak",
        "table",
        "--traces",
        &list.display().to_string(),
        "--auto",
    ]);
    assert_eq!(same.code, 0, "{}", same.stderr);
    assert_eq!(same.stdout, auto.stdout);
    // a list of samples passes unchanged: the bytes of the engine on that text
    let text = std::fs::read_to_string(&list).unwrap();
    assert_eq!(
        crate::traces_in::read(std::slice::from_ref(&list)).unwrap(),
        text
    );
}

#[test]
fn another_shape_is_refused_with_its_code() {
    let d = dir();
    let bad = d.join("bad.json");
    std::fs::write(&bad, "{\"schema\":\"peak-auto/1\",\"peaks\":[]}").unwrap();
    let o = run(&[
        "peak",
        "table",
        "--traces",
        &bad.display().to_string(),
        "--auto",
        "--json",
    ]);
    assert_eq!(o.code, crate::exit::USAGE, "{}", o.stderr);
    assert!(o.stderr.contains("trace-set/1"), "{}", o.stderr);
    let o = run(&[
        "peak",
        "table",
        "--traces",
        &d.join("none.json").display().to_string(),
        "--auto",
    ]);
    assert_eq!(o.code, crate::exit::INPUT, "{}", o.stderr);
}

/// A file that is not JSON at all (a trace CSV) is the same refusal, alone or after a good file, never a parse error of exit 4.
#[test]
fn a_csv_is_refused_as_a_shape_not_read_as_json() {
    let d = dir();
    let csv = d.join("trace.csv");
    std::fs::write(&csv, "rt;y\n1.0;10\n1.1;20\n").unwrap();
    let good = d.join("good.json");
    std::fs::write(
        &good,
        "{\"schema\":\"trace/1\",\"id\":\"a\",\"points\":[[1.0,1.0]]}",
    )
    .unwrap();
    for files in [vec![&csv], vec![&good, &csv]] {
        let mut args = vec!["peak".to_string(), "table".to_string()];
        for f in files {
            args.push("--traces".into());
            args.push(f.display().to_string());
        }
        args.push("--auto".into());
        let o = crate::execute(&args);
        assert_eq!(o.code, crate::exit::USAGE, "{}", o.stderr);
        assert!(o.stderr.contains("trace-set/1"), "{}", o.stderr);
    }
}

/// `[]` or a list of numbers is no list of samples: refused like any other shape.
#[test]
fn an_empty_list_or_a_list_of_numbers_is_refused() {
    let d = dir();
    for (name, body) in [("empty.json", "[]"), ("numbers.json", "[1,2]")] {
        let p = d.join(name);
        std::fs::write(&p, body).unwrap();
        let o = run(&[
            "peak",
            "table",
            "--traces",
            &p.display().to_string(),
            "--auto",
        ]);
        assert_eq!(o.code, crate::exit::USAGE, "{name}: {}", o.stderr);
        assert!(o.stderr.contains("trace-set/1"), "{name}: {}", o.stderr);
    }
}

/// Two traces of one multi-sample file (`trace xic --sample 1`, `--sample 2`: the sample is a number) stay two samples, `x.wiff#1` and `x.wiff#2`.
#[test]
fn the_samples_of_one_multi_sample_file_keep_their_numbers() {
    let doc = |n: u32| {
        format!(
            "{{\"schema\":\"trace-set/1\",\"file\":\"x.wiff\",\"sample\":{n},\"traces\":[{{\"schema\":\"trace/1\",\"id\":\"xic 100\",\"evidence\":{{\"mz\":100}},\"points\":[[1.0,0],[1.1,5],[1.2,0]]}}]}}"
        )
    };
    let d = dir();
    let (a, b) = (d.join("s1.json"), d.join("s2.json"));
    std::fs::write(&a, doc(1)).unwrap();
    std::fs::write(&b, doc(2)).unwrap();
    let list = crate::traces_in::read(&[a.clone(), b.clone()]).unwrap();
    let v: serde_json::Value = serde_json::from_str(&list).unwrap();
    let names: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["x.wiff#1", "x.wiff#2"]);
    // a file of one sample (null) keeps its plain name
    let one = d.join("one.json");
    std::fs::write(&one, doc(1).replace("\"sample\":1", "\"sample\":null")).unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&crate::traces_in::read(&[one]).unwrap()).unwrap();
    assert_eq!(v[0]["name"], "x.wiff");
}
