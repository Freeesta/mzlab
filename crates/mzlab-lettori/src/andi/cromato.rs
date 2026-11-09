//! ANDI-Chromatography (ASTM E1947, netCDF-3): one detector trace (FID, UV…) as an analog chromatogram. The points are `ordinate_values`; their times are `raw_data_retention` (seconds) or,
//! for a uniform sampling, `actual_delay_time + i * actual_sampling_interval`.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{ChromatogramEntry, Limits};
use mzlab_core::mzml::{ChromKind, Chromatogram};

use super::netcdf::{scale, Cdf};

/// A text the file keeps as a global attribute or as a `char` variable.
fn text(cdf: &Cdf, data: &mut dyn ReadAt, limits: &Limits, name: &str) -> Option<String> {
    cdf.text(name).map(str::to_string).or_else(|| {
        let v = cdf.find(name)?;
        cdf.strings(data, limits, v)
            .ok()?
            .into_iter()
            .find(|s| !s.is_empty())
    })
}

/// A scalar variable (or global attribute) as a number.
fn scalar(cdf: &Cdf, data: &mut dyn ReadAt, limits: &Limits, name: &str) -> Option<f64> {
    cdf.number(name).or_else(|| {
        let v = cdf.find(name)?;
        cdf.read(data, limits, v, 0, cdf.elements(v).min(1))
            .ok()?
            .first()
            .copied()
    })
}

fn id(cdf: &Cdf, data: &mut dyn ReadAt, limits: &Limits) -> String {
    text(cdf, data, limits, "detector_name").unwrap_or_else(|| "Detector".into())
}

fn units(cdf: &Cdf, data: &mut dyn ReadAt, limits: &Limits) -> String {
    let da_var = cdf
        .find("ordinate_values")
        .and_then(|v| v.text("units"))
        .map(str::to_string);
    da_var
        .filter(|u| !u.is_empty())
        .or_else(|| text(cdf, data, limits, "detector_unit"))
        .unwrap_or_default()
}

/// The one chromatogram of the file.
pub(super) fn entries(cdf: &Cdf, data: &mut dyn ReadAt, limits: &Limits) -> Vec<ChromatogramEntry> {
    vec![ChromatogramEntry {
        id: id(cdf, data, limits),
        tipo: ChromKind::Analog,
        unita: units(cdf, data, limits),
    }]
}

/// The trace `nome`: times in minutes, the ordinates scaled by `scale_factor`/`add_offset` when the variable has them.
pub(super) fn trace(
    cdf: &Cdf,
    data: &mut dyn ReadAt,
    limits: &Limits,
    name: &str,
) -> Result<Chromatogram> {
    let id = id(cdf, data, limits);
    let v = cdf
        .find("ordinate_values")
        .filter(|_| name == id)
        .ok_or_else(|| Error::new("error.badIndex").with("id", name))?;
    let n = limits.ok_entries(cdf.elements(v))? as u64;
    let mut y = cdf.read(data, limits, v, 0, n)?;
    scale(&mut y, v);
    let retention = cdf
        .find("raw_data_retention")
        .filter(|r| cdf.elements(r) == n);
    let seconds: Vec<f64> = match retention {
        Some(r) => {
            let mut t = cdf.read(data, limits, r, 0, n)?;
            scale(&mut t, r);
            t
        }
        None => {
            let (d, p) = (
                scalar(cdf, data, limits, "actual_delay_time").unwrap_or(0.0),
                scalar(cdf, data, limits, "actual_sampling_interval").unwrap_or(1.0),
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
        unit: units(cdf, data, limits),
        time: seconds.iter().map(|s| s / 60.0).collect(),
        intensity: y,
    })
}
