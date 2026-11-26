//! `commands --find`: the tasks of the agent audit (a low-resolution LC-MS campaign) find the command or step that does them among the
//! first three, every command and step has a summary the search matches, and every row of `related.rs` names something that exists.

use super::{items, search, words};
use crate::catalog::related::RELATED;

/// (task as an agent would say it, the command or step that does it): the audit of the PR that added the search.
pub(crate) const TASKS: &[(&str, &str)] = &[
    ("inventory of a folder of raw files", "file list"),
    (
        "collision energy spread and declustering potential of a tandem file",
        "file method",
    ),
    (
        "sample name mass differs from the method precursor",
        "file method",
    ),
    (
        "convert wiff files to mzML with their sample names",
        "file convert",
    ),
    (
        "assign roles blank qc standard to the samples",
        "sample table",
    ),
    (
        "dark control not recognised in the transformation product screening",
        "sample table",
    ),
    ("which file is which: the dark control, blank, standard", "sample table"),
    (
        "tp.dig_lr fails when the plan contains a standard: set the roles from a csv",
        "sample import",
    ),
    (
        "preview which files are left out and which get a role from their name, check the roles of a table before the run",
        "sample pair",
    ),
    (
        "what are the default parameters of a flow step",
        "flow defaults",
    ),
    ("which noise floor and tolerances fit my files", "estimate"),
    (
        "import sample names and roles from a csv metadata table",
        "sample import",
    ),
    ("non-targeted feature table of all files", "features"),
    (
        "the same peak appears as two duplicate rows, a shoulder merged",
        "features.gapfill",
    ),
    (
        "one target list from the feature tables of several series",
        "quant union",
    ),
    (
        "integrate a target list of SIM ions in many files",
        "chrom.transitions",
    ),
    (
        "S/N of every SIM channel of every file",
        "chrom.transitions",
    ),
    (
        "integrate all peaks of all traces of many files at once",
        "peak table",
    ),
    ("extract ion chromatograms of a list of m/z", "trace xic"),
    (
        "sum the scans of an MS2 window and count the ions it rests on",
        "spectrum get",
    ),
    (
        "blank subtraction threshold on the feature table",
        "table.filter",
    ),
    (
        "group correlated ions and isotopes of one compound",
        "features.group",
    ),
    (
        "find adducts and in-source fragments that co-elute at unit mass",
        "features.companions",
    ),
    (
        "join the feature tables of several series into one target list",
        "features.union",
    ),
    (
        "kinetics dig of the transformation products at unit resolution",
        "tp.dig_lr",
    ),
    (
        "generate hypotheses of the transformation products",
        "tp.generate",
    ),
    (
        "draw a structure from SMILES as SVG PNG or CDXML",
        "mol draw",
    ),
    ("draw the pathway network of the products", "tp.network"),
    ("centroid profile spectra", "spectra.centroid"),
    ("export the feature table as CSV", "export.table_csv"),
    ("record the provenance of a run", "flow record"),
    ("calibration curve with LOD and LOQ", "quant.curve"),
    (
        "what is in the spectrum of a time window that is not in the background",
        "raw scan",
    ),
    (
        "mean spectrum of a window minus its sides per m/z bin",
        "raw scan",
    ),
    (
        "weak noise peaks forced into blanks and matrix: S/N and peak shape gates of a target list",
        "quant targets",
    ),
    (
        "retention times drift between batches, align on an internal standard",
        "quant targets",
    ),
    (
        "mass offset differs by polarity, measure it on the parent",
        "quant targets",
    ),
    ("delay of the uv detector against the ms", "dad delay"),
    ("sim snr", "peak table"),
    ("integrate peaks", "peak table"),
    (
        "convert cps to ions with the dwell or fill time of the scan type",
        "raw xic",
    ),
    (
        "remove ions that are also in the blank sample",
        "table.filter",
    ),
    (
        "background ions and carryover in the matrix blank",
        "table.filter",
    ),
    ("contaminant ions of the matrix blank", "table compare"),
    ("subtract the blank spectrum from a sample", "spectrum get"),
    // the words of a user who does not know the measured-parameters, union and verdict commands exist
    ("which parameters", "estimate"),
    ("noise floor", "estimate"),
    ("what threshold", "estimate"),
    ("ppm", "estimate"),
    ("what ppm tolerance fits my files", "estimate"),
    ("targets from many series", "quant union"),
    ("targets from many series", "features.union"),
    ("is there a peak at this mass", "raw xic"),
    ("how many ions", "raw xic"),
];

/// Short queries where a one-at-a-time command scores the same as the one call for many: the one call comes first.
const BULK_FIRST: &[&str] = &["sim snr", "integrate peaks"];

fn top(q: &str, n: usize) -> Vec<String> {
    search(&words(q))
        .into_iter()
        .take(n)
        .map(|(it, _, _)| it.name)
        .collect()
}

#[test]
fn every_audit_task_finds_its_command_or_step_among_the_first_three() {
    for (task, want) in TASKS {
        let got = top(task, 3);
        assert!(
            got.iter().any(|g| g == want),
            "{task:?}: {want} not in {got:?}"
        );
    }
}

#[test]
fn the_bare_word_blank_lists_the_blank_filter_and_the_blank_ratio() {
    let got = top("blank", 5);
    for want in ["table.filter", "table compare"] {
        assert!(got.iter().any(|g| g == want), "{want} not in {got:?}");
    }
}

#[test]
fn on_equal_scores_the_one_call_for_many_comes_first() {
    for q in BULK_FIRST {
        let got = top(q, 5);
        assert_eq!(
            got.first().map(String::as_str),
            Some("peak table"),
            "{q:?}: {got:?}"
        );
        let lr = got.iter().position(|g| g == "peak auto-lr");
        assert!(lr.is_some_and(|i| i > 0), "{q:?}: {got:?}");
    }
}

#[test]
fn a_repeated_word_counts_once() {
    let once = search(&words("sim"));
    let thrice = search(&words("sim sim sim"));
    let scores = |h: &[(super::Item, f64, Vec<String>)]| -> Vec<(String, f64)> {
        h.iter().map(|(i, s, _)| (i.name.clone(), *s)).collect()
    };
    assert_eq!(scores(&once), scores(&thrice));
    let thrice_apart = search(&["sim".to_string(), "snr".to_string(), "sim".to_string()]);
    assert_eq!(scores(&thrice_apart), scores(&search(&words("sim snr"))));
    let o =
        crate::execute(&["commands", "--find", "sim", "snr", "sim", "--json"].map(String::from));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["query"], serde_json::json!(["sim", "snr"]));
}

/// The guard of new functions: a command or step without a summary (title) the search matches is not discoverable.
#[test]
fn every_command_and_step_has_a_summary_that_finds_it() {
    let all = items();
    assert!(all.iter().any(|i| i.kind == "step") && all.iter().any(|i| i.kind == "command"));
    for it in &all {
        assert!(
            !it.summary.trim().is_empty(),
            "{} {} has no one-line summary",
            it.kind,
            it.name
        );
        let q = format!("{} {}", it.name, it.summary);
        let got = top(&q, 5);
        assert!(
            got.contains(&it.name),
            "{} {}: its name and summary find {got:?}",
            it.kind,
            it.name
        );
        assert!(
            !it.example.is_empty() && it.example.starts_with("mzlab-cli "),
            "{}: example {:?}",
            it.name,
            it.example
        );
    }
}

#[test]
fn every_related_row_names_a_command_or_step_that_exists() {
    let names: Vec<String> = items().into_iter().map(|i| i.name).collect();
    for r in RELATED {
        assert!(names.iter().any(|n| n == r.name), "related row {}", r.name);
        assert!(!r.words.is_empty(), "{}: no task words", r.name);
        for a in r.also {
            assert!(
                names.iter().any(|n| n == a) && *a != r.name,
                "{}: see also {a}",
                r.name
            );
        }
        assert!(
            r.example.is_empty() || r.example.starts_with("mzlab-cli "),
            "{}",
            r.name
        );
    }
}

#[test]
fn the_answer_is_commands_find_1_with_examples_and_bulk_and_stable_codes() {
    let run = |a: &[&str]| crate::execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let o = run(&[
        "commands",
        "--find",
        "integrate",
        "peaks",
        "of",
        "one",
        "SIM",
        "trace",
        "--json",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stderr.is_empty(), "{}", o.stderr);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["schema"], "commands-find/1");
    assert_eq!(
        v["query"],
        serde_json::json!(["integrate", "peak", "sim", "trace"])
    );
    let m = v["matches"].as_array().unwrap();
    assert!(!m.is_empty() && m.len() <= 5);
    for x in m {
        for k in [
            "kind", "name", "summary", "example", "score", "matched", "see_also",
        ] {
            assert!(x.get(k).is_some(), "{k} in {x}");
        }
    }
    let lr = m
        .iter()
        .find(|x| x["name"] == "peak auto-lr")
        .expect("peak auto-lr");
    assert!(lr["bulk"]
        .as_str()
        .unwrap()
        .contains("peak table FILE... --auto"));
    // the same bytes on every run
    assert_eq!(
        run(&[
            "commands",
            "--find",
            "integrate",
            "peaks",
            "of",
            "one",
            "SIM",
            "trace",
            "--json"
        ])
        .stdout,
        o.stdout
    );
    // the text form: one line per match and its example
    let t = String::from_utf8(run(&["commands", "--find", "draw", "structure"]).stdout).unwrap();
    assert!(t.starts_with("mol draw (command)"), "{t}");
    assert_eq!(
        run(&["commands", "--find", "zzqqxx", "--json"]).code,
        crate::exit::SELECTOR
    );
    let none = run(&["commands", "--find", "zzqqxx", "--json"]);
    let e: serde_json::Value = serde_json::from_slice(&none.stdout).unwrap();
    assert_eq!(e["error_key"], "err.cli.find.none");
    assert_eq!(
        run(&["commands", "--find", "--json"]).code,
        crate::exit::USAGE
    );
    assert_eq!(
        run(&["commands", "flow", "--find", "plan"]).code,
        crate::exit::USAGE
    );
}

#[test]
fn the_full_listing_of_a_command_names_its_see_also_and_bulk() {
    let o = crate::execute(&[
        "commands".into(),
        "--json".into(),
        "peak".into(),
        "auto-lr".into(),
    ]);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let c = &v["commands"][0];
    assert!(
        c["see_also"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x == "peak table"),
        "{c}"
    );
    assert!(c["bulk"].as_str().is_some(), "{c}");
}

/// Only a `bulk` that is another command demotes a row on ties; `trace xic` and `tp.dig_lr` describe their own batch use.
#[test]
fn only_a_bulk_that_names_another_command_demotes() {
    let of = |n: &str| crate::catalog::related::of(n).unwrap();
    assert!(
        of("peak auto-lr").elsewhere()
            && of("peak area").elsewhere()
            && of("peak auto").elsewhere()
    );
    assert!(
        !of("trace xic").elsewhere()
            && !of("tp.dig_lr").elsewhere()
            && !of("peak table").elsewhere()
    );
}

/// `peak table --auto` is the low-resolution engine: the high-resolution `peak auto` points at `features`, a fixed window at `--windows`.
#[test]
fn the_bulk_route_of_a_peak_command_runs_the_same_kind_of_integration() {
    let of = |n: &str| crate::catalog::related::of(n).unwrap().bulk;
    assert!(
        of("peak auto").starts_with("mzlab-cli features "),
        "{}",
        of("peak auto")
    );
    assert!(of("peak area").contains("--windows") && !of("peak area").contains("--auto"));
    assert!(of("peak auto-lr").contains("peak table FILE... --auto"));
    let help = |c: &[&str]| {
        let mut a: Vec<String> = c.iter().map(|s| s.to_string()).collect();
        a.push("--help".into());
        String::from_utf8(crate::execute(&a).stdout).unwrap()
    };
    let hr = help(&["peak", "auto"]);
    assert!(
        hr.contains("`features FILE...`") && !hr.contains("`peak table FILE... --auto`"),
        "{hr}"
    );
    let area = help(&["peak", "area"]);
    assert!(
        area.contains("--windows PATH") && !area.contains("--auto"),
        "{area}"
    );
}

/// A second `--find` adds its words: none is dropped.
#[test]
fn a_repeated_find_keeps_every_word() {
    let o =
        crate::execute(&["commands", "--find", "sim", "--find", "snr", "--json"].map(String::from));
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["query"], serde_json::json!(["sim", "snr"]));
}

/// The state words and the shape gates of `quant targets` are findable: the search answers a task about weak peaks in blanks with the command, its
/// example line names the gate, and its help shows both the state words and the gate parameters (a function nobody finds is not used).
#[test]
fn the_state_words_and_shape_gates_of_quant_targets_are_found_from_the_task_and_the_help() {
    let o = crate::execute(
        &[
            "commands", "--find", "forced", "peaks", "in", "blanks", "shape", "gate", "--json",
        ]
        .map(String::from),
    );
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    // The Flow step of the same function may rank just above its command (the bare word "blank" must keep the blank filter in the top five).
    let top2 = &v["matches"].as_array().unwrap()[..2];
    let first = top2
        .iter()
        .find(|m| m["name"] == "quant targets")
        .unwrap_or_else(|| panic!("quant targets not in the first two: {top2:?}"));
    assert!(
        first["example"].as_str().unwrap().contains("min_half"),
        "{first}"
    );
    let help = crate::execute(&["quant", "targets", "--help"].map(String::from));
    let h = String::from_utf8_lossy(&help.stdout).into_owned();
    for w in ["forced", "min_half", "max_fwhm", "max_asym", "decisions"] {
        assert!(h.contains(w), "{w} not in the help");
    }
}

/// The target-list route is found by the words of the loop that missed it (thousands of `peak auto-lr` calls), and `peak auto-lr` points at it.
#[test]
fn a_list_of_targets_over_many_files_finds_quant_targets_and_the_looped_command_points_at_it() {
    let find = |words: &[&str]| {
        let mut a = vec!["commands".to_string(), "--find".into()];
        a.extend(words.iter().map(|s| s.to_string()));
        a.push("--json".into());
        let o = crate::execute(&a);
        assert_eq!(o.code, 0, "{}", o.stderr);
        serde_json::from_slice::<serde_json::Value>(&o.stdout).unwrap()
    };
    let v = find(&[
        "integrate",
        "many",
        "targets",
        "m/z",
        "RT",
        "windows",
        "every",
        "file",
    ]);
    assert_eq!(v["matches"][0]["name"], "quant targets", "{v}");
    let lr = find(&["peak", "auto-lr"]);
    let row = lr["matches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "peak auto-lr")
        .unwrap();
    assert!(
        row["bulk"]
            .as_str()
            .unwrap()
            .starts_with("mzlab-cli quant targets FILE..."),
        "{row}"
    );
    assert!(
        row["see_also"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x == "quant targets"),
        "{row}"
    );
    let help = String::from_utf8(
        crate::execute(&["peak".to_string(), "auto-lr".into(), "--help".into()]).stdout,
    )
    .unwrap();
    assert!(
        help.contains("quant targets FILE... --targets T.csv"),
        "{help}"
    );
}
