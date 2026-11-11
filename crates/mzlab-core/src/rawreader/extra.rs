//! What the mzML of a `.raw` says of a scan besides its points: precursor chain, activation, the maker's totals, injection time, FAIMS and the trailer.

use opentfraw::extra::scan_extras_with;
use opentfraw::mzml::scan_metadata_with;
use opentfraw::{Activation, Analyzer, ExtraFields};

use super::{field, header_fields, raw_error, Field, RawReader, Stream};
use crate::error::Result;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{CvPar, Precursore, SpectrumExtra};
use opentfraw::RawFileReader;

/// The dissociation terms the Thermo activation maps to (as the converters of the maker's data write them); none for the ones PSI-MS has no term for.
fn activation_terms(a: Activation, analyzer: Option<Analyzer>) -> Vec<(String, String)> {
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

fn energy_term(nce: bool) -> String {
    if nce { "MS:1000138" } else { "MS:1000045" }.to_string()
}

/// The precursor of one reaction: its m/z is the target, the selected ion the step's own when known (else the target), its window, activation and
/// energy its own (absent when the file does not give them for this step).
fn reaction_precursor(
    r: &opentfraw::ReactionInfo,
    analyzer: Option<Analyzer>,
    spectrum_ref: Option<String>,
) -> Precursore {
    let half = r.isolation_width.map(|w| w / 2.0);
    Precursore {
        selected_mz: r.selected_mz.or(Some(r.precursor_mz)),
        charge: r.charge.filter(|&z| z > 0),
        intensity: None,
        iso_target: Some(r.precursor_mz),
        iso_lower: half,
        iso_upper: half,
        activation: r
            .activation
            .map(|a| activation_terms(a, analyzer))
            .unwrap_or_default(),
        energy: r.energy.map(|v| (energy_term(r.energy_is_nce), v)),
        spectrum_ref,
    }
}

/// The precursors of a scan. Several isolated together (MSX, multi-band): one per reaction, each with its own target, window and energy. An MSn
/// chain: one per step, the final step's first (its SPS ions when the trailer names them), the trailer's selected ion and charge on the first step
/// only. A single step: the trailer's selected ion (`fallback`: the spectrum's own when the trailer gives none).
fn precursors(
    p: &opentfraw::PrecursorInfo,
    fallback: Option<f64>,
    spectrum_ref: Option<String>,
) -> Vec<Precursore> {
    if p.reactions.iter().any(|r| r.multiple) {
        return p
            .reactions
            .iter()
            .map(|r| reaction_precursor(r, p.analyzer, spectrum_ref.clone()))
            .collect();
    }
    if let [earlier @ .., last] = p.reactions.as_slice()
        && !earlier.is_empty()
    {
        let mut out: Vec<Precursore> = if p.sps_mz.is_empty() {
            vec![reaction_precursor(last, p.analyzer, spectrum_ref)]
        } else {
            p.sps_mz
                .iter()
                .map(|&mz| Precursore {
                    selected_mz: Some(mz),
                    iso_target: Some(mz),
                    iso_lower: None,
                    iso_upper: None,
                    ..reaction_precursor(last, p.analyzer, spectrum_ref.clone())
                })
                .collect()
        };
        out.extend(
            earlier
                .iter()
                .rev()
                .map(|r| reaction_precursor(r, p.analyzer, None)),
        );
        return out;
    }
    let half = p.isolation_width.map(|w| w / 2.0);
    vec![Precursore {
        selected_mz: p.selected_mz.or(p.target_mz).or(fallback),
        charge: p.charge.filter(|&z| z > 0),
        intensity: None,
        iso_target: p.target_mz,
        iso_lower: half,
        iso_upper: half,
        activation: p
            .activation
            .map(|a| activation_terms(a, p.analyzer))
            .unwrap_or_default(),
        energy: p.collision_energy.map(|v| (energy_term(p.ce_is_nce), v)),
        spectrum_ref,
    }]
}

/// `Lettore::scan_extra` of the `.raw`: nothing when the sample cannot be opened, the plain precursor of the spectrum when the scan cannot be read.
pub(super) fn di_scansione(l: &mut RawReader, c: usize, k: usize, s: &Spectrum) -> SpectrumExtra {
    if l.one(c).is_err() {
        return SpectrumExtra::default();
    }
    extra_di_scansione(l, k, s).unwrap_or_default()
}

/// What the mzML says of scan `k` besides its points. The parent of an MSn scan is the last scan of the level above that came before it (the "Master
/// Index" of the trailer is not a scan number). `None` when the scan cannot be read: the writer then falls back to the plain precursor of the spectrum.
fn extra_di_scansione(l: &mut RawReader, k: usize, s: &Spectrum) -> Option<SpectrumExtra> {
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
        e.precursors = precursors(p, s.precursor, parent.map(|v| v.id.clone()));
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
pub(super) fn noise_fields(raw: &RawFileReader, src: &mut Stream, idx: u32) -> Vec<Field> {
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

/// `Lettore::scan_header` of the `.raw`: the header fields of scan `k` and its noise nodes.
pub(super) fn scan_header(l: &mut RawReader, c: usize, k: usize) -> Result<Vec<Field>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use opentfraw::{PrecursorInfo, ReactionInfo};

    fn rx(mz: f64, energy: Option<f64>, width: Option<f64>, multiple: bool) -> ReactionInfo {
        ReactionInfo {
            precursor_mz: mz,
            isolation_width: width,
            activation: Some(Activation::HCD),
            energy,
            multiple,
            ..Default::default()
        }
    }

    #[test]
    fn precursors_isolated_together_are_one_each_with_their_own_window_and_energy() {
        let p = PrecursorInfo {
            target_mz: Some(700.0),
            selected_mz: Some(700.0),
            isolation_width: Some(2.0),
            collision_energy: Some(30.0),
            reactions: vec![
                rx(400.2, Some(25.0), Some(2.0), true),
                rx(550.3, Some(28.0), Some(2.0), true),
                rx(700.0, Some(30.0), Some(2.0), true),
            ],
            ..Default::default()
        };
        let v = precursors(&p, None, Some("scan=1".into()));
        assert_eq!(v.len(), 3);
        let mz: Vec<_> = v.iter().map(|q| q.selected_mz).collect();
        assert_eq!(mz, [Some(400.2), Some(550.3), Some(700.0)]);
        assert_eq!(v[0].iso_target, Some(400.2));
        // each window around its own target, not one window around the last
        assert!(v.iter().all(|q| q.iso_lower == Some(1.0)));
        assert_eq!(v[1].energy.as_ref().map(|e| e.1), Some(28.0));
        assert!(v
            .iter()
            .all(|q| q.spectrum_ref.as_deref() == Some("scan=1")));
    }

    #[test]
    fn an_msn_chain_gives_one_precursor_per_step_with_the_trailer_ion_on_the_first() {
        let mut first = rx(800.0, Some(35.0), None, false);
        first.selected_mz = Some(799.9);
        first.charge = Some(2);
        let p = PrecursorInfo {
            target_mz: Some(500.0),
            selected_mz: Some(500.0),
            isolation_width: Some(1.0),
            reactions: vec![first, rx(500.0, Some(27.0), Some(1.0), false)],
            ..Default::default()
        };
        let v = precursors(&p, None, Some("scan=2".into()));
        assert_eq!(v.len(), 2);
        // the final step first, with its own energy and window and no charge of another step
        assert_eq!((v[0].selected_mz, v[0].charge), (Some(500.0), None));
        assert_eq!(v[0].energy.as_ref().map(|e| e.1), Some(27.0));
        assert_eq!(v[0].iso_lower, Some(0.5));
        assert_eq!(v[0].spectrum_ref.as_deref(), Some("scan=2"));
        assert_eq!((v[1].selected_mz, v[1].charge), (Some(799.9), Some(2)));
        assert_eq!(v[1].iso_target, Some(800.0));
        assert_eq!(v[1].energy.as_ref().map(|e| e.1), Some(35.0));
        assert!(v[1].spectrum_ref.is_none());
    }

    #[test]
    fn the_sps_ions_are_the_final_step_of_an_ms3() {
        let p = PrecursorInfo {
            target_mz: Some(677.3),
            reactions: vec![
                rx(599.8, Some(27.0), None, false),
                rx(677.3, Some(33.0), Some(2.0), false),
            ],
            sps_mz: vec![410.2, 520.3, 633.4],
            ..Default::default()
        };
        let v = precursors(&p, None, None);
        let mz: Vec<_> = v.iter().map(|q| q.iso_target).collect();
        assert_eq!(mz, [Some(410.2), Some(520.3), Some(633.4), Some(599.8)]);
        // a notch has no window the file gives
        assert!(v[..3].iter().all(|q| q.iso_lower.is_none()));
        assert!(v[..3]
            .iter()
            .all(|q| q.energy.as_ref().map(|e| e.1) == Some(33.0)));
    }

    #[test]
    fn a_single_step_is_one_precursor_and_an_unknown_activation_has_no_term() {
        let p = PrecursorInfo {
            target_mz: Some(500.0),
            isolation_width: Some(1.0),
            reactions: vec![rx(500.0, None, Some(1.0), false)],
            ..Default::default()
        };
        let v = precursors(&p, Some(499.0), None);
        assert_eq!(v.len(), 1);
        // no selected ion in the trailer: the target, not the spectrum's own guess
        assert_eq!(v[0].selected_mz, Some(500.0));
        assert!(v[0].activation.is_empty());
        assert!(v[0].energy.is_none());
    }
}
