//! The truths `verita/1` of the family `thermo` against its reader (Rosina bench, docs/studi/ROSINA.md F): private corpus, skipped without `MZLAB_DATI`.
#![cfg(feature = "thermo")]

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;
#[path = "comune/verita.rs"]
mod verita;

#[test]
fn verita_thermo() {
    verita::corpus(
        "thermo",
        |r| r.starts_with("thermo/"),
        verita::Tol::HR,
        dati_percorsi::percorso,
    );
}
