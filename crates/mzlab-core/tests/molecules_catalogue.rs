//! The built-in catalogue (`mzlab/web/molecules/catalogue.csv`, environmental compounds only): canonical form, the must-have names (common EU pesticides,
//! pharmaceuticals, contaminants) found by a search, the removed designer-drug and forensic codes gone, the ranking of a typed prefix and the m/z of the
//! screening list. The molecules of the other tests are invented; these read the real catalogue.

use mzlab_core::json::parse;
use mzlab_core::molecules::{from_csv, route, search, to_csv, Molecule};

/// The archive of the engine is one for the whole process: the tests that load it take turns.
static ARCHIVE: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn catalogue() -> (String, Vec<Molecule>) {
    let text = std::fs::read_to_string(root().join("mzlab/web/molecules/catalogue.csv")).unwrap();
    let all =
        from_csv(&text, "catalog", None).unwrap_or_else(|e| panic!("{:?}", &e[..e.len().min(3)]));
    (text, all)
}

fn quoted(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}

#[test]
fn the_catalogue_is_canonical_and_complete() {
    let (text, all) = catalogue();
    assert_eq!(to_csv(&all), text, "mzlab/web/molecules/catalogue.csv is not in the canonical form: run tools/generate_molecules.py");
    assert!(all.iter().all(|m| m.origin.catalog == ["hrms"]));
    assert!(all.iter().all(|m| !m.formula.is_empty() && m.smiles.as_deref().is_some_and(|s| !s.is_empty())));
}

fn list_of(file: &str) -> Vec<String> {
    std::fs::read_to_string(root().join("tests/data").join(file))
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn carries(all: &[Molecule], name: &str) -> bool {
    let w = name.to_lowercase();
    all.iter().any(|m| {
        m.name.to_lowercase() == w || m.synonyms.iter().any(|s| s.to_lowercase() == w)
    })
}

/// The catalogue is environmental: every name of the must-have list (common EU pesticides, pharmaceuticals, contaminants) is there and found by a search, and none
/// of the removed designer-drug and forensic codes is left (as a name or as a synonym).
#[test]
fn the_catalogue_has_the_must_have_names_and_none_of_the_removed_codes() {
    let (_, all) = catalogue();
    let missing: Vec<String> = list_of("catalogue_must_have.txt")
        .into_iter()
        .filter(|n| !carries(&all, n))
        .collect();
    assert!(missing.is_empty(), "missing from the catalogue: {missing:?}");
    for n in list_of("catalogue_must_have.txt") {
        let hits = search(&all, &n, 8, 10.0);
        assert!(
            hits.iter().any(|h| {
                let m = &all[h.0];
                m.name.eq_ignore_ascii_case(&n) || m.synonyms.iter().any(|s| s.eq_ignore_ascii_case(&n))
            }),
            "{n:?} is not in the first 8 answers"
        );
    }
    let kept: Vec<String> = list_of("catalogue_removed.txt")
        .into_iter()
        .filter(|n| carries(&all, n))
        .collect();
    assert!(kept.is_empty(), "removed codes still there: {kept:?}");
}

/// A typed prefix gives the whole names first (equal, then starts with, then a word start, then contains), so «dimeth» lists dimethomorph and its neighbours
/// before any name that merely contains it; and the formula of a pesticide finds it.
#[test]
fn prefix_matches_come_first_and_the_formula_finds_the_compound() {
    let (_, all) = catalogue();
    let names: Vec<String> = search(&all, "dimeth", 8, 10.0)
        .iter()
        .map(|h| all[h.0].name.clone())
        .collect();
    assert!(names.iter().any(|n| n == "dimethomorph"), "{names:?}");
    let hits = search(&all, "dimeth", usize::MAX, 10.0);
    let scores: Vec<f64> = hits.iter().map(|h| h.1).collect();
    assert!(scores.windows(2).all(|w| w[0] <= w[1]), "not ordered by rank");
    assert!(hits.iter().take_while(|h| h.1 <= 1.0).any(|h| all[h.0].name == "dimethomorph"));
    let by_formula = search(&all, "C21H22ClNO4", 8, 10.0);
    assert!(by_formula.iter().any(|h| all[h.0].name == "dimethomorph"));
}

/// The screening list (`list` on the `hrms` catalogue) has every record of the catalogue, with the m/z of each adduct computed by the engine from the formula.
#[test]
fn the_screening_list_gives_the_mz_of_every_record() {
    let (text, all) = catalogue();
    let _g = ARCHIVE.lock().unwrap_or_else(|e| e.into_inner());
    route("load", &format!("{{\"catalog\":{}}}", quoted(&text))).unwrap();
    let out = route(
        "list",
        "{\"catalog\":\"hrms\",\"adducts\":[\"[M+H]+\",\"[M-H]-\"]}",
    )
    .unwrap();
    let list = parse(out.as_bytes()).unwrap();
    let rows = list.arr();
    assert_eq!(rows.len(), all.len());
    const PROTON: f64 = 1.007276466;
    for (r, m) in rows.iter().zip(&all) {
        let mz: Vec<f64> = r.get("mz").unwrap().arr().iter().map(|x| x.f64().unwrap()).collect();
        assert!((mz[0] - (m.mass + PROTON)).abs() <= 1e-5, "{}", m.name);
        assert!((mz[1] - (m.mass - PROTON)).abs() <= 1e-5, "{}", m.name);
    }
}

