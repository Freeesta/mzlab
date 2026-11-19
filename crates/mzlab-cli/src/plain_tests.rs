//! The Italian command names are refused; the English ones (and the one English old spelling, `peaks-edges`) answer (`plain.rs`, `lib.rs`).

use crate::execute;

fn run(args: &[&str]) -> crate::Outcome {
    execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

#[test]
fn the_italian_command_names_are_refused() {
    for old in ["canali", "campioni", "variabili", "nome-gruppo"] {
        assert_eq!(run(&[old, "--help"]).code, 2, "{old}");
    }
    for new in ["channels", "samples", "peak-edges"] {
        assert_eq!(run(&[new, "--help"]).code, 0, "{new}");
    }
    // `peaks-edges` is the one old command spelling still accepted (it is English)
    assert_eq!(
        run(&["peaks-edges", "--help"]).stdout,
        run(&["peak-edges", "--help"]).stdout
    );
    let top = String::from_utf8_lossy(&run(&["--help"]).stdout).into_owned();
    // the citation reminder is the footer, after the list of commands
    let (body, footer) = top.trim_end().rsplit_once("\n\n").unwrap();
    assert!(body.contains("Exit codes: 0 ok;") && !body.contains("old names still accepted"));
    assert_eq!(footer, crate::citation::reminder());
    assert!(!top.contains("mzlab-cli canali") && !top.contains("mzlab-cli campioni"));
}
