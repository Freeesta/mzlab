//! What the mzML says of a spectrum of a `.wiff` besides its points (vendor TIC, collision energy, isolation window, scan-type term): read from the sample
//! and experiment named in the spectrum id.

use super::reader::WiffReader;
use crate::mzml::Spectrum;

/// The userParam of an MS3 spectrum that holds the m/z of its first precursor (`Spectrum::precursor` is the second).
pub const PARENT_PARAM: &str = "ms3 first precursor m/z";

/// The number after `key` in a spectrum id (`sample=2 ... experiment=1`).
fn id_field(id: &str, key: &str) -> Option<usize> {
    id.split_whitespace()
        .find_map(|w| w.strip_prefix(key))
        .and_then(|v| v.parse::<usize>().ok())
}

/// The index record of the block of `s`, when the block is where the spectrum number says.
fn record_of<'a>(l: &'a WiffReader, s: &Spectrum) -> Option<&'a super::Record> {
    let p = l
        .parts
        .iter()
        .find(|p| Some(p.w.sample as usize) == id_field(&s.id, "sample="))?;
    let r = p.w.records.get(*p.scans.get(s.index)?)?;
    let same = id_field(&s.id, "cycle=") == Some(r.cycle + 1)
        && id_field(&s.id, "experiment=") == Some(r.experiment + 1);
    same.then_some(r)
}

/// What the mzML says of a spectrum besides its points: the collision energy and isolation window of its experiment, and (readers module) the spectrum
/// type (CV term) of precursor-ion and neutral-loss scans.
pub(super) fn extra_of(l: &WiffReader, s: &Spectrum) -> crate::mzmlwrite::SpectrumExtra {
    let e = id_field(&s.id, "sample=")
        .and_then(|n| l.parts.iter().find(|p| p.w.sample as usize == n))
        .zip(id_field(&s.id, "experiment="))
        .and_then(|(p, e)| p.w.experiments.get(e.wrapping_sub(1)));
    // the vendor TIC of the block in every build: the engine (no `estesi`) writes the same mzML and index as the CLI and the app
    let r = record_of(l, s);
    let mut extra = crate::mzmlwrite::SpectrumExtra {
        tic: r.map(|r| r.tic).filter(|t| t.is_finite() && *t > 0.0),
        charge: r.and_then(|r| r.charge),
        ..Default::default()
    };
    let Some(e) = e else { return extra };
    (extra.ce, extra.iso_half) = (e.ce, e.iso_half);
    if let Some(p) = e.parent {
        extra.params.push((PARENT_PARAM.into(), p.to_string()));
    }
    match (e.code, e.fixed) {
        (Some(super::modes::CODE_NEUTRAL_LOSS), Some(l)) => extra
            .params
            .push((super::modes::LOSS_PARAM.into(), l.to_string())),
        (_, f) => extra.product = f,
    }
    if let Some(sg) = super::modes::tag(e.code) {
        extra
            .params
            .push((super::modes::SCAN_TYPE_PARAM.into(), sg.into()));
    }
    #[cfg(feature = "estesi")]
    if let Some((acc, name)) = super::extended::spectrum_term(e.code) {
        extra.params.push((format!("cv:{acc}"), name.to_string()));
    }
    extra
}
