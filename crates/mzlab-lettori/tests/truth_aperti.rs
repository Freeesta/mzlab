//! The truths `verita/1` of the family `aperti` against its reader (Rosina bench, docs/studi/ROSINA.md F): private corpus, skipped without `MZLAB_DATI`.
#![cfg(feature = "aperti")]

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;
#[path = "common/truth.rs"]
mod truth;

#[test]
fn open_truth() {
    truth::corpus(
        "aperti",
        |r| {
            let r = r.to_lowercase();
            r.ends_with(".mzxml") || r.ends_with(".mzml")
        },
        truth::Tol::HR,
        dati_percorsi::percorso,
    );
}
