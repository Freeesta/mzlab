//! `mzlab-cli flow run` on three synthetic mzML files: the base chain of the Flow canvas (index, detect, align, gapfill) gives the table of the page,
//! the run equals `run_plan` of the desktop route with 1 and 2 workers, `--set` reaches the parameters, `--dry-run` writes nothing, exit codes.

use std::path::{Path, PathBuf};

use crate::tests::{layered_table, mzml, page_table};

const BASE: &str = r#"{"format":"mzflow/2","inputs":[{"name":"s_t000.mzML","type":"raw"},{"name":"s_t010.mzML","type":"raw"},{"name":"s_t030.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:s_t000.mzML","file:s_t010.mzML","file:s_t030.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}},
{"id":"ali","step":"features.align@1","in":{"peaks":["det.peaks"]}},
{"id":"gap","step":"features.gapfill@1","in":{"table":"ali.table","index":["idx.index"]}}]}"#;

/// A folder with the three files and `plan.json` (the base chain); the files as (name, bytes).
pub(crate) fn fixture(tag: &str) -> (PathBuf, Vec<(String, Vec<u8>)>) {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_run_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let files: Vec<(String, Vec<u8>)> = [0.0, 10.0, 30.0]
        .iter()
        .map(|&t| (format!("s_t{:03}.mzML", t as u32), mzml(t)))
        .collect();
    for (n, b) in &files {
        std::fs::write(dir.join("data").join(n), b).unwrap();
    }
    std::fs::write(dir.join("plan.json"), BASE).unwrap();
    (dir, files)
}

pub(crate) fn flow_run(args: &[&str]) -> crate::Outcome {
    let mut a = vec!["flow".to_string(), "run".to_string()];
    a.extend(args.iter().map(|x| x.to_string()));
    crate::execute(&a)
}

pub(crate) fn p(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// The files of a run folder, `prov.json` and `corsa.json` without the times and the heap.
pub(crate) fn folder(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|f| f.is_file())
        .map(|f| {
            let name = f.file_name().unwrap().to_string_lossy().into_owned();
            let mut bytes = std::fs::read(&f).unwrap();
            if name == "prov.json" || name == "corsa.json" {
                bytes = untimed(&String::from_utf8(bytes).unwrap()).into_bytes();
            }
            (name, bytes)
        })
        .collect();
    v.sort();
    v
}

/// `text` with the numbers of `ms`, `total_ms` and `heap_peak_bytes` blanked.
pub(crate) fn untimed(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(k) = ["\"ms\":", "\"total_ms\":", "\"heap_peak_bytes\":"]
        .iter()
        .filter_map(|key| rest.find(key).map(|k| k + key.len()))
        .min()
    {
        out += &rest[..k];
        rest = rest[k..].trim_start_matches(|c: char| c.is_ascii_digit() || ".-e+".contains(c));
        out += "_";
    }
    out + rest
}

#[test]
fn the_base_chain_of_the_canvas_gives_the_table_of_the_page() {
    let (dir, files) = fixture("base");
    let out = dir.join("out");
    let o = flow_run(&[
        p(&dir.join("plan.json")),
        "--in",
        p(&dir.join("data")),
        "--out",
        p(&out),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stderr.starts_with("done: 4 steps"), "{}", o.stderr);
    for n in [
        "plan.json",
        "prov.json",
        "corsa.json",
        "idx-s_t030.mzidx",
        "det-s_t000.mzfeatf",
        "ali.mzfeat",
    ] {
        assert!(out.join(n).is_file(), "{n}");
    }
    assert_eq!(
        std::fs::read(out.join("gap.mzfeat")).unwrap(),
        page_table(&files, "").0,
        "align and fill"
    );
    assert_eq!(
        std::fs::read(out.join("ali.mzfeat")).unwrap(),
        page_table(&files, r#"{"fill":false}"#).0,
        "align alone"
    );

    // plan.json run again (inputs beside it) is the same run; the desktop route and two workers too
    let again = dir.join("again");
    std::fs::create_dir_all(&again).unwrap();
    for (n, b) in &files {
        std::fs::write(again.join(n), b).unwrap();
    }
    std::fs::copy(out.join("plan.json"), again.join("plan.json")).unwrap();
    let o = flow_run(&[
        p(&again.join("plan.json")),
        "--out",
        p(&again.join("out")),
        "--workers",
        "2",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(folder(&out), folder(&again.join("out")));
    let text = std::fs::read(out.join("plan.json")).unwrap();
    let desk = dir.join("desk");
    std::fs::create_dir_all(&desk).unwrap();
    let (_, prov) = crate::flow::run_plan(&text, &dir.join("data"), &desk, 1, None).unwrap();
    assert_eq!(
        untimed(&prov),
        untimed(&std::fs::read_to_string(out.join("prov.json")).unwrap())
    );
    assert_eq!(
        std::fs::read(desk.join("gap.mzfeat")).unwrap(),
        std::fs::read(out.join("gap.mzfeat")).unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn set_reaches_the_parameters_and_the_plan_as_run_keeps_them() {
    let (dir, files) = fixture("set");
    let plan = dir.join("plan.json");
    let data = dir.join("data");

    // --set by step id (one instance) and by instance id, typed by the descriptor; the plan as run keeps them
    let out = dir.join("out");
    let o = flow_run(&[
        p(&plan),
        "--in",
        p(&data),
        "--out",
        p(&out),
        "--set",
        "features.detect.min_height=50000",
        "--set",
        "gap.preset=lr",
    ]);
    assert_eq!(
        o.code, 3,
        "the preset lr is a preset of detect, not of gapfill: {}",
        o.stderr
    );
    let o = flow_run(&[
        p(&plan),
        "--in",
        p(&data),
        "--out",
        p(&out),
        "--set",
        "features.detect.min_height=50000",
        "--set",
        "ali.drop_artefacts=false",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let as_run = std::fs::read_to_string(out.join("plan.json")).unwrap();
    assert!(
        as_run.contains("\"min_height\":50000") && as_run.contains("\"drop_artefacts\":false"),
        "{as_run}"
    );
    assert_ne!(
        std::fs::read(out.join("gap.mzfeat")).unwrap(),
        page_table(&files, "").0,
        "the setting changes the table"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_rt_correction_is_a_parameter_of_the_alignment() {
    let (dir, _) = fixture("warp");
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let run = |value: &str| {
        let set = format!("ali.rt_warp={value}");
        flow_run(&[p(&plan), "--in", p(&data), "--out", p(&out), "--set", &set])
    };
    let o = run("loess");
    assert_eq!(o.code, 0, "{}", o.stderr);
    let as_run = std::fs::read_to_string(out.join("plan.json")).unwrap();
    assert!(as_run.contains("\"rt_warp\":\"loess\""), "{as_run}");
    assert_eq!(run("cubic").code, 3, "a value the descriptor does not list");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_wavelet_peak_model_is_an_algorithm_of_the_detection() {
    let (dir, _) = fixture("centwave");
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let run = |set: &[&str]| {
        let mut args = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
        for s in set {
            args.extend(["--set", s]);
        }
        flow_run(&args)
    };
    let o = run(&["det.algorithm=centwave", "det.snr=8"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let as_run = std::fs::read_to_string(out.join("plan.json")).unwrap();
    assert!(
        as_run.contains("\"algorithm\":\"centwave\"") && as_run.contains("\"snr\":8"),
        "{as_run}"
    );
    assert_ne!(run(&["det.algorithm=threshold"]).code, 0);
    assert_eq!(run(&["det.snr=8"]).code, 2, "snr belongs to centwave only");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_broad_band_pass_is_one_switch_of_the_wavelet_model() {
    let (dir, _) = fixture("broad");
    let (plan, data, out) = (dir.join("plan.json"), dir.join("data"), dir.join("out"));
    let run = |set: &[&str]| {
        let mut args = vec![p(&plan), "--in", p(&data), "--out", p(&out)];
        for s in set {
            args.extend(["--set", s]);
        }
        flow_run(&args)
    };
    let o = run(&["det.algorithm=centwave", "det.broad_peaks=true"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let as_run = std::fs::read_to_string(out.join("plan.json")).unwrap();
    assert!(as_run.contains("\"broad_peaks\":true"), "{as_run}");
    assert_eq!(
        run(&["det.broad_peaks=true"]).code,
        2,
        "the pass belongs to centwave only"
    );
    assert_eq!(
        run(&["det.algorithm=centwave", "det.broad_peaks=3"]).code,
        2,
        "a boolean"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dry_run_writes_nothing_and_the_exit_codes_say_what_is_wrong() {
    let (dir, _) = fixture("dry");
    let plan = dir.join("plan.json");
    let data = dir.join("data");
    // --dry-run: the steps in order with their parameters, no folder
    let dry = dir.join("dry");
    let o = flow_run(&[p(&plan), "--in", p(&data), "--out", p(&dry), "--dry-run"]);
    let text = String::from_utf8(o.stdout).unwrap();
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        text.contains("[2/4] features.detect@1 (det)") && text.contains("s_t030.mzML"),
        "{text}"
    );
    assert!(!dry.exists());

    // the exit codes: usage 2, plan 3, inputs 4
    let code = |args: &[&str]| flow_run(args).code;
    assert_eq!(code(&[p(&plan), "--in", p(&data)]), 2, "no --out");
    assert_eq!(
        code(&[
            p(&plan),
            "--in",
            p(&data),
            "--dry-run",
            "--set",
            "det.nope=1"
        ]),
        2
    );
    assert_eq!(
        code(&[
            p(&plan),
            "--in",
            p(&data),
            "--dry-run",
            "--set",
            "det.min_height=high"
        ]),
        2
    );
    assert_eq!(
        code(&[
            p(&plan),
            "--in",
            p(&data),
            "--dry-run",
            "--set",
            "det.min_height=-5"
        ]),
        3
    );
    assert_eq!(
        code(&[p(&plan), "--in", p(&dir), "--dry-run"]),
        4,
        "not in the folder"
    );
    assert_eq!(
        code(&[p(&plan), "--in", p(&data), p(&data), "--dry-run"]),
        4,
        "found twice"
    );
    assert!(String::from_utf8(flow_run(&["--help"]).stdout)
        .unwrap()
        .contains("--set INSTANCE.NAME=V"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The browser runner builds its recipe from the layers of the plan with the same function as `flow run`: with parameters set, the table is byte for byte the same.
#[test]
fn the_recipe_of_the_layers_gives_the_bytes_of_flow_run() {
    let (dir, files) = fixture("layers");
    let out = dir.join("out");
    let o = flow_run(&[
        p(&dir.join("plan.json")),
        "--in",
        p(&dir.join("data")),
        "--out",
        p(&out),
        "--set",
        "features.detect.min_height=50000",
        "--set",
        "ali.drop_artefacts=false",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let layers = r#"[{"step":"features.detect","params":{"min_height":50000}},{"step":"features.align","params":{"drop_artefacts":false}}]"#;
    let flow = std::fs::read(out.join("ali.mzfeat")).unwrap();
    for as_text in [false, true] {
        assert!(
            flow == layered_table(&files, layers, as_text),
            "different bytes (as text: {as_text})"
        );
    }
    assert_ne!(
        flow,
        layered_table(&files, "[]", true),
        "the layers change the table"
    );
    assert!(mzlab_feat_wasm::run::layers_from_json(r#"{"step":1}"#).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_parallel_alignment_reports_its_progress_and_the_bytes_do_not_change() {
    use mzlab_core::budget::MemBudget;
    use mzlab_feat_wasm::run;

    use crate::flow_base::{open, Files};

    let (dir, _) = fixture("progress");
    let (data, out) = (dir.join("data"), dir.join("out"));
    let o = flow_run(&[
        p(&dir.join("plan.json")),
        "--in",
        p(&data),
        "--out",
        p(&out),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let names = ["s_t000", "s_t010", "s_t030"];
    let peaks: Vec<PathBuf> = names
        .iter()
        .map(|k| out.join(format!("det-{k}.mzfeatf")))
        .collect();
    let mut recipe =
        run::recipe_for_estimated("", &mut open(&out.join("idx-s_t000.mzidx")).unwrap()).unwrap();
    recipe.fill_gaps = true;
    recipe.weak_files = 1;
    let idx = |k: usize| open(&out.join(format!("idx-{}.mzidx", names[k])));
    let (mut plain, mut with) = (Vec::new(), Vec::new());
    let budget = MemBudget::mb(1024.0);
    run::align_to_par(
        &mut Files(peaks.clone()),
        &budget,
        &recipe,
        &idx,
        &mut plain,
    )
    .unwrap();
    let mut seen: Vec<(usize, usize)> = Vec::new();
    run::align_to_par_progress(
        &mut Files(peaks),
        &budget,
        &recipe,
        &idx,
        &mut with,
        &mut |d, n| seen.push((d, n)),
    )
    .unwrap();
    assert!(plain == with, "the callback changes nothing");
    // the alignment (one unit in memory), the weak tier and the gap filling of the three files
    assert_eq!(seen.first(), Some(&(0, 7)), "{seen:?}");
    assert_eq!(seen.last(), Some(&(7, 7)), "{seen:?}");
    assert!(seen.windows(2).all(|w| w[0].0 <= w[1].0), "{seen:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_alignment_reads_the_weak_candidates_the_detection_stored() {
    use mzlab_core::budget::MemBudget;
    use mzlab_feat::slabs::Stored;
    use mzlab_feat::weakfile;
    use mzlab_feat_wasm::run;

    use crate::flow_base::{open, Files};

    let (dir, _) = fixture("weak");
    let (data, out) = (dir.join("data"), dir.join("out"));
    let o = flow_run(&[
        p(&dir.join("plan.json")),
        "--in",
        p(&data),
        "--out",
        p(&out),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let names = ["s_t000", "s_t010", "s_t030"];
    let peaks: Vec<PathBuf> = names
        .iter()
        .map(|k| out.join(format!("det-{k}.mzfeatf")))
        .collect();
    let stored = |k: usize| std::fs::read(format!("{}.weak", peaks[k].display())).unwrap();
    let mut recipe =
        run::recipe_for_estimated("", &mut open(&out.join("idx-s_t000.mzidx")).unwrap()).unwrap();
    recipe.fill_gaps = false;
    recipe.weak_files = 1;
    let key = recipe.weak_hash();
    let candidates = weakfile::from_bytes(&stored(1), &key).unwrap();
    let mut candidates =
        candidates.expect("the recipe of the alignment is the one of the detection");
    assert!(
        weakfile::from_bytes(&stored(1), "0123456789abcdef0123456789abcdef")
            .unwrap()
            .is_none()
    );
    let align = |src: &mut Files| {
        let mut bytes = Vec::new();
        run::align_to(
            src,
            &MemBudget::mb(1024.0),
            &recipe,
            |k| open(&out.join(format!("idx-{}.mzidx", names[k]))),
            &mut bytes,
        )
        .unwrap();
        bytes
    };
    let plain = align(&mut Files(peaks.clone()));

    // a candidate that the strong peaks do not hold reaches the table; the same bytes under another key are not read
    candidates.mz.push(777.7);
    candidates.rt.push(9.0);
    candidates.left.push(8.9);
    candidates.right.push(9.1);
    candidates.area.push(1e6);
    candidates.height.push(1e5);
    candidates.flags.push(mzlab_feat::table::FLAG_GOOD);
    let planted = weakfile::to_bytes(&candidates, &key);
    let foreign = weakfile::to_bytes(&candidates, "0123456789abcdef0123456789abcdef");
    std::fs::write(format!("{}.weak", peaks[1].display()), &planted).unwrap();
    let with = align(&mut Files(peaks.clone()));
    assert_ne!(with, plain, "the stored candidates are read");
    std::fs::write(format!("{}.weak", peaks[1].display()), foreign).unwrap();
    assert_eq!(
        align(&mut Files(peaks.clone())),
        plain,
        "another key is detected again"
    );
    std::fs::remove_file(format!("{}.weak", peaks[1].display())).unwrap();
    assert_eq!(
        align(&mut Files(peaks.clone())),
        plain,
        "no stored candidates is detected again"
    );
    assert!(Files(peaks).weak(1).unwrap().is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn prov_names_the_executable_that_ran() {
    let (dir, _) = fixture("module");
    let out = dir.join("out");
    let o = flow_run(&[
        p(&dir.join("plan.json")),
        "--in",
        p(&dir.join("data")),
        "--out",
        p(&out),
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let prov =
        mzlab_core::json::parse(std::fs::read(out.join("prov.json")).unwrap().as_slice()).unwrap();
    let exe = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    let agent = prov.get("agent").unwrap();
    assert_eq!(
        agent
            .get("modules")
            .and_then(|m| m.get("mzlab-cli"))
            .and_then(|s| s.str()),
        Some(mzlab_core::budget::sha256_hex(&exe).as_str())
    );
    assert!(agent.get("target").is_some());
}
