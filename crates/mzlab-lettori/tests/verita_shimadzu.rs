//! The truths `verita/1` of the family `shimadzu` against its reader (Rosina bench, docs/studi/ROSINA.md F): private corpus, skipped without `MZLAB_DATI`.
#![cfg(feature = "shimadzu")]

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;
#[path = "comune/verita.rs"]
mod verita;

#[test]
fn verita_shimadzu() {
    verita::corpus(
        "shimadzu",
        |r| r.starts_with("shimadzu/"),
        verita::Tol::LR,
        dati_percorsi::percorso,
    );
}
