//! The truths `verita/1` of the family `sciex` against its reader (Rosina bench, docs/studi/ROSINA.md F): private corpus, skipped without `MZLAB_DATI`.
#![cfg(feature = "sciex")]

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;
#[path = "common/truth.rs"]
mod truth;

#[test]
fn truth_sciex() {
    truth::corpus(
        "sciex",
        |r| r.starts_with("sciex/"),
        truth::Tol::LR,
        dati_percorsi::percorso,
    );
}
