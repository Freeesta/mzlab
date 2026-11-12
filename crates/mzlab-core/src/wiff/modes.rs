//! Table of the scan-type codes of `ExperimentHeader` (+122) of a `.wiff`: Analyst abbreviation, MS level, CV term of the spectrum and the output kind
//! of each. Read by `mod.rs`, `extended.rs` and `reader.rs`; the page shows the abbreviation as the exact type of an experiment. Codes 0, 4, 5, 6, 7, 11, 13, 15 are
//! seen in real files (0 is the Q1 MS full scan of a 3200 QTRAP: level-1 profile scans like EMS, its meaning inferred from the method name and the
//! spectra); the others follow the `ScanType` enum of Analyst and stay `confermato: false` until a file shows them.

/// Codes seen in real files.
pub const CODE_Q1: u32 = 0;
pub const CODE_MRM: u32 = 4;
pub const CODE_PRECURSOR_ION: u32 = 5;
pub const CODE_PRODUCT_ION: u32 = 6;
pub const CODE_NEUTRAL_LOSS: u32 = 7;
pub const CODE_PRODUCT: u32 = 11;
/// MS3 of a 3200 QTRAP (Analyst `MS3`): Q1 isolates the first precursor (`ExperimentHeader` +42), the second one (`ExperimentHeaderQTrapEX` +64) is isolated
/// from its fragments and trapped; the sample of the file seen is named `MS3_237-194` after that chain, which the two stored masses match.
pub const CODE_MS3: u32 = 13;
pub const CODE_FULL: u32 = 15;
/// Selected-ion monitoring on Q1 or Q3 (Q1MI, Q3MI): one value per ion per cycle, like an MRM block. Not yet seen in a real file.
pub const CODE_Q1MI: u32 = 1;
pub const CODE_Q3MI: u32 = 3;

/// What an experiment of this type writes into the mzML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Spettro,
    Srm,
    Sim,
}

#[derive(Debug, Clone, Copy)]
pub struct Modo {
    pub code: u32,
    /// Analyst abbreviation (EMS, EPI, MRM, MS3, NL, Prec, PI, Q1, Q3, Q1MI, Q3MI, EMC, ER, TDF).
    pub tag: &'static str,
    pub level: u8,
    /// CV `(accession, name)` of the spectrum type, when it needs one.
    pub term: Term,
    pub output: Output,
    /// Seen in a real file: only these are read by the readers module.
    pub confirmed: bool,
}

type Term = Option<(&'static str, &'static str)>;

#[rustfmt::skip]
const fn m(code: u32, sigla: &'static str, level: u8, term: Term, uscita: Output, confermato: bool) -> Modo {
    Modo { code, tag: sigla, level, term, output: uscita, confirmed: confermato }
}

use Output::{Sim, Spettro, Srm};

#[rustfmt::skip]
pub const MODI: [Modo; 17] = [
    m(CODE_Q1, "Q1", 1, None, Spettro, true),
    m(CODE_Q1MI, "Q1MI", 1, None, Sim, false),
    m(2, "Q3", 1, None, Spettro, false),
    m(CODE_Q3MI, "Q3MI", 1, None, Sim, false),
    m(CODE_MRM, "MRM", 2, None, Srm, true),
    m(CODE_PRECURSOR_ION, "Prec", 2, Some(("MS:1000341", "precursor ion spectrum")), Spettro, true),
    m(CODE_PRODUCT_ION, "PI", 2, None, Spettro, true),
    m(CODE_NEUTRAL_LOSS, "NL", 2, Some(("MS:1000326", "constant neutral loss spectrum")), Spettro, true),
    m(8, "TOF MS", 1, None, Spettro, false),
    m(9, "TOF MS/MS", 2, None, Spettro, false),
    m(10, "?", 1, None, Spettro, false),
    m(CODE_PRODUCT, "EPI", 2, None, Spettro, true),
    m(12, "ER", 1, Some(("MS:1000791", "enhanced resolution spectrum")), Spettro, false),
    m(CODE_MS3, "MS3", 3, None, Spettro, true),
    m(14, "TDF", 2, Some(("MS:1000790", "time-delayed fragmentation spectrum")), Spettro, false),
    m(CODE_FULL, "EMS", 1, None, Spettro, true),
    m(16, "EMC", 1, Some(("MS:1000789", "enhanced multiply charged spectrum")), Spettro, false),
];

/// The row of `code`, `None` outside the table.
pub fn mode(code: u32) -> Option<&'static Modo> {
    MODI.get(usize::try_from(code).ok()?)
        .filter(|x| x.code == code)
}

/// The engine's own reader (3200 QTRAP) handles MRM, SIM (Q1MI, Q3MI), product-ion, full (EMS) and Q1 full scans only.
pub fn nativo(code: Option<u32>) -> bool {
    matches!(
        code,
        Some(CODE_MRM | CODE_PRODUCT | CODE_FULL | CODE_Q1 | CODE_Q1MI | CODE_Q3MI)
    )
}

/// True for the SIM experiments (Q1MI, Q3MI): their blocks are one value per ion and become SIM chromatograms, not SRM ones.
pub fn sim(code: Option<u32>) -> bool {
    code.and_then(mode).is_some_and(|x| x.output == Output::Sim)
}

/// The readers module accepts a code only when its row is confirmed.
pub fn confirmed(code: Option<u32>) -> bool {
    code.and_then(mode).is_some_and(|x| x.confirmed)
}

/// The Analyst abbreviation of an experiment's scan type (`None` for a code outside the table).
pub fn tag(code: Option<u32>) -> Option<&'static str> {
    code.and_then(mode).map(|x| x.tag)
}

/// The userParam of every spectrum of a `.wiff` that names the Analyst abbreviation of its experiment: PSI-MS has no term for an EPI (product ions
/// trapped in Q3) nor for a product-ion scan of Q3, both written as `MSn spectrum`, so the scan table reads the type back from it (`scan_type_named`).
pub const SCAN_TYPE_PARAM: &str = "analyst scan type";
/// The userParam of a neutral-loss scan: the constant loss (Da) Q1 and Q3 are kept apart by, which is not an m/z of the spectrum.
pub const LOSS_PARAM: &str = "constant neutral loss (Da)";

/// The experiment of a scan (`EPI`, `PI`, `EMS`, `Q1`) from the userParams a reader gives it (`SpectrumExtra::params`).
pub(crate) fn scan_type_param(params: &[(String, String)]) -> Option<&'static str> {
    params
        .iter()
        .find(|(k, _)| k == SCAN_TYPE_PARAM)
        .and_then(|(_, v)| scan_type_named(v))
}

pub(crate) fn scan_type_named(v: &str) -> Option<&'static str> {
    use crate::idxread::{KINDS, SPECIAL};
    KINDS.iter().skip(SPECIAL).find(|k| **k == v).copied()
}

/// The CV term of the spectrum type of an experiment: `None` for the types that need none (full scan, product-ion scan).
pub fn spectrum_term(code: Option<u32>) -> Option<(&'static str, &'static str)> {
    code.and_then(mode).and_then(|x| x.term)
}

/// The polarity of the ion spray voltage (`IS`) of a method: 1, -1, or 0 with no method, no `IS` or a voltage of zero; the experiment header wins over it.
pub fn source_polarity(method: Option<&crate::methodinfo::Method>) -> i8 {
    method
        .and_then(|m| m.source.iter().find(|p| p.id == "IS"))
        .map_or(0, |p| {
            if p.value > 0.0 {
                1
            } else if p.value < 0.0 {
                -1
            } else {
                0
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methodinfo::{Method, Param};

    #[test]
    fn the_sign_of_the_ion_spray_voltage_is_the_polarity() {
        let with = |id: &str, value: f64| Method {
            source: vec![Param {
                id: id.to_string(),
                label: "",
                value,
                unit: "V",
            }],
            ..Method::default()
        };
        assert_eq!(source_polarity(Some(&with("IS", 5500.0))), 1);
        assert_eq!(source_polarity(Some(&with("IS", -4500.0))), -1);
        assert_eq!(source_polarity(Some(&with("IS", 0.0))), 0);
        assert_eq!(source_polarity(Some(&with("TEM", -4500.0))), 0);
        assert_eq!(source_polarity(None), 0);
    }

    #[test]
    fn table_is_indexed_by_code() {
        assert!(MODI.iter().enumerate().all(|(i, x)| x.code as usize == i));
        assert!(mode(17).is_none());
        assert_eq!(tag(Some(11)), Some("EPI"));
        assert_eq!(tag(Some(15)), Some("EMS"));
        assert!(
            confirmed(Some(5))
                && confirmed(Some(6))
                && confirmed(Some(0))
                && !confirmed(Some(1))
                && !confirmed(None)
        );
        assert!(nativo(Some(4)) && nativo(Some(0)) && !nativo(Some(5)));
        assert!(nativo(Some(1)) && sim(Some(1)) && sim(Some(3)) && !sim(Some(4)) && !sim(None));
        assert_eq!(tag(Some(0)), Some("Q1"));
    }
}
