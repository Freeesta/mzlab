//! What the readers module (feature `estesi`) adds to the `.wiff` reader: the scan types of the other QTRAPs and the check that nothing
//! the reader does not know is read as a full scan. Known: MRM (4), precursor-ion scan (5), neutral-loss scan (7), product-ion / EPI (11) and
//! survey / full scan (15). The TOF codes, any other code and a method with more than one period are `err.file.wiff` with the cause in `detail`.

use super::{bad, Experiment, Wiff, HEADER_PRECURSOR};

/// Offset of the polarity in `ExperimentHeader` (u32: 0 positive, 1 negative).
const HEADER_POLARITY: usize = 86;
use crate::error::Result;
use crate::mzmlwrite::Header;
use crate::ole::Ole;

pub const CODE_MRM: u32 = 4;
pub const CODE_PRECURSOR_ION: u32 = 5;
pub const CODE_NEUTRAL_LOSS: u32 = 7;
pub const CODE_PRODUCT: u32 = 11;
pub const CODE_FULL: u32 = 15;

/// The method that holds the experiments of `sample`: its own `MethodN`, or, when the file stores fewer methods than it has samples (a batch run with one
/// method), the nearest one before it.
pub(crate) fn method_no(ole: &Ole, sample: u32) -> u32 {
    let has = |k: u32| {
        ole.has(&format!(
            "MethodSubtree/Method{k}/DeviceMethod0/Period0/Experiment0/ExperimentHeader"
        ))
    };
    (1..=sample.min(10_000))
        .rev()
        .find(|&k| has(k))
        .unwrap_or(sample)
}

/// The method of `sample` read from the method that stands for it (`method_no`), for the samples without a `MethodN` of their own.
pub(crate) fn fallback_method(ole: &Ole, sample: u32) -> Option<crate::methodinfo::Method> {
    crate::methodinfo::method_of_sample(ole, method_no(ole, sample))
}

/// The number of periods of the method of `sample` (the `PeriodN` storages of its first device method).
fn periods(ole: &Ole, sample: u32) -> usize {
    let prefix = format!(
        "MethodSubtree/Method{}/DeviceMethod0/Period",
        method_no(ole, sample)
    );
    let mut n = 0;
    for p in ole.paths_list() {
        if let Some(rest) = p.strip_prefix(&prefix) {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(k) = digits.parse::<usize>() {
                n = n.max(k + 1);
            }
        }
    }
    n
}

/// The experiments of `sample` checked: a method of several periods or a scan type this reader does not know is an error; the precursor-ion and
/// neutral-loss scans are MS2 whose selected ion is the fixed mass of the header (as ProteoWizard writes them).
pub(super) fn refine(ole: &Ole, sample: u32, mut ex: Vec<Experiment>) -> Result<Vec<Experiment>> {
    let m = method_no(ole, sample);
    if ex.is_empty() && m != sample {
        ex = super::experiments(ole, m);
    }
    let p = periods(ole, sample);
    if p > 1 {
        return Err(bad(&format!("periods={p}")));
    }
    for (i, e) in ex.iter_mut().enumerate() {
        e.polarity = ole
            .read(&format!(
                "MethodSubtree/Method{m}/DeviceMethod0/Period0/Experiment{i}/ExperimentHeader"
            ))
            .and_then(|h| {
                h.get(HEADER_POLARITY..HEADER_POLARITY + 4)
                    .and_then(|b| <[u8; 4]>::try_from(b).ok())
                    .map(u32::from_le_bytes)
            })
            .and_then(|v| match v {
                0 => Some(1),
                1 => Some(-1),
                _ => None,
            });
        match e.code {
            Some(CODE_MRM | CODE_FULL | CODE_PRODUCT) => {}
            Some(CODE_PRECURSOR_ION | CODE_NEUTRAL_LOSS) => {
                let header = ole.read(&format!("MethodSubtree/Method{m}/DeviceMethod0/Period0/Experiment{i}/ExperimentHeader")).unwrap_or_default();
                e.level = 2;
                e.precursor = header
                    .get(HEADER_PRECURSOR..HEADER_PRECURSOR + 8)
                    .map(|b| f64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
                    .filter(|m| m.is_finite() && *m > 0.0);
            }
            Some(c) => return Err(bad(&format!("scan type code {c} (experiment {})", i + 1))),
            None => return Err(bad(&format!("no scan type code (experiment {})", i + 1))),
        }
    }
    Ok(ex)
}

impl Wiff {
    /// The polarity of the scans of experiment `e`: the one of its header, else the sign of the ion spray voltage of the method.
    pub(super) fn polarity_of(&self, e: &Experiment) -> i8 {
        e.polarity.unwrap_or(self.polarity)
    }

    /// True when the engine's reader (written for the 3200 QTRAP: full scans, product-ion scans and MRM) would not read this sample as the file is:
    /// another instrument model, or a scan type besides those three.
    pub fn variante(&self) -> bool {
        let other_model = self
            .model
            .as_deref()
            .is_some_and(|m| !m.replace(' ', "").contains("3200QTRAP"));
        other_model
            || self
                .experiments
                .iter()
                .any(|e| !matches!(e.code, Some(CODE_MRM | CODE_FULL | CODE_PRODUCT)))
    }
}

/// The CV term of the spectrum type of an experiment: `None` for the types that need none (full scan, product-ion scan).
pub fn spectrum_term(code: Option<u32>) -> Option<(&'static str, &'static str)> {
    match code {
        Some(CODE_PRECURSOR_ION) => Some(("MS:1000341", "precursor ion spectrum")),
        Some(CODE_NEUTRAL_LOSS) => Some(("MS:1000326", "constant neutral loss spectrum")),
        _ => None,
    }
}

/// The instrument of the mzML for the models the Log names, as `(accession, name)`; `m` is the `Component ID` without spaces and in lower case.
fn instrument_cv(m: &str) -> Option<(&'static str, &'static str)> {
    const MODELS: [(&str, &str, &str); 10] = [
        ("qtrap6500+", "MS:1002582", "QTRAP 6500+"),
        ("qtrap6500", "MS:1002581", "QTRAP 6500"),
        ("qtrap5500", "MS:1000931", "QTRAP 5500"),
        ("qtrap4500", "MS:1002580", "QTRAP 4500"),
        ("4000qtrap", "MS:1000139", "4000 QTRAP"),
        ("api4000", "MS:1000147", "API 4000"),
        ("triplequad4500", "MS:1002592", "Triple Quad 4500"),
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
            if let Some((acc, term)) = spectrum_term(e.code) {
                if !h.content.iter().any(|(a, _)| a == acc) {
                    h.content.push((acc.to_string(), term.to_string()));
                }
            }
        }
        h
    }
}
