//! The truths `verita/1` of the family `andi` against its reader (Rosina bench, docs/studi/ROSINA.md F): private corpus, skipped without `MZLAB_DATI`.
#![cfg(feature = "andi")]

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;
#[path = "comune/verita.rs"]
mod verita;

#[test]
fn verita_andi() {
    verita::corpus(
        "andi",
        |r| r.to_lowercase().ends_with(".cdf"),
        verita::Tol::LR,
        dati_percorsi::percorso,
    );
}
