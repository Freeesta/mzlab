//! What the mzML of a `.raw` says of a scan besides its points: precursor chain, activation, the maker's totals, injection time, FAIMS and the trailer.

use opentfraw::extra::scan_extras_with;
use opentfraw::mzml::scan_metadata_with;
use opentfraw::{Activation, Analyzer, ExtraFields};

use super::{field, header_fields, raw_error, Campo, LettoreRaw, Stream};
use crate::error::Result;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{CvPar, Precursore, SpectrumExtra};
use opentfraw::RawFileReader;

/// The dissociation terms the Thermo activation maps to (as the converters of the maker's data write them); none for the ones PSI-MS has no term for.
fn termini_attivazione(a: Activation, analyzer: Option<Analyzer>) -> Vec<(String, String)> {
    const BEAM: (&str, &str) = ("MS:1000422", "beam-type collision-induced dissociation");
    const ETD: (&str, &str) = ("MS:1000598", "electron transfer dissociation");
    let t: &[(&str, &str)] = match a {
        Activation::HCD => &[BEAM],
        Activation::ETD => &[ETD],
        Activation::EThcD => &[
            ETD,
            (
                "MS:1002631",
                "electron-transfer/higher-energy collision dissociation",
            ),
        ],
        Activation::CID if analyzer == Some(Analyzer::FTMS) => &[BEAM],
        Activation::CID => &[("MS:1000133", "collision-induced dissociation")],
        Activation::MPID => &[("MS:1000435", "photodissociation")],
        Activation::ECD => &[("MS:1000250", "electron capture dissociation")],
        Activation::IRMPD => &[("MS:1000262", "infrared multiphoton dissociation")],
        Activation::PQD => &[("MS:1000599", "pulsed q dissociation")],
        Activation::UVPD => &[("MS:1003246", "ultraviolet photodissociation")],
        Activation::SID => &[("MS:1000136", "surface-induced dissociation")],
        Activation::PD => &[],
    };
    t.iter()
        .map(|(a, n)| (a.to_string(), n.to_string()))
        .collect()
}

/// `Lettore::extra_scansione` of the `.raw`: nothing when the sample cannot be opened, the plain precursor of the spectrum when the scan cannot be read.
pub(super) fn di_scansione(l: &mut LettoreRaw, c: usize, k: usize, s: &Spectrum) -> SpectrumExtra {
    if l.one(c).is_err() {
        return SpectrumExtra::default();
    }
    extra_di_scansione(l, k, s).unwrap_or_default()
}

/// What the mzML says of scan `k` besides its points. The parent of an MSn scan is the last scan of the level above that came before it (the "Master
/// Index" of the trailer is not a scan number). `None` when the scan cannot be read: the writer then falls back to the plain precursor of the spectrum.
fn extra_di_scansione(l: &mut LettoreRaw, k: usize, s: &Spectrum) -> Option<SpectrumExtra> {
    let idx = l.row(k).ok()?;
    let first = l.raw.run_header.sample_info.first_scan_number;
    let params = l.raw.scan_params(&mut l.src, first + idx);
    let m = scan_metadata_with(&l.raw, idx, params.as_ref().map(|p| p.borrowed()))?;
    let mut e = SpectrumExtra {
        tic: Some(m.total_ion_current),
        base_peak: Some((m.base_peak_mz, m.base_peak_intensity)),
        window: (m.high_mz > m.low_mz).then_some((m.low_mz, m.high_mz)),
        ..SpectrumExtra::default()
    };
    if let Some(v) = m.ion_injection_time_ms {
        e.scan_params.push(
            CvPar::new("MS:1000927", "ion injection time", v)
                .with_unit("UO:0000028", "millisecond"),
        );
    }
    if let Some(v) = m.faims_cv {
        e.scan_params.push(
            CvPar::new("MS:1001581", "FAIMS compensation voltage", v)
                .with_unit("UO:0000218", "volt"),
        );
    }
    if let Some(p) = &m.precursor {
        let level = l.indice.get(k)?.livello;
        let parent = (level >= 2)
            .then(|| l.indice[..k].iter().rev().find(|v| v.livello + 1 == level))
            .flatten();
        let half = p.isolation_width.map(|w| w / 2.0);
        e.precursors.push(Precursore {
            selected_mz: p.selected_mz.or(p.target_mz).or(s.precursor),
            charge: p.charge.filter(|&z| z > 0),
            intensity: None,
            iso_target: p.target_mz,
            iso_lower: half,
            iso_upper: half,
            activation: p
                .activation
                .map(|a| termini_attivazione(a, p.analyzer))
                .unwrap_or_default(),
            energy: p.collision_energy.map(|v| {
                (
                    if p.ce_is_nce {
                        "MS:1000138"
                    } else {
                        "MS:1000045"
                    }
                    .to_string(),
                    v,
                )
            }),
            spectrum_ref: parent.map(|v| v.id.clone()),
        });
    }
    e.params = scan_extras_with(&l.raw, params, &m, &ExtraFields::All)
        .into_iter()
        .collect();
    Some(e)
}

fn median(mut v: Vec<f32>) -> f32 {
    // hot path: per-point/per-peak sort, std is ~2x faster than sortc (docs/studi/OTTIMIZZAZIONE_CORE.md §4.1)
    #[allow(clippy::disallowed_methods)]
    v.sort_by(f32::total_cmp);
    v[v.len() / 2]
}

/// The noise and the baseline an FT scan carries (a piecewise-linear function of m/z, a few dozen nodes): `thermo:noise_function` holds every node as
/// `m/z,noise,baseline` joined by `;` (nothing is lost: the value at a peak is the interpolation), `thermo:noise` and `thermo:baseline` the median of the nodes
/// for a quick read. A scan without them (ion trap, SRM) gives nothing. The peaks are not touched.
pub(super) fn noise_fields(raw: &RawFileReader, src: &mut Stream, idx: u32) -> Vec<Campo> {
    let scan = raw.run_header.sample_info.first_scan_number + idx;
    let Ok(p) = raw.read_scan_labels(src, scan) else {
        return Vec::new();
    };
    let nodes = &p.noise_nodes;
    if nodes.is_empty() {
        return Vec::new();
    }
    let function = nodes
        .iter()
        .map(|n| format!("{},{},{}", n.mz, n.noise, n.baseline))
        .collect::<Vec<_>>()
        .join(";");
    vec![
        field(
            "thermo:noise",
            median(nodes.iter().map(|n| n.noise).collect()),
            None,
            None,
        ),
        field(
            "thermo:baseline",
            median(nodes.iter().map(|n| n.baseline).collect()),
            None,
            None,
        ),
        field("thermo:noise_function", function, None, None),
    ]
}

/// `Lettore::intestazione` of the `.raw`: the header fields of scan `k` and its noise nodes.
pub(super) fn intestazione(l: &mut LettoreRaw, c: usize, k: usize) -> Result<Vec<Campo>> {
    l.one(c)?;
    let idx = l.row(k)?;
    let first = l.raw.run_header.sample_info.first_scan_number;
    let params = l.raw.scan_params(&mut l.src, first + idx);
    let m = scan_metadata_with(&l.raw, idx, params.as_ref().map(|p| p.borrowed()))
        .ok_or_else(|| raw_error("scan metadata"))?;
    let mut out = header_fields(&l.raw, params, &m);
    out.extend(noise_fields(&l.raw, &mut l.src, k as u32));
    Ok(out)
}
