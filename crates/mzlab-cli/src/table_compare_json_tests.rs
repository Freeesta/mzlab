//! `table compare --json` and the usage errors that come from the engine (`err.compare.quantity`, a selection that leaves nothing to treat).

use super::{run, table, SEL};

fn args<'a>(t: &'a str, more: &[&'a str]) -> Vec<&'a str> {
    [&["table", "compare", t][..], more, &SEL].concat()
}

#[test]
fn json_is_the_default_form_and_csv_cannot_be_combined_with_it() {
    let (_, t) = table("json_flag");
    let plain = run(&args(&t, &[]));
    assert_eq!(plain.0, 0, "{}", plain.2);
    // --json and --format json are the default, byte for byte, for a run and for a plan
    assert_eq!(run(&args(&t, &["--json"])), plain);
    assert_eq!(run(&args(&t, &["--json", "--format", "json"])), plain);
    let dry = run(&args(&t, &["--dry-run"]));
    assert_eq!(run(&args(&t, &["--dry-run", "--json"])), dry);
    // CSV is another form of the same answer: asking for both is a usage error, never a silent choice
    let (code, out, _) = run(&args(&t, &["--json", "--format", "csv"]));
    assert_eq!(code, 2);
    assert!(
        out.contains(r#""error_key":"err.cli.usage""#) && out.contains("exclude each other"),
        "{out}"
    );
    // a failure with --json is the one error/1 on stdout, with its own code
    let (code, out, _) = run(&[
        "table", "compare", &t, "--json", "--ref", "nothing*", "--blank", "MQ*",
    ]);
    assert_eq!(code, 5);
    assert!(
        out.starts_with(r#"{"schema":"error/1""#) && out.contains("err.compare.selector"),
        "{out}"
    );
}

#[test]
fn a_height_of_a_csv_and_a_selection_with_nothing_to_treat_are_usage_errors() {
    let (_, t) = table("engine_usage");
    let (code, _, err) = run(&[
        "table",
        "compare",
        &t,
        "--ref",
        "s_t0",
        "--blank",
        "MQ*",
        "--quantity",
        "height",
    ]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("err.compare.quantity"), "{err}");
    // every column is a reference or a blank: no treated column is left
    let (code, _, err) = run(&["table", "compare", &t, "--ref", "s_t*", "--blank", "MQ*"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("err.stat.ref.notreated"), "{err}");
}
