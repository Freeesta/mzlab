//! The truths `verita/1` of the family `shimadzu` against its reader (Rosina bench, docs/studi/ROSINA.md F): private corpus, skipped without `MZLAB_DATI`.
#![cfg(feature = "shimadzu")]

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;
#[path = "common/truth.rs"]
mod truth;

#[test]
fn truth_shimadzu() {
    truth::corpus(
        "shimadzu",
        |r| r.starts_with("shimadzu/"),
        truth::Tol::LR,
        dati_percorsi::percorso,
    );
}
