//! The precursor chain of an mzXML scan as the mzML we write has it (`SpectrumExtra::precursors`); used by `mzxml`.

use mzlab_core::lettori::VoceScansione;
use mzlab_core::mzmlwrite::{attivazione_da_nome, Precursore};

use super::mzxml::numero;
use super::mzxml_scan::{attr, Testa};

/// Whether the file has the scan `id` (`scan=N`): the scans come in ascending number, so a bisection finds it.
fn nel_file(indice: &[VoceScansione], id: &str) -> bool {
    let n = |s: &str| s.strip_prefix("scan=").and_then(|x| x.parse::<u64>().ok());
    let Some(want) = n(id) else { return false };
    indice
        .binary_search_by_key(&want, |v| n(&v.id).unwrap_or(0))
        .is_ok()
}

/// The precursors of a scan as the mzML has them, outermost first: the m/z, charge, intensity, isolation width and activation of each `precursorMz`
/// and the collision energy of the scan (on the last one). The parent scan is named only if the file has it.
pub(super) fn precursori_di(t: &Testa, indice: &[VoceScansione]) -> Vec<Precursore> {
    let mut v: Vec<Precursore> = t
        .precursori
        .iter()
        .filter_map(|(g, mz)| {
            let mz: f64 = mz.trim().parse().ok().filter(|x: &f64| x.is_finite())?;
            let meta = numero(g, "windowWideness").map(|w| w / 2.0);
            let radice = format!("scan={}", attr(g, "precursorScanNum").trim());
            Some(Precursore {
                selected_mz: Some(mz),
                charge: numero(g, "precursorCharge")
                    .map(|z| z as i32)
                    .filter(|&z| z > 0),
                intensity: numero(g, "precursorIntensity"),
                iso_target: Some(mz),
                iso_lower: meta,
                iso_upper: meta,
                activation: attivazione_da_nome(attr(g, "activationMethod"))
                    .map(|(a, n)| vec![(a.to_string(), n.to_string())])
                    .unwrap_or_default(),
                energy: None,
                spectrum_ref: nel_file(indice, &radice).then_some(radice),
            })
        })
        .collect();
    if let (Some(p), Some(e)) = (v.last_mut(), numero(t.scan, "collisionEnergy")) {
        p.energy = Some(("MS:1000045".into(), e));
    }
    v
}
