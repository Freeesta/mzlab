//! The search box of a parent compound: the wide search (InChIKey, one slip of a letter), the table of the adducts and what a typed formula or m/z stands for (the neutral M or an
//! ion of the table). Known values: bentazone C10H12N2O3S, 240.056863 Da. The molecules of the tests are invented except that public formula.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use mzlab_core::json::{parse, Value};
use mzlab_core::molecules::{cerca, from_csv, route, Molecola};

fn ms(csv: &str) -> Vec<Molecola> {
    from_csv(csv, "utente", None).unwrap_or_else(|e| panic!("{e:?}"))
}

const PARENTS: &str = "\
name;synonyms;molecular_formula;inchikey
Bentazon;Bentazone;C10H12N2O3S;ZOMSMJKLGFBRBS-UHFFFAOYSA-N
Bentazone sodium;;C10H11N2NaO3S;AAAAAAAAAAAAAA-AAAAAAAAAA-A
Atrazina;;C8H14ClN5;MXWJVTOOROXGIU-UHFFFAOYSA-N
";

fn first_names(ms: &[Molecola], q: &str) -> Vec<String> {
    mzlab_core::molecules::cerca_ampia(ms, q, 8, 10.0)
        .iter()
        .map(|h| ms[h.0].nome.clone())
        .collect()
}

#[test]
fn wide_search_finds_inchikeys_and_one_letter_slips() {
    let v = ms(PARENTS);
    // the plain search is unchanged: a name that is not written right finds nothing
    assert!(cerca(&v, "bentazan", 8, 10.0).is_empty());
    // a slip of one letter (replaced, dropped, added), in the name or in a synonym
    for q in ["bentazan", "bentazne", "bentazzon", "Bentazonee"] {
        assert_eq!(first_names(&v, q)[0], "Bentazon", "{q}");
    }
    assert!(first_names(&v, "bentxzan").is_empty());
    assert!(first_names(&v, "xyz").is_empty());
    // what the plain search finds comes first and is not mixed with the guesses
    assert_eq!(
        first_names(&v, "bentazon"),
        ["Bentazon", "Bentazone sodium"]
    );
    // the InChIKey, whole or by its first block, in any case
    assert_eq!(first_names(&v, "zomsmjklgfbrbs-uhfffaoysa-n"), ["Bentazon"]);
    assert_eq!(first_names(&v, "MXWJVTOOROXGIU"), ["Atrazina"]);
    assert_eq!(first_names(&v, "C8H14ClN5"), ["Atrazina"]);
}

fn ion(q: &str, adduct: &str) -> Value {
    parse(
        route(
            "ione",
            &format!("{{\"q\":\"{q}\",\"adduct\":\"{adduct}\"}}"),
        )
        .unwrap()
        .as_bytes(),
    )
    .unwrap()
}

#[test]
fn a_typed_formula_or_mz_is_read_as_the_neutral_or_as_an_ion() {
    let f = |v: &Value, k: &str| v.get(k).and_then(|x| x.f64()).unwrap();
    let s = |v: &Value, k: &str| v.get(k).and_then(|x| x.str()).unwrap().to_string();
    // bentazone C10H12N2O3S, 240.0569: M itself
    let n = ion("C10H12N2O3S", "neutral");
    assert_eq!(
        (s(&n, "formula"), f(&n, "massa")),
        ("C10H12N2O3S".into(), 240.056863)
    );
    assert!(n.get("mz").and_then(|x| x.f64()).is_none());
    // the ion formulas of the same molecule give the same M, and the m/z of the table
    let h = ion("C10H13N2O3S", "[M+H]+");
    assert_eq!(s(&h, "formula"), "C10H12N2O3S");
    assert_eq!(f(&h, "massa"), 240.056863);
    assert!((f(&h, "mz") - 241.064139).abs() < 2e-6);
    let d = ion("C10H11N2O3S", "[M-H]-");
    assert_eq!(s(&d, "formula"), "C10H12N2O3S");
    assert!((f(&d, "mz") - 239.049587).abs() < 2e-6);
    assert_eq!(
        s(&ion("C10H12N2O3SNa", "[M+Na]+"), "formula"),
        "C10H12N2O3S"
    );
    // an m/z: M is the m/z minus the shift of the adduct, or the m/z itself
    let m = ion("241.064139", "[M+H]+");
    assert_eq!(
        (s(&m, "kind"), s(&m, "formula")),
        ("mass".into(), "".into())
    );
    assert!((f(&m, "massa") - 240.056863).abs() < 2e-6);
    assert_eq!(f(&ion("240.0569", "neutral"), "massa"), 240.0569);
    // an adduct the formula does not hold, or one that is not in the table, is an error
    assert!(route("ione", r#"{"q":"C10H12N2O3S","adduct":"[M+Na]+"}"#).is_err());
    assert!(route("ione", r#"{"q":"C10H12N2O3S","adduct":"[M+X]+"}"#).is_err());
    assert!(route("ione", r#"{"q":"-5","adduct":"neutral"}"#).is_err());
}

#[test]
fn the_adduct_list_is_the_table_of_the_engine() {
    assert_eq!(
        route("addotti", "{}").unwrap(),
        r#"["[M+H]+","[M+NH4]+","[M+Na]+","[M+K]+","[M-H]-","[M+Cl]-","[M+HCOO]-"]"#
    );
}
