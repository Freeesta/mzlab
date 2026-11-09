//! What the mzML says of a spectrum of a `.wiff` besides its points (vendor TIC, collision energy, isolation window, scan-type term): read from the sample
//! and experiment named in the spectrum id.

use super::lettore::WiffReader;
use crate::mzml::Spectrum;

/// The number after `key` in a spectrum id (`sample=2 ... experiment=1`).
fn id_field(id: &str, key: &str) -> Option<usize> {
    id.split_whitespace()
        .find_map(|w| w.strip_prefix(key))
        .and_then(|v| v.parse::<usize>().ok())
}

/// The TIC the index of the file holds for the block of `s`, when the block is where the spectrum number says and the value is usable.
fn header_tic(l: &WiffReader, s: &Spectrum) -> Option<f64> {
    let p = l
        .parts
        .iter()
        .find(|p| Some(p.w.sample as usize) == id_field(&s.id, "sample="))?;
    let r = p.w.records.get(*p.scans.get(s.index)?)?;
    let same = id_field(&s.id, "cycle=") == Some(r.cycle + 1)
        && id_field(&s.id, "experiment=") == Some(r.experiment + 1);
    (same && r.tic.is_finite() && r.tic > 0.0).then_some(r.tic)
}

/// What the mzML says of a spectrum besides its points: the collision energy and isolation window of its experiment, and (readers module) the spectrum
/// type (CV term) of precursor-ion and neutral-loss scans.
pub(super) fn extra_of(l: &WiffReader, s: &Spectrum) -> crate::mzmlwrite::SpectrumExtra {
    let e = id_field(&s.id, "sample=")
        .and_then(|n| l.parts.iter().find(|p| p.w.sample as usize == n))
        .zip(id_field(&s.id, "experiment="))
        .and_then(|(p, e)| p.w.experiments.get(e.wrapping_sub(1)));
    // the vendor TIC of the block in every build: the engine (no `estesi`) writes the same mzML and index as the CLI and the app
    let mut extra = crate::mzmlwrite::SpectrumExtra {
        tic: header_tic(l, s),
        ..Default::default()
    };
    let Some(e) = e else { return extra };
    (extra.ce, extra.iso_half) = (e.ce, e.iso_half);
    if let Some(sg) = super::modi::tag(e.code) {
        extra
            .params
            .push((super::modi::SCAN_TYPE_PARAM.into(), sg.into()));
    }
    #[cfg(feature = "estesi")]
    if let Some((acc, name)) = super::estesi::spectrum_term(e.code) {
        extra.params.push((format!("cv:{acc}"), name.to_string()));
    }
    extra
}
