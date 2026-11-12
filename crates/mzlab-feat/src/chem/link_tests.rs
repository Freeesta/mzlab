//! Tests of `link/1`: the percent-encoding of the characters SMILES and InChI use, the rows and the preferred PubChem search.

use super::*;

#[test]
fn pct_encodes_every_reserved_byte() {
    assert_eq!(pct("C#N"), "C%23N");
    assert_eq!(pct("[NH4+]"), "%5BNH4%2B%5D");
    assert_eq!(pct("C/C=C\\C"), "C%2FC%3DC%5CC");
    assert_eq!(
        pct("N[C@@H](C)C(=O)O"),
        "N%5BC%40%40H%5D%28C%29C%28%3DO%29O"
    );
    assert_eq!(pct("a-b.c_d~e%"), "a-b.c_d~e%25");
    assert_eq!(pct("é"), "%C3%A9");
}

#[test]
fn whole_drawing_prefers_exact_and_part_prefers_substructure() {
    let w = link_json(r#"{"smiles":"CCO","part":false}"#).unwrap();
    assert!(
        w.starts_with(r#"{"format":"link/1","part":false,"preferred":"pubchem-exact","#),
        "{w}"
    );
    let p = link_json(r#"{"smiles":"CCO","part":true}"#).unwrap();
    assert!(p.contains(r#""preferred":"pubchem-sub""#), "{p}");
    assert!(!p.contains("chebi"), "no InChI, no ChEBI row: {p}");
}

#[test]
fn chebi_carries_the_encoded_inchi() {
    let r = link_json(r#"{"smiles":"CO","inchi":"InChI=1S/CH4O/c1-2/h2H,1H3"}"#).unwrap();
    assert!(
        r.contains(r#""url":"https://www.ebi.ac.uk/chebi/search?query=InChI%3D1S%2FCH4O%2Fc1-2%2Fh2H%2C1H3","copy":null"#),
        "{r}"
    );
    assert!(
        r.contains(r#"{"id":"biotransformer","url":"https://biotransformer.ca/new","copy":"CO"}"#),
        "{r}"
    );
}

#[test]
fn bad_requests_keep_the_key() {
    for r in [
        "",
        "{}",
        r#"{"smiles":""}"#,
        r#"{"smiles":"C C"}"#,
        r#"{"smiles":3}"#,
    ] {
        assert_eq!(link_json(r).unwrap_err().key, "error.link.request", "{r}");
    }
}
