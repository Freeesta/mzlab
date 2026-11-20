//! The chemistry group (`mol *`, `ion *`) meets the agent contract: every JSON answer opens with `"schema"` (never `"format"`) and the tag of its
//! entry, a failure is one `error/1`, and the exit code tells a syntax error (2) from a request chemistry cannot satisfy (1) and an unreadable file (4).

use crate::{execute, Outcome};

/// The same two atoms with a bond typed aromatic (4): a cut of it is a valid request the chemistry refuses.
const AROMATIC: &str = "\n  t\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  4  0\nM  END\n";

/// A molfile with an element that is not in the table: it reads, and no chemistry can be done with it.
const NO_ELEMENT: &str = "\n  t\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 Xx  0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0\nM  END\n";

const MOL: &str = "\n  t\n\n  2  1  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0\nM  END\n";

/// A V2000 molfile of `n` carbons and no bond: over the atom limit of the check (400) and of the graph (200).
fn many_atoms(n: usize) -> String {
    let atom = "    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n";
    format!(
        "\n  t\n\n{n:>3}  0  0  0  0  0  0  0  0  0999 V2000\n{}M  END\n",
        atom.repeat(n)
    )
}

/// A V3000 molfile of 2001 carbons: over the atom limit of the export (2000).
fn export_too_big() -> String {
    let atoms: String = (1..=2001)
        .map(|i| format!("M  V30 {i} C 0 0 0 0\n"))
        .collect();
    format!("\n  t\n\n  0  0  0  0  0  0  0  0  0  0999 V3000\nM  V30 BEGIN CTAB\nM  V30 COUNTS 2001 0 0 0 0\nM  V30 BEGIN ATOM\n{atoms}M  V30 END ATOM\nM  V30 END CTAB\nM  END\n")
}

fn run(a: &[&str]) -> Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn text(o: &Outcome) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

fn tmp(name: &str, body: &str) -> String {
    let p = std::env::temp_dir().join(format!("chem_contract_{}_{name}", std::process::id()));
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn every_answer_opens_with_its_schema_and_none_has_a_format_key() {
    let label = tmp(
        "label.json",
        r#"{"structures":[[{"el":"C","h":3},{"el":"O","h":1}]],"arrows":[]}"#,
    );
    let anal = tmp("anal.json", r#"{"atoms":[{"el":"C","h":3},{"el":"Br"}]}"#);
    let mf = tmp("mf.json", &format!("{{\"molfile\":{:?}}}", MOL));
    let mol = tmp("x.mol", MOL);
    let link = tmp("link.json", r#"{"smiles":"CCO","part":false}"#);
    let db = tmp("db.smi", "CCO eth\n");
    let tables = tmp(
        "t.json",
        r#"{"losses":[{"id":"H2O","f":"H2O"}],"adducts":[],"contam":[]}"#,
    );
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (
            vec!["ion", "mz", "C9H8O4", "--adduct", "[M+H]+"],
            "ion_mz/1",
        ),
        (vec!["ion", "mz", "250.8-251.5"], "ion_mz/1"),
        (vec!["ion", "mz", "288.2+16"], "ion_mz/1"),
        (vec!["ion", "pattern", "C9H8O4", "[M+H]+"], "ion_pattern/1"),
        (
            vec![
                "ion", "match", "18.01", "--tables", &tables, "--set", "hr=1",
            ],
            "mz-match/1",
        ),
        (vec!["ion", "elements", "--json"], "elements/1"),
        (vec!["mol", "label", &label], "label/1"),
        (vec!["mol", "anal", &anal], "analysis/1"),
        (vec!["mol", "check", &mf], "check/1"),
        (vec!["mol", "cip", &mf], "cip/1"),
        (vec!["mol", "hilite", &mf], "highlight/1"),
        (vec!["mol", "sel", &mf], "selection/1"),
        (vec!["mol", "link", &link], "link/1"),
        (vec!["mol", "cut", &mol, "--set", "bonds=0-1"], "cut/1"),
        (vec!["mol", "mech", &mf], "mechanism/1"),
        (vec!["mol", "draw", "smiles:CCO", "--json"], "draw/1"),
        (vec!["mol", "open", &mol], "open/1"),
        (
            vec!["mol", "search", "--sub", "CO", "CCO"],
            "substructure/1",
        ),
        (
            vec!["mol", "search", "--sub", "CO", "--db", &db],
            "substructure_db/1",
        ),
    ];
    for (a, schema) in cases {
        let o = run(&a);
        assert_eq!(o.code, 0, "{a:?}: {}", o.stderr);
        let t = text(&o);
        assert!(
            t.starts_with(&format!("{{\"schema\":\"{schema}\"")),
            "{a:?}: {t}"
        );
        let v: serde_json::Value = serde_json::from_str(&t).unwrap();
        assert!(v.get("format").is_none(), "{a:?}: a format key is left");
        let e = crate::cmd::find(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .unwrap()
            .0;
        assert!(
            e.schema.is_empty() || e.schema == schema,
            "{a:?}: entry {} says {}",
            e.name,
            e.schema
        );
    }
}

#[test]
fn every_chemistry_entry_with_one_schema_names_it() {
    for e in
        crate::cmd::entries().filter(|e| e.name.starts_with("mol ") || e.name.starts_with("ion "))
    {
        assert!(!e.exits.is_empty(), "{}: no exit list", e.name);
        // mol export is text and mol search has two documents: the others carry one tag
        if !matches!(e.name, "mol export" | "mol search") {
            assert!(!e.schema.is_empty(), "{}: no schema", e.name);
        }
    }
}

#[test]
fn exit_codes_split_syntax_impossible_and_unreadable() {
    // syntax of a formula, adduct or SMILES in an argument: 2
    for a in [
        vec!["ion", "mz", "C9Xx"],
        vec!["ion", "mz", "C(H"],
        vec!["ion", "pattern", "C9H8O4", "[M+Z]+"],
        vec!["mol", "search", "--sub", "C(", "CCO"],
    ] {
        let o = run(&a);
        assert_eq!(o.code, 2, "{a:?}: {}", o.stderr);
    }
    // a valid request chemistry cannot satisfy: 1, one error/1 under --json
    let o = run(&["ion", "pattern", "H", "[M-H-H2O]-", "--json"]);
    assert_eq!(o.code, 1);
    assert!(text(&o).starts_with("{\"schema\":\"error/1\",\"code\":1"));
    // a bond cut that is not in the molfile, and the cut of an aromatic bond: 1
    let ether = tmp("ether.mol", MOL);
    let benzene = tmp("aromatic.mol", AROMATIC);
    let heterolytic = [
        "--set",
        "bonds=0-1",
        "--set",
        "mode=heterolytic",
        "--set",
        "cation=1",
    ];
    assert_eq!(
        run(&[&["mol", "cut", &ether][..], &heterolytic].concat()).code,
        0
    );
    assert_eq!(run(&["mol", "cut", &ether, "--set", "bonds=0-5"]).code, 1);
    assert_eq!(run(&["mol", "cut", &benzene, "--set", "bonds=0-1"]).code, 1);
    // a file that is missing or is no request: 4
    assert_eq!(run(&["mol", "label", "/no/such/r.json"]).code, 4);
    let junk = tmp("junk.json", "not json");
    for c in [
        "label", "anal", "check", "cip", "hilite", "sel", "link", "mech",
    ] {
        let o = run(&["mol", c, &junk]);
        assert_eq!(o.code, 4, "mol {c}: {}", o.stderr);
        let o = run(&["mol", c, &junk, "--json"]);
        assert!(
            text(&o).starts_with("{\"schema\":\"error/1\",\"code\":4"),
            "mol {c}"
        );
    }
    // a bad value of an option is usage, not an unreadable file: format and ion take one path, the engine's
    assert_eq!(
        run(&["mol", "export", &ether, "--set", "format=pdf"]).code,
        2
    );
    assert_eq!(run(&["mol", "export", &ether, "--set", "ion=nope"]).code, 2);
}

/// One rule for an unreadable molfile, whichever command reads it: 4, and one `error/1` under --json; an element the table lacks is 1.
#[test]
fn a_junk_molfile_is_unreadable_for_every_command_that_reads_one() {
    let wrapped = tmp("junk_mf.json", "{\"molfile\":\"this is not a molfile\"}");
    let bare = tmp("junk.mol", "this is not a molfile");
    let no_element = tmp("noel.json", &format!("{{\"molfile\":{NO_ELEMENT:?}}}"));
    for c in ["check", "cip", "hilite", "sel", "mech"] {
        assert_eq!(run(&["mol", c, &wrapped]).code, 4, "mol {c}");
    }
    // sel and mech read the molfile into the graph, which has no mass for an element the table lacks
    for c in ["sel", "mech"] {
        assert_eq!(run(&["mol", c, &no_element]).code, 1, "mol {c}");
    }
    for a in [
        vec!["mol", "cut", &bare, "--set", "bonds=0-1"],
        vec!["mol", "export", &bare],
    ] {
        let o = run(&a);
        assert_eq!(o.code, 4, "{a:?}: {}", o.stderr);
        // mol export is text: it has no JSON form
        if a[1] == "export" {
            continue;
        }
        let mut j = a.clone();
        j.push("--json");
        let o = run(&j);
        assert!(
            text(&o).starts_with("{\"schema\":\"error/1\",\"code\":4"),
            "{j:?}: {}",
            text(&o)
        );
    }
    // a structure list that cannot be read is input too (a missing file, or no rows), a list over the size limit is data
    let empty = tmp("empty.smi", "");
    for db in [empty.as_str(), "/no/such/db.smi"] {
        let a = ["mol", "search", "--sub", "CO", "--db", db, "--json"];
        let o = run(&a);
        assert!(
            text(&o).starts_with("{\"schema\":\"error/1\",\"code\":4"),
            "{a:?}: {}",
            text(&o)
        );
        assert_eq!(o.code, 4, "{a:?}");
    }
}

const NOWHERE: &str = "/no/such/dir/out.json";

/// Paths of the request files the probes read.
struct Fx {
    mf: String,
    no_element: String,
    junk: String,
    ether: String,
    label: String,
    no_atoms: String,
    anal: String,
    link: String,
    tables: String,
    db: String,
    empty: String,
    list_big: String,
    export_big: String,
    atoms_big: String,
    bad_query: String,
}

fn fixtures() -> Fx {
    Fx {
        mf: tmp("p_mf.json", &format!("{{\"molfile\":{MOL:?}}}")),
        no_element: tmp("p_noel.json", &format!("{{\"molfile\":{NO_ELEMENT:?}}}")),
        junk: tmp("p_junk.json", "not json"),
        ether: tmp("p_ether.mol", MOL),
        label: tmp(
            "p_label.json",
            r#"{"structures":[[{"el":"C","h":3},{"el":"O","h":1}]],"arrows":[]}"#,
        ),
        no_atoms: tmp("p_noatoms.json", r#"{"atoms":[]}"#),
        anal: tmp("p_anal.json", r#"{"atoms":[{"el":"C","h":3},{"el":"Br"}]}"#),
        link: tmp("p_link.json", r#"{"smiles":"CCO","part":false}"#),
        tables: tmp(
            "p_t.json",
            r#"{"losses":[{"id":"H2O","f":"H2O"}],"adducts":[],"contam":[]}"#,
        ),
        db: tmp("p_db.smi", "CCO eth\n"),
        empty: tmp("p_empty.smi", ""),
        list_big: tmp("p_big.smi", &"C\n".repeat(200_001)),
        export_big: tmp("p_big.mol", &export_too_big()),
        atoms_big: tmp(
            "p_atoms.json",
            &format!("{{\"molfile\":{:?}}}", many_atoms(500)),
        ),
        bad_query: tmp(
            "p_query.json",
            &format!(
                "{{\"molfile\":{MOL:?},\"world\":\"hr\",\"layers\":[{{\"kind\":\"moiety\",\"label\":\"x\",\"query\":\"C(\"}}]}}"
            ),
        ),
    }
}

type Probes<'a> = Vec<(&'a str, Vec<(Vec<&'a str>, u8)>)>;

fn ion_probes(fx: &Fx) -> Probes<'_> {
    let (tables, nowhere) = (&fx.tables[..], NOWHERE);
    vec![
        (
            "ion mz",
            vec![
                (vec!["C9H8O4"], 0),
                (vec!["H-5"], 1),
                (vec!["C9Xx"], 2),
                (vec!["C9H8O4", "--out", nowhere], 6),
            ],
        ),
        (
            "ion pattern",
            vec![
                (vec!["C9H8O4", "[M+H]+"], 0),
                (vec!["H", "[M-H-H2O]-"], 1),
                (vec!["C9H8O4", "[M+Z]+"], 2),
                (vec!["C9H8O4", "[M+H]+", "--out", nowhere], 6),
            ],
        ),
        (
            "ion match",
            vec![
                (vec!["18.01", "--tables", &tables], 0),
                (vec![], 2),
                (vec!["18.01", "--tables", "/no/such/t.json"], 4),
                (vec!["18.01", "--tables", &tables, "--out", nowhere], 6),
            ],
        ),
        (
            "ion elements",
            vec![
                (vec!["--json"], 0),
                (vec!["--nope"], 2),
                (vec!["--json", "--out", nowhere], 6),
            ],
        ),
    ]
}

fn mol_probes(fx: &Fx) -> Probes<'_> {
    let nowhere = NOWHERE;
    let (label, junk, anal, no_atoms, link) = (
        &fx.label[..],
        &fx.junk[..],
        &fx.anal[..],
        &fx.no_atoms[..],
        &fx.link[..],
    );
    let mut probes: Probes<'_> = vec![
        (
            "mol label",
            vec![
                (vec![&label], 0),
                (vec![], 2),
                (vec![&junk], 4),
                (vec![&label, "--out", nowhere], 6),
            ],
        ),
        (
            "mol anal",
            vec![
                (vec![&anal], 0),
                (vec![&no_atoms], 1),
                (vec![], 2),
                (vec![&junk], 4),
                (vec![&anal, "--out", nowhere], 6),
            ],
        ),
        (
            "mol link",
            vec![
                (vec![&link], 0),
                (vec![], 2),
                (vec![&junk], 4),
                (vec![&link, "--out", nowhere], 6),
            ],
        ),
    ];
    // the one request each of the five refuses although it parses: too many atoms (check, sel, mech), a bad query (hilite); cip has none
    for (c, refused) in [
        ("check", Some(&fx.atoms_big)),
        ("cip", None),
        ("hilite", Some(&fx.bad_query)),
        ("sel", Some(&fx.no_element)),
        ("mech", Some(&fx.no_element)),
    ] {
        let name: &'static str = Box::leak(format!("mol {c}").into_boxed_str());
        let mut list = vec![
            (vec![&fx.mf[..]], 0),
            (vec![], 2),
            (vec![junk], 4),
            (vec![&fx.mf[..], "--out", nowhere], 6),
        ];
        if let Some(r) = refused {
            list.push((vec![&r[..]], 1));
        }
        probes.push((name, list));
    }
    probes
}

fn draw_probes() -> Probes<'static> {
    let db: &'static str = Box::leak(
        format!(
            "db={}/../../mzlab/web/molecole/catalogo.csv",
            env!("CARGO_MANIFEST_DIR")
        )
        .into_boxed_str(),
    );
    // over the 2000-atom limit: data (1), as every `*.tooBig`
    let big: &'static str = Box::leak(format!("smiles:{}", "C".repeat(2001)).into_boxed_str());
    let edit: &'static str = r#"edit=[{"op":"atom.set","id":9,"chg":1}]"#;
    vec![(
        "mol draw",
        vec![
            (vec!["smiles:CCO"], 0),
            // an operation that fails is a usage error (2) naming its index, never a partial drawing
            (vec!["smiles:CCO", "--set", edit], 2),
            (vec!["smiles:CCO", "--set", "edit=@/no/such/patch.json"], 4),
            (vec![big], 1),
            (vec!["smiles:C(("], 2),
            (vec!["name:caffeine"], 2),
            // an InChI alone is never parsed: usage, the hint asks for the InChIKey
            (vec!["InChI=1S/CH4/h1H4"], 2),
            (vec!["/no/such/x.mol"], 4),
            // content that is not a molfile is unreadable input
            (vec!["molfile:not a molfile\nM  END\n"], 4),
            (vec!["name:zzzzzzzzzz", "--set", db], 5),
            (vec!["smiles:CCO", "--out", NOWHERE], 6),
        ],
    )]
}

fn export_probes(fx: &Fx) -> Probes<'_> {
    let nowhere = NOWHERE;
    let (ether, export_big) = (&fx.ether[..], &fx.export_big[..]);
    let (list_big, empty, db) = (&fx.list_big[..], &fx.empty[..], &fx.db[..]);
    vec![
        (
            "mol export",
            vec![
                (vec![&ether], 0),
                (vec![&export_big], 1),
                (vec![&ether, "--set", "format=pdf"], 2),
                (vec!["/no/such/x.mol"], 4),
                (vec![&ether, "--out", nowhere], 6),
            ],
        ),
        (
            "mol cut",
            vec![
                (vec![&ether, "--set", "bonds=0-1"], 0),
                (vec![&ether, "--set", "bonds=0-5"], 1),
                (vec![&ether], 2),
                (vec!["/no/such/x.mol", "--set", "bonds=0-1"], 4),
                (vec![&ether, "--set", "bonds=0-1", "--out", nowhere], 6),
            ],
        ),
        (
            "mol search",
            vec![
                (vec!["--sub", "CO", "CCO"], 0),
                (vec!["--sub", "C(", "CCO"], 2),
                (vec!["--sub", "CO", "--db", &list_big], 1),
                (vec!["--sub", "C(", "CCO"], 2),
                (vec!["--sub", "CO", "--db", &empty], 4),
                (vec!["--sub", "CO", "--db", &db, "--out", nowhere], 6),
            ],
        ),
    ]
}

/// The exit list of an entry is what the command can give: one probe per listed code, the code each probe gives is the one expected, and the
/// probes cover the list exactly (a code the list lacks never occurs, a listed one is reachable).
#[test]
fn each_entry_lists_exactly_the_codes_its_probes_give() {
    let fx = fixtures();
    let mut bad = Vec::new();
    let mut probes = ion_probes(&fx);
    probes.extend(mol_probes(&fx));
    probes.extend(export_probes(&fx));
    probes.extend(draw_probes());
    for (name, list) in probes {
        let e = crate::cmd::entries().find(|e| e.name == name).unwrap();
        let mut seen = std::collections::BTreeSet::new();
        for (rest, want) in list {
            let mut a: Vec<&str> = name.split(' ').collect();
            a.extend(rest);
            let o = run(&a);
            if o.code != want {
                bad.push(format!(
                    "{a:?}: wanted {want}, got {} ({})",
                    o.code,
                    o.stderr.trim()
                ));
            }
            seen.insert(want);
        }
        let listed: std::collections::BTreeSet<u8> = e.exits.iter().copied().collect();
        if seen != listed {
            bad.push(format!(
                "{name}: probes cover {seen:?}, the entry lists {listed:?}"
            ));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
