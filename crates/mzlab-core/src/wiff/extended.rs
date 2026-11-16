//! What the readers module (feature `estesi`) adds to the `.wiff` reader: the scan types of the other QTRAPs and the check that nothing
//! the reader does not know is read as a full scan. Known: Q1 full scan (0), MRM (4), precursor-ion scan (5), product-ion scan (6), neutral-loss scan (7), product-ion / EPI (11) and
//! survey / full scan (15). The TOF codes and any other code are `err.file.wiffScanType`, a method of several periods `err.file.wiffPeriods`, a sample kept in a .wiff2 `err.file.wiffNeeds2`.

use super::{bad, Experiment, Wiff, HEADER_PRECURSOR};

use crate::error::{Error, Result};
use crate::mzmlwrite::Header;
use crate::ole::Ole;

pub use super::modes::{
    spectrum_term, CODE_MS3, CODE_NEUTRAL_LOSS, CODE_PRECURSOR_ION, CODE_PRODUCT_ION,
};

/// Offset of the second precursor (f64) of an MS3 experiment in `ExperimentHeaderQTrapEX`.
const QTRAP_PRECURSOR2: usize = 64;

/// The positive finite f64 at `at` of `stream`.
fn f64_at(ole: &Ole, stream: &str, at: usize) -> Option<f64> {
    let b = ole.read(stream)?;
    let v = f64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?);
    (v.is_finite() && v > 0.0).then_some(v)
}

/// The detection window of each scheduled MRM experiment (`Experiment::window_s`).
pub(crate) fn mark_windows(ole: &Ole, sample: u32, experiments: &mut [super::Experiment]) {
    let no = method_no(ole, sample);
    for (i, e) in experiments.iter_mut().enumerate() {
        e.window_s = crate::method_wiff::scheduled(ole, no, i);
    }
}

pub(crate) use super::method_no;

/// The method of `sample` read from the method that stands for it (`method_no`), for the samples without a `MethodN` of their own.
pub(crate) fn fallback_method(ole: &Ole, sample: u32) -> Option<crate::methodinfo::Method> {
    crate::methodinfo::method_of_sample(ole, method_no(ole, sample))
}

/// The number of periods of the method of `sample` (the `PeriodN` storages of its first device method).
fn periods(ole: &Ole, sample: u32) -> usize {
    super::period_count(ole, method_no(ole, sample))
}

/// The experiments of `sample` checked: a method of several periods or a scan type this reader does not know is an error. The precursor-ion,
/// product-ion (6) and neutral-loss scans are MS2. The mass of the header is the selected ion of a product-ion scan and, as ProteoWizard writes it,
/// of a precursor-ion scan (the m/z Q3 stays on, in its `<precursorList>`) and of a neutral-loss scan, where it is the loss (also `fixed`).
pub(super) fn refine(ole: &Ole, sample: u32, mut ex: Vec<Experiment>) -> Result<Vec<Experiment>> {
    let m = method_no(ole, sample);
    if ex.is_empty() && m != sample {
        ex = super::experiments(ole, m);
    }
    let p = periods(ole, sample);
    if p > 1 {
        return Err(Error::new("err.file.wiffPeriods").with("periods", p));
    }
    let base = format!("MethodSubtree/Method{m}/DeviceMethod0/Period0/Experiment");
    for (i, e) in ex.iter_mut().enumerate() {
        match e.code {
            Some(c) if super::modes::nativo(Some(c)) => {}
            Some(c @ (CODE_PRECURSOR_ION | CODE_PRODUCT_ION | CODE_NEUTRAL_LOSS)) => {
                let header = ole.read(&format!("MethodSubtree/Method{m}/DeviceMethod0/Period0/Experiment{i}/ExperimentHeader")).unwrap_or_default();
                e.level = 2;
                let mass = header
                    .get(HEADER_PRECURSOR..HEADER_PRECURSOR + 8)
                    .map(|b| f64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
                    .filter(|m| m.is_finite() && *m > 0.0);
                if c == CODE_NEUTRAL_LOSS {
                    e.fixed = mass;
                }
                e.precursor = mass;
            }
            Some(CODE_MS3) => {
                let first = f64_at(
                    ole,
                    &format!("{base}{i}/ExperimentHeader"),
                    HEADER_PRECURSOR,
                );
                let second = f64_at(
                    ole,
                    &format!("{base}{i}/ExperimentHeaderQTrapEX"),
                    QTRAP_PRECURSOR2,
                );
                // a header that holds one mass only is read as an MS2 of it rather than refused
                (e.level, e.precursor, e.parent) = match (first, second) {
                    (Some(a), Some(b)) => (3, Some(b), Some(a)),
                    (a, b) => (2, a.or(b), None),
                };
            }
            Some(c) if !super::modes::confirmed(Some(c)) => {
                if crate::wiff::dad::wiff2_sample_info(ole, sample).is_some() {
                    return Err(Error::new("err.file.wiffNeeds2"));
                }
                return Err(super::scan_type_error(c, i + 1));
            }
            Some(_) => {}
            None => return Err(bad(&format!("no scan type code (experiment {})", i + 1))),
        }
    }
    Ok(ex)
}

impl Wiff {
    /// True when the engine's reader (written for the 3200 QTRAP: full scans, product-ion scans and dense MRM) would not read this sample as the file is:
    /// another instrument model, a scan type besides those three, or a scheduled MRM experiment (its sparse blocks; a file with no Log has no model to tell).
    pub fn variant(&self) -> bool {
        let other_model = self
            .model
            .as_deref()
            .is_some_and(|m| !m.replace(' ', "").contains("3200QTRAP"));
        other_model
            || self
                .experiments
                .iter()
                .any(|e| !super::modes::nativo(e.code) || e.window_s.is_some())
    }
}

/// The instrument of the mzML for the models the Log names, as `(accession, name)`; `m` is the `Component ID` without spaces and in lower case.
/// Every accession and name is a term of the PSI-MS CV v4.2.6 kept in `data/psims-cv.json`. Longer keys come first (`contains` takes the first hit).
/// A model with no term there (e.g. «QTRAP 7500») is not listed: its name goes out as a userParam (`instrument_terms`), never as a made-up accession.
fn instrument_cv(m: &str) -> Option<(&'static str, &'static str)> {
    const MODELS: [(&str, &str, &str); 19] = [
        ("qtrap6500+", "MS:1002582", "QTRAP 6500+"),
        ("qtrap6500", "MS:1002581", "QTRAP 6500"),
        ("qtrap5500", "MS:1000931", "QTRAP 5500"),
        ("qtrap4500", "MS:1002580", "QTRAP 4500"),
        ("4000qtrap", "MS:1000139", "4000 QTRAP"),
        ("api4000", "MS:1000147", "API 4000"),
        ("api5000", "MS:1000654", "API 5000"),
        ("api3200", "MS:1000653", "API 3200"),
        ("triplequad3500", "MS:1002591", "Triple Quad 3500"),
        ("triplequad4500", "MS:1002592", "Triple Quad 4500"),
        (
            "triplequad5500+qtrapready",
            "MS:1003458",
            "Triple Quad 5500+ QTRAP Ready",
        ),
        ("triplequad5500+", "MS:1003457", "Triple Quad 5500+"),
        ("triplequad5500", "MS:1002593", "Triple Quad 5500"),
        ("triplequad6500+", "MS:1002595", "Triple Quad 6500+"),
        ("triplequad6500", "MS:1002594", "Triple Quad 6500"),
        ("triplequad7500", "MS:1003144", "Triple Quad 7500"),
        ("qstarpulsar", "MS:1000656", "QSTAR Pulsar"),
        ("tripletof5600", "MS:1000932", "TripleTOF 5600"),
        ("tripletof6600", "MS:1002533", "TripleTOF 6600"),
    ];
    MODELS
        .iter()
        .find(|(k, _, _)| m.contains(k))
        .map(|&(_, a, n)| (a, n))
}

impl Wiff {
    /// The header of the mzML as the readers module writes it: the instrument CV term of the model when it is one of the known (the generic term and
    /// the model as a userParam otherwise) and the spectrum types the experiments produce in the file content.
    pub(crate) fn header_estesi(&self, name: &str, scan_sha1: Option<String>) -> Header {
        let mut h = self.header(name, scan_sha1, false);
        let key = self.model.as_deref().map(|m| {
            m.chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .to_lowercase()
        });
        if let Some((acc, term)) = key
            .as_deref()
            .filter(|k| !k.contains("3200qtrap"))
            .and_then(instrument_cv)
        {
            h.instrument = Some((acc.to_string(), term.to_string()));
            h.run_params.clear();
        }
        for e in &self.experiments {
            if let Some((acc, term)) = spectrum_term(e.code)
                && !h.content.iter().any(|(a, _)| a == acc)
            {
                h.content.push((acc.to_string(), term.to_string()));
            }
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::super::Wiff;
    use super::instrument_cv;
    use crate::error::Error;

    fn header_of(model: &str) -> crate::mzmlwrite::Header {
        let mut w = Wiff::refused(1, 0, Error::new("x"));
        w.model = Some(model.to_string());
        w.wiff_sha1 = Some("aa".into());
        w.header_estesi("a.wiff", None)
    }

    /// The header of a synthetic file of each model names the model: the CV term when the vocabulary has one, otherwise the generic term and the
    /// real name as a userParam (never the bare generic literal alone).
    #[test]
    fn the_header_names_the_real_model() {
        for (log, acc, name) in [
            ("Triple Quad 5500", "MS:1002593", "Triple Quad 5500"),
            ("Triple Quad 6500+", "MS:1002595", "Triple Quad 6500+"),
            ("API 5000", "MS:1000654", "API 5000"),
            ("API 3200", "MS:1000653", "API 3200"),
        ] {
            let h = header_of(log);
            assert_eq!(h.instrument, Some((acc.into(), name.into())), "{log}");
            assert!(h.run_params.is_empty(), "{log}");
        }
        let h = header_of("QTRAP 7500");
        assert_eq!(h.instrument.map(|i| i.0), Some("MS:1000031".to_string()));
        assert_eq!(
            h.run_params,
            [("instrument model".to_string(), "QTRAP 7500".to_string())]
        );
        // the 3200 QTRAP keeps its own term (the probe and the engine reader treat it apart)
        assert_eq!(
            header_of("3200 QTRAP").instrument.map(|i| i.0),
            Some("MS:1000651".to_string())
        );
    }

    /// Each model the Log can name gives the term of the PSI-MS CV (checked against the table kept in the repository, never a typed accession).
    #[test]
    fn every_model_maps_to_its_own_term_of_the_psi_ms_cv() {
        let cv: serde_json::Value =
            serde_json::from_str(include_str!("../../data/psims-cv.json")).unwrap();
        let name_of = |acc: &str| {
            cv["terms"]
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t[0] == acc && t[2] == 0)
                .map(|t| t[1].as_str().unwrap().to_string())
        };
        for (log, acc) in [
            ("Triple Quad 5500", "MS:1002593"),
            ("Triple Quad 5500+", "MS:1003457"),
            ("Triple Quad 5500+ QTRAP Ready", "MS:1003458"),
            ("Triple Quad 6500", "MS:1002594"),
            ("Triple Quad 6500+", "MS:1002595"),
            ("Triple Quad 7500", "MS:1003144"),
            ("Triple Quad 3500", "MS:1002591"),
            ("API 5000", "MS:1000654"),
            ("API 3200", "MS:1000653"),
            ("QTRAP 6500+", "MS:1002582"),
            ("QTRAP 5500", "MS:1000931"),
        ] {
            let key = log.replace(' ', "").to_lowercase();
            let (a, n) = instrument_cv(&key).unwrap_or_else(|| panic!("{log}: no term"));
            assert_eq!(a, acc, "{log}");
            assert_eq!(name_of(a).as_deref(), Some(n), "{log}: name of {a}");
        }
    }

    /// A model with no term in the CV (the QTRAP 7500) gets none: its name is written as a userParam, no accession is invented.
    #[test]
    fn a_model_without_a_term_is_not_given_one() {
        assert_eq!(instrument_cv("qtrap7500"), None);
        assert_eq!(instrument_cv("sciex7500+"), None);
    }
}
