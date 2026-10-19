#![cfg(feature = "chem")]
//! Dumps the number of classes and sites of `+O`, `-CH2` and `-2H` for the SMILES in `MZLAB_SITES_IN` (a JSON list) to `MZLAB_SITES_OUT`, for
//! `tests/test_interp_riferimento.py`, which compares them with RDKit. Without the variables it does nothing.

use std::collections::BTreeSet;

use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::sites;

fn counts(smiles: &str, transform: &str) -> String {
    let Ok(s) = sites(&Mol::parse(smiles).unwrap(), transform) else {
        return "[0,0]".into();
    };
    let classes: BTreeSet<_> = s.iter().map(|x| (x.class, x.kind)).collect();
    format!("[{},{}]", classes.len(), s.len())
}

#[test]
fn dump_class_counts_for_the_rdkit_reference() {
    let (Ok(input), Ok(out)) = (
        std::env::var("MZLAB_SITES_IN"),
        std::env::var("MZLAB_SITES_OUT"),
    ) else {
        return;
    };
    let list: Vec<String> = std::fs::read_to_string(input)
        .unwrap()
        .trim()
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .map(|x| x.trim().trim_matches('"').to_string())
        .collect();
    let rows: Vec<String> = list
        .iter()
        .map(|smi| {
            format!(
                "{{\"smiles\":\"{smi}\",\"+O\":{},\"-CH2\":{},\"-2H\":{}}}",
                counts(smi, "+O"),
                counts(smi, "-CH2"),
                counts(smi, "-2H")
            )
        })
        .collect();
    std::fs::write(out, format!("[{}]", rows.join(","))).unwrap();
}
