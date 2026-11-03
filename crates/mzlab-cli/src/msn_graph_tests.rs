use std::path::PathBuf;

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
    assert_eq!(v.get("formato").unwrap().str(), Some("msngrafo/2"));
    assert_eq!(
        v.get("sorgente").unwrap().get("nome").unwrap().str(),
        Some("msn.mzML")
    );
    let scelte: Vec<&str> = v
        .get("nodi")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|n| n.get("formula")?.get("scelta")?.str())
        .collect();
    assert_eq!(
        scelte,
        [
            "C14H15N3O5",
            "C12H15N3O",
            "C13H10NO4",
            "C9H8NO2",
            "C10H8NO3"
        ]
    );
    // the 261 of the file is not a sub-formula of the ion: it leaves the graph with its fragment, and its path is noted
    let c = v.get("conteggi").unwrap();
    assert_eq!(c.get("scartati_formula").unwrap().f64(), Some(2.0));
}

#[test]
fn a_truth_file_gives_one_line_of_numbers() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_msn_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let verita = dir.join("verita.json");
    let t = r#"{"nodes":[{"path":[305],"formula":"C14H15N3O5"},{"path":[305,244],"formula":"C13H10NO4"},{"path":[305,261],"formula":"C13H9N2O4"}]}"#;
    std::fs::write(&verita, t).unwrap();
    let f = msn();
    let o = run(&[
        "msn-graph",
        f.to_str().unwrap(),
        "--formula",
        "C14H15N3O5 [M]+",
        "--verita",
        verita.to_str().unwrap(),
        "--riassunto",
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
            n("percorsi_trovati"),
            n("formula_giusta_fra_candidati"),
            n("unica_e_giusta")
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
    assert_eq!(run(&["msn-graph", "x.mzML", "--riassunto"]).code, 2);
    assert_eq!(
        run(&["msn-graph", "x.mzML", "--ppm-formula", "dieci"]).code,
        2
    );
    assert_eq!(run(&["msn-graph", "--help"]).code, 0);
}

/// (old flags, new flags) of every renamed flag of `msn-graph`; `t` is a truth file.
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
            vec!["--export", "csv", "--pulito", "--nodo", "tutto"],
            vec!["--export", "csv", "--clean", "--node", "tutto"],
        ),
        (
            vec!["--verita", t, "--riassunto"],
            vec!["--truth", t, "--summary"],
        ),
    ]
}

/// Every old flag of `msn-graph` gives the same stdout bytes and exit code as its English name; `tutti` = `all`.
#[test]
fn old_flags_are_aliases_of_the_english_ones() {
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
        assert_eq!(
            (o.code, &o.stdout),
            (n.code, &n.stdout),
            "{old:?} vs {new:?}"
        );
    }
    // the refusals name the new flags
    let r = run(&["msn-graph", "x.mzML", "--summary"]);
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("--summary needs --truth"), "{}", r.stderr);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The help shows the English flags, the step parameter next to them, and ends with the line of the old names.
#[test]
fn help_shows_new_flags_and_ends_with_the_old_names() {
    let h = String::from_utf8(run(&["msn-graph", "--help"]).stdout).unwrap();
    assert!(h.contains("--low-mz N") && h.contains("(param low_mz)"));
    let last = h.trim_end().lines().last().unwrap();
    assert!(
        last.starts_with("old names still accepted: --mz-basso"),
        "{last}"
    );
    assert!(!h
        .trim_end()
        .lines()
        .rev()
        .skip(1)
        .any(|l| l.contains("--mz-basso")));
}
