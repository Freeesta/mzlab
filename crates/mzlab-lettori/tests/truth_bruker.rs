//! The truths `verita/1` of the family `bruker` against its reader (Rosina bench, docs/studi/ROSINA.md F): private corpus, skipped without `MZLAB_DATI`.
#![cfg(feature = "bruker")]

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;
#[path = "common/truth.rs"]
mod truth;

#[test]
fn truth_bruker() {
    truth::corpus(
        "bruker",
        |r| r.starts_with("bruker/"),
        truth::Tol::HR,
        dati_percorsi::percorso,
    );
}
