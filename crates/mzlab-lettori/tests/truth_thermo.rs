//! The truths `verita/1` of the family `thermo` against its reader (Rosina bench, docs/studi/ROSINA.md F): private corpus, skipped without `MZLAB_DATI`.
#![cfg(feature = "thermo")]

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;
#[path = "common/truth.rs"]
mod truth;

#[test]
fn truth_thermo() {
    truth::corpus(
        "thermo",
        |r| r.starts_with("thermo/"),
        truth::Tol::HR,
        dati_percorsi::percorso,
    );
}
