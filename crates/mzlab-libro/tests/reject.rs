//! Sources the converter must refuse instead of guessing: a repeated figure id and a NUL character.
use mzlab_libro::{convert, Options};

fn opt() -> Options {
    Options {
        teoria: true,
        ..Options::default()
    }
}

fn err(md: &str) -> String {
    convert(md, "01-p.md", &opt()).unwrap_err().to_string()
}

#[test]
fn a_repeated_figure_id_is_an_error_naming_the_id_and_line() {
    let md =
        "# T\n\n![alt uno](img/a.svg \"c1\") {#fig:x}\n\n![alt due](img/b.svg \"c2\") {#fig:x}\n";
    assert_eq!(err(md), "01-p.md:5: duplicate figure id x");
    // distinct ids still convert and are numbered in order
    let ok = md
        .replace("{#fig:x}\n", "{#fig:y}\n")
        .replacen("{#fig:y}", "{#fig:x}", 1);
    let html = convert(&ok, "01-p.md", &opt()).unwrap();
    assert!(html.contains("<b>Figura 01-p.1</b>") && html.contains("<b>Figura 01-p.2</b>"));
}

#[test]
fn a_nul_character_is_an_error_naming_the_line() {
    let md = "# T\n\nUn `codice` e poi \x000\x00 fine **b**.\n";
    assert_eq!(err(md), "01-p.md:3: NUL character in the source");
    // a clean source with the same inline constructs still converts
    assert!(convert("# T\n\nUn `codice` fine **b**.\n", "01-p.md", &opt()).is_ok());
}
