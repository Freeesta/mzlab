//! ANDI-Chromatography (ASTM E1947, netCDF-3): one detector trace (FID, UV…) as an analog chromatogram. The points are `ordinate_values`; their times are `raw_data_retention` (seconds) or,
//! for a uniform sampling, `actual_delay_time + i * actual_sampling_interval`.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{Limiti, VoceCromatogramma};
use mzlab_core::mzml::{ChromKind, Chromatogram};

use super::netcdf::Cdf;

/// A text the file keeps as a global attribute or as a `char` variable.
fn testo(cdf: &Cdf, dati: &mut dyn ReadAt, limiti: &Limiti, nome: &str) -> Option<String> {
    cdf.testo(nome).map(str::to_string).or_else(|| {
        let v = cdf.trova(nome)?;
        cdf.stringhe(dati, limiti, v)
            .ok()?
            .into_iter()
            .find(|s| !s.is_empty())
    })
}

/// A scalar variable (or global attribute) as a number.
fn scalare(cdf: &Cdf, dati: &mut dyn ReadAt, limiti: &Limiti, nome: &str) -> Option<f64> {
    cdf.numero(nome).or_else(|| {
        let v = cdf.trova(nome)?;
        cdf.leggi(dati, limiti, v, 0, cdf.elementi(v).min(1))
            .ok()?
            .first()
            .copied()
    })
}

fn id(cdf: &Cdf, dati: &mut dyn ReadAt, limiti: &Limiti) -> String {
    testo(cdf, dati, limiti, "detector_name").unwrap_or_else(|| "Detector".into())
}

fn unita(cdf: &Cdf, dati: &mut dyn ReadAt, limiti: &Limiti) -> String {
    let da_var = cdf
        .trova("ordinate_values")
        .and_then(|v| v.testo("units"))
        .map(str::to_string);
    da_var
        .filter(|u| !u.is_empty())
        .or_else(|| testo(cdf, dati, limiti, "detector_unit"))
        .unwrap_or_default()
}

/// The one chromatogram of the file.
pub(super) fn voci(cdf: &Cdf, dati: &mut dyn ReadAt, limiti: &Limiti) -> Vec<VoceCromatogramma> {
    vec![VoceCromatogramma {
        id: id(cdf, dati, limiti),
        tipo: ChromKind::Analog,
        unita: unita(cdf, dati, limiti),
    }]
}

/// The trace `nome`: times in minutes, the ordinates scaled by `scale_factor`/`add_offset` when the variable has them.
pub(super) fn traccia(
    cdf: &Cdf,
    dati: &mut dyn ReadAt,
    limiti: &Limiti,
    nome: &str,
) -> Result<Chromatogram> {
    let id = id(cdf, dati, limiti);
    let v = cdf
        .trova("ordinate_values")
        .filter(|_| nome == id)
        .ok_or_else(|| Error::new("error.badIndex").with("id", nome))?;
    let n = limiti.voci_ok(cdf.elementi(v))? as u64;
    let mut y = cdf.leggi(dati, limiti, v, 0, n)?;
    let (f, o) = (
        v.numero("scale_factor").unwrap_or(1.0),
        v.numero("add_offset").unwrap_or(0.0),
    );
    y.iter_mut().for_each(|x| *x = *x * f + o);
    let ritenzione = cdf
        .trova("raw_data_retention")
        .filter(|r| cdf.elementi(r) == n);
    let secondi: Vec<f64> = match ritenzione {
        Some(r) => cdf.leggi(dati, limiti, r, 0, n)?,
        None => {
            let (d, p) = (
                scalare(cdf, dati, limiti, "actual_delay_time").unwrap_or(0.0),
                scalare(cdf, dati, limiti, "actual_sampling_interval").unwrap_or(1.0),
            );
            (0..n).map(|i| d + i as f64 * p).collect()
        }
    };
    Ok(Chromatogram {
        index: 0,
        id,
        kind: ChromKind::Analog,
        q1: None,
        q3: None,
        unit: unita(cdf, dati, limiti),
        time: secondi.iter().map(|s| s / 60.0).collect(),
        intensity: y,
    })
}
