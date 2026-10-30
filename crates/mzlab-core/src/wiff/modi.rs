//! Table of the scan-type codes of `ExperimentHeader` (+122) of a `.wiff`: Analyst abbreviation, MS level, CV term of the spectrum and the output kind
//! of each. Read by `mod.rs`, `estesi.rs` and `lettore.rs`; the page shows the abbreviation as the exact type of an experiment. Codes 0, 4, 5, 6, 7, 11, 15 are
//! seen in real files (0 is the Q1 MS full scan of a 3200 QTRAP: level-1 profile scans like EMS, its meaning inferred from the method name and the
//! spectra); the others follow the `ScanType` enum of Analyst and stay `confermato: false` until a file shows them.

/// Codes seen in real files.
pub const CODE_Q1: u32 = 0;
pub const CODE_MRM: u32 = 4;
pub const CODE_PRECURSOR_ION: u32 = 5;
pub const CODE_PRODUCT_ION: u32 = 6;
pub const CODE_NEUTRAL_LOSS: u32 = 7;
pub const CODE_PRODUCT: u32 = 11;
pub const CODE_FULL: u32 = 15;

/// What an experiment of this type writes into the mzML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Uscita {
    Spettro,
    Srm,
    Sim,
}

#[derive(Debug, Clone, Copy)]
pub struct Modo {
    pub code: u32,
    /// Analyst abbreviation (EMS, EPI, MRM, MS3, NL, Prec, PI, Q1, Q3, Q1MI, Q3MI, EMC, ER, TDF).
    pub sigla: &'static str,
    pub level: u8,
    /// CV `(accession, name)` of the spectrum type, when it needs one.
    pub term: Term,
    pub uscita: Uscita,
    /// Seen in a real file: only these are read by the readers module.
    pub confermato: bool,
}

type Term = Option<(&'static str, &'static str)>;

#[rustfmt::skip]
const fn m(code: u32, sigla: &'static str, level: u8, term: Term, uscita: Uscita, confermato: bool) -> Modo {
    Modo { code, sigla, level, term, uscita, confermato }
}

use Uscita::{Sim, Spettro, Srm};

#[rustfmt::skip]
pub const MODI: [Modo; 17] = [
    m(CODE_Q1, "Q1", 1, None, Spettro, true),
    m(1, "Q1MI", 1, None, Sim, false),
    m(2, "Q3", 1, None, Spettro, false),
    m(3, "Q3MI", 1, None, Sim, false),
    m(CODE_MRM, "MRM", 2, None, Srm, true),
    m(CODE_PRECURSOR_ION, "Prec", 2, Some(("MS:1000341", "precursor ion spectrum")), Spettro, true),
    m(CODE_PRODUCT_ION, "PI", 2, None, Spettro, true),
    m(CODE_NEUTRAL_LOSS, "NL", 2, Some(("MS:1000326", "constant neutral loss spectrum")), Spettro, true),
    m(8, "TOF MS", 1, None, Spettro, false),
    m(9, "TOF MS/MS", 2, None, Spettro, false),
    m(10, "?", 1, None, Spettro, false),
    m(CODE_PRODUCT, "EPI", 2, None, Spettro, true),
    m(12, "ER", 1, Some(("MS:1000791", "enhanced resolution spectrum")), Spettro, false),
    m(13, "MS3", 3, None, Spettro, false),
    m(14, "TDF", 2, Some(("MS:1000790", "time-delayed fragmentation spectrum")), Spettro, false),
    m(CODE_FULL, "EMS", 1, None, Spettro, true),
    m(16, "EMC", 1, Some(("MS:1000789", "enhanced multiply charged spectrum")), Spettro, false),
];

/// The row of `code`, `None` outside the table.
pub fn modo(code: u32) -> Option<&'static Modo> {
    MODI.get(usize::try_from(code).ok()?)
        .filter(|x| x.code == code)
}

/// The engine's own reader (3200 QTRAP) handles MRM, product-ion, full (EMS) and Q1 full scans only.
pub fn nativo(code: Option<u32>) -> bool {
    matches!(code, Some(CODE_MRM | CODE_PRODUCT | CODE_FULL | CODE_Q1))
}

/// The readers module accepts a code only when its row is confirmed.
pub fn confermato(code: Option<u32>) -> bool {
    code.and_then(modo).is_some_and(|x| x.confermato)
}

/// The Analyst abbreviation of an experiment's scan type (`None` for a code outside the table).
pub fn sigla(code: Option<u32>) -> Option<&'static str> {
    code.and_then(modo).map(|x| x.sigla)
}

/// The CV term of the spectrum type of an experiment: `None` for the types that need none (full scan, product-ion scan).
pub fn spectrum_term(code: Option<u32>) -> Option<(&'static str, &'static str)> {
    code.and_then(modo).and_then(|x| x.term)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_indexed_by_code() {
        assert!(MODI.iter().enumerate().all(|(i, x)| x.code as usize == i));
        assert!(modo(17).is_none());
        assert_eq!(sigla(Some(11)), Some("EPI"));
        assert_eq!(sigla(Some(15)), Some("EMS"));
        assert!(
            confermato(Some(5)) && confermato(Some(6)) && confermato(Some(0)) && !confermato(Some(1)) && !confermato(None)
        );
        assert!(nativo(Some(4)) && nativo(Some(0)) && !nativo(Some(5)));
        assert_eq!(sigla(Some(0)), Some("Q1"));
    }
}
