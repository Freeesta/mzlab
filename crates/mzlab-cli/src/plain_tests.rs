//! The short English command names and their old spellings (`plain.rs`, `lib.rs`): same stdout bytes and exit code.

use crate::execute;

fn run(args: &[&str]) -> crate::Outcome {
    execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn the_short_english_names_and_the_old_ones_are_the_same_command() {
    for (old, new) in [
        ("canali", "channels"),
        ("campioni", "samples"),
        ("peaks-edges", "peak-edges"),
    ] {
        let (o, n) = (run(&[old, "--help"]), run(&[new, "--help"]));
        assert_eq!(
            (o.code, o.stdout.clone()),
            (n.code, n.stdout.clone()),
            "{old}"
        );
        assert!(
            String::from_utf8_lossy(&n.stdout).contains("old names still accepted"),
            "{new}"
        );
    }
    let top = String::from_utf8_lossy(&run(&["--help"]).stdout).into_owned();
    assert!(top.trim_end().ends_with(
        "old names still accepted: canali, campioni, peaks-edges, nome-gruppo, variabili, --canale, --jsonl"
    ));
    assert!(!top.contains("mzlab-cli canali") && !top.contains("mzlab-cli campioni"));
}
