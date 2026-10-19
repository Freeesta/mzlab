//! The truths `verita/1` of the family `waters` against its reader (Rosina bench, docs/studi/ROSINA.md F): private corpus, skipped without `MZLAB_DATI`.
#![cfg(feature = "waters")]

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;
#[path = "comune/verita.rs"]
mod verita;

#[test]
fn verita_waters() {
    verita::corpus(
        "waters",
        |r| r.starts_with("waters/"),
        verita::Tol::HR,
        dati_percorsi::percorso,
    );
}
