//! A V3000 molfile through the `mol *` commands: the same answer as the V2000 of the same molecule (byte for byte), and a damaged or oversized one
//! is an error with its exit code (4 unreadable, 1 refused), never an answer for an empty molecule.

use crate::{execute, Outcome};

const V2000: &str = "\n  t\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.2500    0.7000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    2.5000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0\n  2  3  1  0\nM  END\n";

const V3000: &str = "\n  t\n\n  0  0  0  0  0  0  0  0  0  0999 V3000\nM  V30 BEGIN CTAB\nM  V30 COUNTS 3 2 0 0 0\nM  V30 BEGIN ATOM\nM  V30 1 C 0 0 0 0\nM  V30 2 C 1.25 0.7 0 0\nM  V30 3 O 2.5 0 0 0\nM  V30 END ATOM\nM  V30 BEGIN BOND\nM  V30 1 1 1 2\nM  V30 2 1 2 3\nM  V30 END BOND\nM  V30 END CTAB\nM  END\n";

/// A V3000 chain of `n` carbons (over 999 atoms V3000 is the only form).
fn chain(n: usize) -> String {
    let atoms: String = (1..=n).map(|i| format!("M  V30 {i} C 0 0 0 0\n")).collect();
    let bonds: String = (1..n)
        .map(|i| format!("M  V30 {i} 1 {i} {}\n", i + 1))
        .collect();
    format!("\n  t\n\n  0  0  0  0  0  0  0  0  0  0999 V3000\nM  V30 BEGIN CTAB\nM  V30 COUNTS {n} {} 0 0 0\nM  V30 BEGIN ATOM\n{atoms}M  V30 END ATOM\nM  V30 BEGIN BOND\n{bonds}M  V30 END BOND\nM  V30 END CTAB\nM  END\n", n - 1)
}

fn run(a: &[&str]) -> Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn tmp(name: &str, body: &str) -> String {
    let p = std::env::temp_dir().join(format!("chem_v3000_{}_{name}", std::process::id()));
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

fn request(mf: &str) -> String {
    serde_json::json!({ "molfile": mf }).to_string()
}

#[test]
fn v2000_and_v3000_of_one_molecule_give_the_same_bytes_in_every_command_that_reads_one() {
    let (a, b) = (
        tmp("a.json", &request(V2000)),
        tmp("b.json", &request(V3000)),
    );
    for c in ["sel", "check", "cip", "mech"] {
        let (x, y) = (run(&["mol", c, &a]), run(&["mol", c, &b]));
        assert_eq!(x.code, y.code, "mol {c}: {}", y.stderr);
        assert_eq!(x.stdout, y.stdout, "mol {c}");
    }
    let sel = run(&["mol", "sel", &b]);
    assert_eq!(sel.code, 0);
    assert!(String::from_utf8(sel.stdout)
        .unwrap()
        .contains("\"formula\":\"C2H6O\""));
    let (a, b) = (tmp("a.mol", V2000), tmp("b.mol", V3000));
    for args in [vec!["export"], vec!["cut", "--set", "bonds=0-1"]] {
        let (mut x, mut y) = (vec!["mol", args[0], &a], vec!["mol", args[0], &b]);
        x.extend(&args[1..]);
        y.extend(&args[1..]);
        let (x, y) = (run(&x), run(&y));
        assert_eq!(
            (x.code, &x.stdout),
            (y.code, &y.stdout),
            "mol {}: {}",
            args[0],
            y.stderr
        );
        assert_eq!(y.code, 0, "mol {}: {}", args[0], y.stderr);
    }
}

#[test]
fn a_damaged_v3000_file_is_unreadable_for_every_command_that_reads_one() {
    // the counts line says three atoms and two bonds, the file holds one bond fewer: a file cut short
    let cut_short = V3000.replace("M  V30 2 1 2 3\n", "");
    let req = tmp("short.json", &request(&cut_short));
    for c in ["sel", "check", "cip", "mech"] {
        let o = run(&["mol", c, &req]);
        assert_eq!(o.code, 4, "mol {c}: {}", o.stderr);
        let o = run(&["mol", c, &req, "--json"]);
        assert!(
            String::from_utf8(o.stdout)
                .unwrap()
                .starts_with("{\"schema\":\"error/1\",\"code\":4"),
            "mol {c}"
        );
    }
    let bare = tmp("short.mol", &cut_short);
    for a in [
        vec!["mol", "export", &bare],
        vec!["mol", "cut", &bare, "--set", "bonds=0-1"],
    ] {
        assert_eq!(run(&a).code, 4, "{a:?}");
    }
}

#[test]
fn a_file_above_999_atoms_is_read_or_refused_with_its_code_never_empty() {
    let big = chain(1200);
    // the export reads it (V3000 is the only form of it) and writes it back whole
    let o = run(&["mol", "export", &tmp("big.mol", &big)]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let out = String::from_utf8(o.stdout).unwrap();
    assert!(
        out.contains("V3000") && out.contains("COUNTS 1200 1199"),
        "{}",
        &out[..200]
    );
    // the graph and the check have an atom limit: data refused (1), their error key in the answer
    let req = tmp("big.json", &request(&big));
    for c in ["sel", "check", "mech"] {
        let o = run(&["mol", c, &req, "--json"]);
        assert_eq!(o.code, 1, "mol {c}: {}", o.stderr);
        let text = String::from_utf8(o.stdout).unwrap();
        assert!(
            text.starts_with("{\"schema\":\"error/1\",\"code\":1") && text.contains("tooBig"),
            "mol {c}: {text}"
        );
    }
    // the cut reads the bare file: the same refusal, exit 1 with the key in the answer
    let bare = tmp("big.mol", &big);
    let o = run(&["mol", "cut", &bare, "--set", "bonds=0-1", "--json"]);
    assert_eq!(o.code, 1, "mol cut: {}", o.stderr);
    let text = String::from_utf8(o.stdout).unwrap();
    assert!(text.contains("tooBig"), "mol cut: {text}");
}
