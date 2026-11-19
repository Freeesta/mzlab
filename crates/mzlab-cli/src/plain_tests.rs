//! The Italian command names are refused and the old flat names are gone; the tree names answer (`cmd/`, `lib.rs`).

use crate::execute;

fn run(args: &[&str]) -> crate::Outcome {
    execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn the_italian_command_names_are_refused() {
    for old in ["canali", "campioni", "variabili", "nome-gruppo"] {
        assert_eq!(run(&[old, "--help"]).code, 2, "{old}");
    }
    for gone in ["channels", "samples", "peak-edges", "peaks-edges", "steps"] {
        assert_eq!(run(&[gone, "--help"]).code, 2, "{gone}");
    }
    for new in [
        &["file", "channels"][..],
        &["sample", "vars"],
        &["peak", "edges"],
    ] {
        let mut a = new.to_vec();
        a.push("--help");
        assert_eq!(run(&a).code, 0, "{new:?}");
    }
    let top = String::from_utf8_lossy(&run(&["--help"]).stdout).into_owned();
    // the citation reminder is the footer, after the list of commands
    let (body, footer) = top.trim_end().rsplit_once("\n\n").unwrap();
    assert!(body.contains("Exit codes: 0 ok;") && !body.contains("old names still accepted"));
    assert_eq!(footer, crate::citation::reminder());
    assert!(!top.contains("mzlab-cli canali") && !top.contains("mzlab-cli campioni"));
}
