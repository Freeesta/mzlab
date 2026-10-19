//! The truths `verita/1` of the family `sciex` against its reader (Rosina bench, docs/studi/ROSINA.md F): private corpus, skipped without `MZLAB_DATI`.
#![cfg(feature = "sciex")]

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;
#[path = "comune/verita.rs"]
mod verita;

#[test]
fn verita_sciex() {
    verita::corpus(
        "sciex",
        |r| r.starts_with("sciex/"),
        verita::Tol::LR,
        dati_percorsi::percorso,
    );
}
