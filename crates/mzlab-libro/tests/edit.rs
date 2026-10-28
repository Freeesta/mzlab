//! Editing from the preview: which blocks carry `data-src`, how an edited block goes back into the source, and what is refused.
use mzlab_libro::{edit_block, preview, Options};

const MD: &str = "# Titolo\n\n{.lead} Un paragrafo con **grassetto** e H~2~O.\n\nSecondo paragrafo semplice.\n\n- voce uno\n- voce due\n\n1. primo\n\nTabella: Prova\n| A | B |\n|---|---|\n| 1 |  testo |\n\n> [!nota] Riquadro\n> Dentro il riquadro.\n\n![Schema del metodo](img/schema.svg \"Didascalia **forte**\") {#fig:s}\n\n??? Che cosa misura?\nMisura la massa\nsu carica.\n";

fn opt() -> Options {
    Options {
        teoria: true,
        ..Options::default()
    }
}

fn edit(src: &str, html: &str) -> Result<String, String> {
    edit_block(MD, "01-prova.md", &opt(), src, html)
}

#[test]
fn only_simple_top_level_blocks_are_editable() {
    let p = preview(MD, "01-prova.md", &opt()).unwrap();
    for key in [
        "3-3", "5-5", "7", "8", "10", "13:0", "13:1", "15:0", "15:1", "t12", "f20", "qs22",
        "qa23-24",
    ] {
        assert!(
            p.contains(&format!("data-src=\"{key}\"")),
            "{key} missing in {p}"
        );
    }
    assert_eq!(
        p.matches("data-src").count(),
        13,
        "headings, the box and its paragraph stay locked"
    );
    assert!(!p.contains("<h1 data-src"));
}

#[test]
fn a_preview_without_the_marks_is_the_real_page() {
    let real = mzlab_libro::convert(MD, "01-prova.md", &opt()).unwrap();
    let marked = preview(MD, "01-prova.md", &opt()).unwrap();
    let mut stripped = marked.clone();
    while let Some(a) = stripped.find(" data-src=\"") {
        let end = a + 11 + stripped[a + 11..].find('"').unwrap() + 1;
        stripped.replace_range(a..end, "");
    }
    // the preview wraps a figure caption in a bare <span>
    let w = stripped.find("<span>").unwrap();
    stripped.replace_range(w..w + 6, "");
    let w = stripped[w..].find("</span>").unwrap() + w;
    stripped.replace_range(w..w + 7, "");
    assert_eq!(stripped, real);
}

#[test]
fn a_changed_word_goes_to_the_right_line() {
    let r = edit("5-5", "Secondo paragrafo <i>cambiato</i>.").unwrap();
    assert_eq!(
        r,
        MD.replace(
            "Secondo paragrafo semplice.",
            "Secondo paragrafo *cambiato*."
        )
    );
}

#[test]
fn inline_markup_is_kept_and_the_lead_class_too() {
    let r = edit(
        "3-3",
        "Un paragrafo con <b>grassetto</b> e H<sub>2</sub>O in <em>più</em>.",
    )
    .unwrap();
    assert!(
        r.contains("{.lead} Un paragrafo con **grassetto** e H~2~O in _più_.\n"),
        "{r}"
    );
}

#[test]
fn the_browser_spellings_are_accepted() {
    let r = edit("5-5", "Secondo&nbsp;paragrafo <strong>forte</strong> &amp; <span style=\"color: red\">rosso</span><br>").unwrap();
    assert!(r.contains("Secondo paragrafo **forte** & rosso\n"), "{r}");
}

#[test]
fn list_items_keep_their_marker() {
    assert!(edit("7", "voce <b>1</b>")
        .unwrap()
        .contains("- voce **1**\n- voce due\n"));
    assert!(edit("10", "uno").unwrap().contains("\n1. uno\n"));
}

#[test]
fn a_table_cell_keeps_the_padding_of_the_row() {
    assert!(edit("15:1", "nuovo testo")
        .unwrap()
        .contains("| 1 |  nuovo testo |\n"));
    assert!(edit("13:0", "Colonna")
        .unwrap()
        .contains("| Colonna | B |\n"));
}

#[test]
fn what_would_change_meaning_is_refused() {
    for html in [
        "a *b* c",
        "a `b` c",
        "[x](y.md)",
        "a $b$ c",
        "a [@k] c",
        "a @eq:x c",
        "a_b_c _d_ e",
    ] {
        let e = edit("5-5", html).unwrap_err();
        assert!(e.contains("edit it in the source box"), "{html}: {e}");
    }
    assert!(
        edit("5-5", "- una voce")
            .unwrap_err()
            .contains("source box"),
        "a typed marker would make a list"
    );
    assert!(edit("5-5", "# titolo").unwrap_err().contains("source box"));
}

#[test]
fn unknown_markup_and_empty_blocks_are_refused() {
    assert!(edit("5-5", "a <u>b</u>")
        .unwrap_err()
        .contains("can't be kept"));
    assert!(edit("5-5", "<img src=x>")
        .unwrap_err()
        .contains("can't be kept"));
    assert!(edit("5-5", "<b>  </b>").is_err());
    assert!(edit("15:1", "a | b").unwrap_err().contains("| character"));
    assert!(edit("99-99", "x").is_err());
}

#[test]
fn captions_and_questions_are_edited_in_place() {
    let r = edit("t12", "Tabella <b>nuova</b>").unwrap();
    assert!(r.contains("\nTabella: Tabella **nuova**\n| A | B |"), "{r}");
    let r = edit("f20", "Un'altra <i>didascalia</i>").unwrap();
    assert!(
        r.contains("![Schema del metodo](img/schema.svg \"Un'altra *didascalia*\") {#fig:s}"),
        "{r}"
    );
    let r = edit("qs22", "Cosa misura il rivelatore?").unwrap();
    assert!(
        r.contains("\n??? Cosa misura il rivelatore?\nMisura la massa\n"),
        "{r}"
    );
    let r = edit(
        "qa23-24",
        "Misura la <b>massa</b> su carica, <em>sempre</em>.",
    )
    .unwrap();
    assert!(
        r.ends_with("??? Che cosa misura?\nMisura la **massa** su carica, _sempre_.\n"),
        "answer lines collapse into one: {r}"
    );
}

#[test]
fn a_caption_that_cannot_be_written_back_is_refused() {
    assert!(edit("f20", "dice \"ciao\"")
        .unwrap_err()
        .contains("\" character"));
    assert!(edit("t12", "a *b* c").unwrap_err().contains("source box"));
    assert!(edit("qs22", "").is_err() && edit("qa23-24", "<b> </b>").is_err());
    assert!(edit("f21", "x").is_err(), "a key that is not a figure line");
}

const SEZ: &str = "# T\n\n## Prima sezione\n\nTesto.\n\n### Dettaglio {#dett}\n\nAltro.\n";

fn edit_sez(src: &str, html: &str) -> Result<String, String> {
    edit_block(SEZ, "01-prova.md", &opt(), src, html)
}

#[test]
fn section_titles_are_editable_and_keep_their_anchor() {
    let p = preview(SEZ, "01-prova.md", &opt()).unwrap();
    assert!(
        p.contains("<h2 data-src=\"h3\">")
            && p.contains("<h3 id=\"dett\" data-src=\"h7\">")
            && !p.contains("<h1 data-src")
    );
    assert_eq!(
        edit_sez("h3", "Nuova <i>sezione</i>").unwrap(),
        SEZ.replace("## Prima sezione", "## Nuova *sezione* {#prima-sezione}")
    );
    assert_eq!(
        edit_sez("h7", "Dettaglio nuovo").unwrap(),
        SEZ.replace("### Dettaglio {#dett}", "### Dettaglio nuovo {#dett}")
    );
    assert!(edit_sez("h3", "uno {#x}").is_err());
    assert!(
        edit_sez("h1", "x").is_err(),
        "the title of the chapter is not a section"
    );
}

#[test]
fn the_headings_check_allows_only_retitled_sections_with_their_anchors() {
    use mzlab_libro::retitled;
    let n = "01-prova.md";
    assert_eq!(retitled(SEZ, SEZ, n), Ok(false));
    let ok = SEZ.replace("## Prima sezione", "## Altro titolo {#prima-sezione}");
    assert_eq!(retitled(SEZ, &ok, n), Ok(true));
    assert!(
        retitled(SEZ, &SEZ.replace("## Prima sezione", "## Altro titolo"), n)
            .unwrap_err()
            .contains("{#prima-sezione}")
    );
    assert!(retitled(SEZ, &SEZ.replace("{#dett}", "{#altro}"), n)
        .unwrap_err()
        .contains("must stay"));
    assert!(retitled(SEZ, &SEZ.replace("# T\n", "# Altro\n"), n)
        .unwrap_err()
        .contains("title of the chapter"));
    assert!(retitled(
        SEZ,
        &SEZ.replace("### Dettaglio {#dett}", "## Dettaglio {#dett}"),
        n
    )
    .unwrap_err()
    .contains("level"));
    assert!(retitled(SEZ, &format!("{SEZ}\n## Nuova\n"), n)
        .unwrap_err()
        .contains("added, removed"));
    assert!(retitled(
        &ok,
        &ok.replace("Altro titolo", "Ancora"),
        "21-glossario.md"
    )
    .unwrap_err()
    .contains("glossary"));
}

#[test]
fn a_reversed_line_range_is_an_unknown_block_not_a_panic() {
    for src in ["5-2", "3-2", "qa24-23", "qa5-2"] {
        assert_eq!(
            edit(src, "Testo"),
            Err("Unknown block.".to_string()),
            "{src}"
        );
    }
}
