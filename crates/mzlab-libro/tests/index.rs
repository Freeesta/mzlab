//! The search index rebuilt for one chapter: for every chapter page of the Teoria it must give back the committed `indice-ricerca.js` byte for byte
//! (so the Rust port of tools/genera_indice_teoria.py is the generator), and a retitled section changes only the rows of its chapter.
use std::fs;
use std::path::PathBuf;

use mzlab_libro::reindex;

fn teoria() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/teoria")
}

/// The page files that have a title row of their own in the index.
fn chapter_files(index: &str) -> Vec<String> {
    index
        .lines()
        .filter(|l| l.contains("\", \"capitolo\", \"") || l.contains("\", \"pratica\", \""))
        .map(|l| l.split("\", \"").nth(2).unwrap().to_string())
        .collect()
}

#[test]
fn every_chapter_gives_back_the_committed_index() {
    let index = fs::read_to_string(teoria().join("indice-ricerca.js")).unwrap();
    let files = chapter_files(&index);
    assert!(files.len() >= 30, "chapters found: {}", files.len());
    // the glossary has rows of its own at the end of the index (its terms): its titles are not retitled from the editor
    for f in files.iter().filter(|f| *f != "21-glossario.html") {
        let html = fs::read_to_string(teoria().join(f)).unwrap();
        let again = reindex(&index, f, &html).unwrap_or_else(|e| panic!("{f}: {e}"));
        if again != index {
            let (a, b) = (index.lines(), again.lines());
            let d = a
                .zip(b)
                .find(|(x, y)| x != y)
                .map(|(x, y)| format!("{x}\n{y}"))
                .unwrap_or_else(|| "length".into());
            panic!(
                "{f}: the rebuilt index differs at\n{}",
                d.chars().take(600).collect::<String>()
            );
        }
    }
}

#[test]
fn the_glossary_is_refused() {
    let index = fs::read_to_string(teoria().join("indice-ricerca.js")).unwrap();
    let html = fs::read_to_string(teoria().join("21-glossario.html")).unwrap();
    assert!(reindex(&index, "21-glossario.html", &html).is_err());
}

#[test]
fn a_retitled_section_changes_only_its_own_rows() {
    let index = fs::read_to_string(teoria().join("indice-ricerca.js")).unwrap();
    let html = fs::read_to_string(teoria().join("05-gc.html")).unwrap();
    let h2 = html.find("<h2").unwrap();
    let gt = html[h2..].find('>').unwrap() + h2 + 1;
    let end = html[gt..].find("</h2>").unwrap() + gt;
    let new = format!("{}Titolo àèì nuovo{}", &html[..gt], &html[end..]);
    let out = reindex(&index, "05-gc.html", &new).unwrap();
    let (a, b): (Vec<&str>, Vec<&str>) = (index.lines().collect(), out.lines().collect());
    assert_eq!(a.len(), b.len());
    let changed: Vec<&&str> = b.iter().filter(|l| !a.contains(l)).collect();
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert!(
        changed[0].contains("\"Titolo àèì nuovo\", \"sezione\", \"05-gc.html#titolo-aei-nuovo\", ")
            && changed[0].contains("titolo aei nuovo"),
        "{}",
        changed[0]
    );
}

#[test]
fn an_unknown_entity_or_chapter_is_refused() {
    let index = fs::read_to_string(teoria().join("indice-ricerca.js")).unwrap();
    assert!(reindex(&index, "99-niente.html", "<h2>x</h2>")
        .is_err_and(|e| e.contains("not in the search index")));
    assert!(reindex(&index, "05-gc.html", "<h2>a &unknownent; b</h2>")
        .is_err_and(|e| e.contains("unsupported entity")));
}
