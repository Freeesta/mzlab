use std::collections::BTreeSet;
use std::path::PathBuf;

use mzlab_core::json::{parse, Value};

use crate::execute;

/// The synthetic infusion of the route `/api/msntree` (an MS2 of 305 and two MS3), public.
fn msn() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/ref/msntree_rotta/msn.mzML")
}

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn the_graph_of_a_file_with_the_formula_of_its_ion() {
    let f = msn();
    let o = run(&["msn-graph", f.to_str().unwrap(), "--formula", "C14H15N3O5+"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let v = mzlab_core::json::parse(&o.stdout).unwrap();
    assert_eq!(v.get("schema").unwrap().str(), Some("msn-graph/1"));
    assert_eq!(
        v.get("source").unwrap().get("name").unwrap().str(),
        Some("msn.mzML")
    );
    let choices: Vec<&str> = v
        .get("nodes")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|n| n.get("formula")?.get("choice")?.str())
        .collect();
    assert_eq!(
        choices,
        [
            "C14H15N3O5",
            "C12H15N3O",
            "C13H10NO4",
            "C9H8NO2",
            "C10H8NO3"
        ]
    );
    // the 261 of the file is not a sub-formula of the ion: it leaves the graph with its fragment, and its path is noted
    let c = v.get("counts").unwrap();
    assert_eq!(c.get("discarded_formula").unwrap().f64(), Some(2.0));
}

#[test]
fn a_truth_file_gives_one_line_of_numbers() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_msn_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let truth = dir.join("verita.json");
    let t = r#"{"nodes":[{"path":[305],"formula":"C14H15N3O5"},{"path":[305,244],"formula":"C13H10NO4"},{"path":[305,261],"formula":"C13H9N2O4"}]}"#;
    std::fs::write(&truth, t).unwrap();
    let f = msn();
    let o = run(&[
        "msn-graph",
        f.to_str().unwrap(),
        "--formula",
        "C14H15N3O5 [M]+",
        "--truth",
        truth.to_str().unwrap(),
        "--summary",
    ]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let r = String::from_utf8(o.stdout).unwrap();
    assert_eq!(r.lines().count(), 1);
    let v = mzlab_core::json::parse(r.as_bytes()).unwrap();
    let n = |k: &str| v.get(k).and_then(|x| x.f64());
    assert_eq!(
        (
            n("percorsi_verita"),
            n("paths_found"),
            n("correct_formula_among_candidates"),
            n("unique_and_correct")
        ),
        (Some(3.0), Some(2.0), Some(2.0), Some(2.0))
    );
    assert!(n("ppm_max_giuste").unwrap() <= 10.0);
    // only numbers: no formula and no name of the file in the line
    assert!(!r.contains('C') && !r.contains("msn"));
}

#[test]
fn bad_arguments_are_refused() {
    assert_eq!(run(&["msn-graph"]).code, 2);
    assert_eq!(run(&["msn-graph", "x.mzML", "--summary"]).code, 2);
    assert_eq!(
        run(&["msn-graph", "x.mzML", "--ppm-formula", "dieci"]).code,
        2
    );
    assert_eq!(run(&["msn-graph", "--help"]).code, 0);
}

/// (old flags, new flags) of every flag of `msn-graph` that had an Italian name; `t` is a truth file.
const SMILES: &str = "CCOC(=O)c1c[nH]c2ccccc12";

fn alias_cases(t: &str) -> Vec<(Vec<&str>, Vec<&str>)> {
    vec![
        (vec!["--mz-basso", "200"], vec!["--low-mz", "200"]),
        (vec!["--ppm-basso", "30"], vec!["--wide-ppm", "30"]),
        (vec!["--prove-min", "2"], vec!["--min-evidence", "2"]),
        (vec!["--attivazione", "cid"], vec!["--activation", "cid"]),
        (vec!["--isotopi"], vec!["--isotopes"]),
        (
            vec!["--scelta", "244=C13H10NO4"],
            vec!["--pick", "244=C13H10NO4"],
        ),
        (
            vec!["--interpreta", "--ppm-perdita", "5"],
            vec!["--interpret", "--loss-ppm", "5"],
        ),
        (
            vec!["--regole", "--toll-perdita-mda", "5"],
            vec!["--rules", "--loss-tol-mda", "5"],
        ),
        (
            vec![
                "--smiles",
                SMILES,
                "--rotture",
                "--max-legami",
                "1",
                "--ppm-struttura",
                "5",
            ],
            vec![
                "--smiles",
                SMILES,
                "--breaks",
                "--max-bonds",
                "1",
                "--struct-ppm",
                "5",
            ],
        ),
        (
            vec!["--smiles", SMILES, "--strutture", "n3"],
            vec!["--smiles", SMILES, "--structures", "n3"],
        ),
        (
            vec!["--smiles", SMILES, "--strutture", "tutti"],
            vec!["--smiles", SMILES, "--structures", "all"],
        ),
        (
            vec!["--export", "csv", "--pulito", "--nodo", "all"],
            vec!["--export", "csv", "--clean", "--node", "all"],
        ),
        (
            vec!["--verita", t, "--riassunto"],
            vec!["--truth", t, "--summary"],
        ),
    ]
}

/// The English flags work; the former Italian names (and `tutti`) are refused: no alias is kept.
#[test]
fn only_the_english_flags_are_accepted() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_msn_alias_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let truth = dir.join("t.json");
    std::fs::write(
        &truth,
        r#"{"nodes":[{"path":[305],"formula":"C14H15N3O5"}]}"#,
    )
    .unwrap();
    let t = truth.to_str().unwrap();
    let cases = alias_cases(t);
    let f = msn();
    for (old, new) in cases {
        let go = |flags: &[&str]| {
            let mut a = vec!["msn-graph", f.to_str().unwrap(), "--formula", "C14H15N3O5+"];
            a.extend(flags);
            run(&a)
        };
        let (o, n) = (go(&old), go(&new));
        assert_eq!(n.code, 0, "{new:?}: {}", n.stderr);
        assert_eq!(o.code, 2, "{old:?} must be refused");
    }
    // the refusals name the new flags
    let r = run(&["msn-graph", "x.mzML", "--summary"]);
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("--summary needs --truth"), "{}", r.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The help shows the English flags and the step parameter next to them, and no former name.
#[test]
fn help_shows_the_english_flags_only() {
    let h = String::from_utf8(run(&["msn-graph", "--help"]).stdout).unwrap();
    assert!(h.contains("--low-mz N") && h.contains("(param low_mz)"));
    assert!(!h.contains("old names still accepted") && !h.contains("--mz-basso"));
}

// Every key and string value of the MSn JSON outputs on the synthetic infusion holds no Italian word of `tools/italian_words.tsv`.

fn italian() -> BTreeSet<String> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/italian_words.tsv");
    std::fs::read_to_string(p)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split('\t').next())
        .filter(|w| w.len() >= 4)
        .map(str::to_lowercase)
        .collect()
}

/// The Italian words among the keys and string values of `v`, as `path: word`.
fn hits(v: &Value, path: &str, words: &BTreeSet<String>, out: &mut Vec<String>) {
    let check = |text: &str, out: &mut Vec<String>| {
        for w in text.split(|c: char| !c.is_alphabetic()) {
            if words.contains(&w.to_lowercase()) {
                out.push(format!("{path}: {w}"));
            }
        }
    };
    match v {
        // i18n keys (`warn.msngrafo.isolation`) are identifiers of the lang files, outside the scope of the format rename
        Value::Str(s) if s.starts_with("warn.") || s.starts_with("err.") => {}
        Value::Str(s) => check(s, out),
        Value::Arr(a) => a.iter().for_each(|x| hits(x, path, words, out)),
        Value::Obj(o) => {
            for (k, x) in o {
                check(k, out);
                hits(x, &format!("{path}/{k}"), words, out);
            }
        }
        _ => {}
    }
}

fn assert_english(what: &str, text: &[u8]) {
    let v = parse(text).unwrap_or_else(|_| panic!("{what} is not JSON"));
    let mut found = vec![];
    hits(&v, what, &italian(), &mut found);
    found.dedup();
    assert!(found.is_empty(), "Italian words in {what}: {found:?}");
}

#[test]
fn the_msn_json_outputs_hold_no_italian_word() {
    let d = std::env::temp_dir().join(format!("mzlab_msn_english_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = msn();
    let ion = ["--formula", "C14H15N3O5+"];
    let smi = ["--smiles", "CCOC(=O)c1c[nH]c2ccccc12"];
    let all = [
        &["msn-graph", f.to_str().unwrap()][..],
        &ion[..],
        &smi[..],
        &["--interpret", "--rules", "--breaks", "--structures", "all"][..],
    ]
    .concat();
    let g = run(&all);
    assert_eq!(g.code, 0, "{}", g.stderr);
    assert_english("msn-graph", &g.stdout);
    let gp = d.join("g.json");
    std::fs::write(&gp, &g.stdout).unwrap();
    let plain = run(&[&["msn-graph", f.to_str().unwrap()][..], &ion[..]].concat());
    let pp = d.join("p.json");
    std::fs::write(&pp, &plain.stdout).unwrap();

    let c = run(&["msn-graph", "complete", pp.to_str().unwrap()]);
    assert_eq!(c.code, 0, "{}", c.stderr);
    assert_english("msn-graph complete", &c.stdout);

    let lib = d.join("lib.json");
    let (l, p) = (lib.to_str().unwrap(), pp.to_str().unwrap());
    let add = [
        &["msn-lib", "add", l, f.to_str().unwrap(), "--id", "STD-1"][..],
        &ion[..],
    ]
    .concat();
    assert_eq!(run(&add).code, 0);
    assert_english("msn-lib", std::fs::read(&lib).unwrap().as_slice());
    let s = run(&["msn-lib", "search", l, p]);
    assert_eq!(s.code, 0, "{}", s.stderr);
    assert_english("msn-search", &s.stdout);

    let cmp = run(&["msn-graph", "compare", p, p]);
    assert_eq!(cmp.code, 0, "{}", cmp.stderr);
    assert_english("msn-compare", &cmp.stdout);

    let lines = "{\"id\":\"R\",\"layer\":\"hcd\",\"pol\":1,\"v\":{\"n\":2,\"it\":100,\"a\":2,\"ia\":100,\"b\":0,\"ib\":0,\"c\":1,\"ic\":80,\"r\":{\"loss_c2h2o_acetyl\":1}}}\n";
    let lf = d.join("lines.jsonl");
    std::fs::write(&lf, lines).unwrap();
    let sum = run(&["msn-rules", "sum", lf.to_str().unwrap()]);
    assert_eq!(sum.code, 0, "{}", sum.stderr);
    assert_english("msn-rules sum", &sum.stdout);
    let _ = std::fs::remove_dir_all(&d);
}
