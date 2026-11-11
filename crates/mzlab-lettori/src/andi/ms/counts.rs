//! The scans of an ANDI-MS file checked against a second source: `scan_index` and `point_count` are never negative or a fill, scans with points never share one
//! and lie inside `mass_values`. A fill value (`_FillValue`, the netCDF default of the type, or the -9999 of the AIA templates) is no value, never data.

use super::super::netcdf::{broken, scale, Cdf, Kind, Var};
use super::AndiReader;
use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::readers::Limits;

/// What the AIA templates write for a value not measured.
const AIA_MISSING: f64 = -9999.0;

/// `x`, as stored (before `scale_factor`), says "no value".
pub(super) fn is_fill(v: &Var, x: f64) -> bool {
    let default = match v.tipo {
        Kind::Byte => -127.0,
        Kind::Short => -32_767.0,
        Kind::Int => -2_147_483_647.0,
        Kind::Float => f64::from(9.969_21e36_f32),
        Kind::Double => 9.969_209_968_386_869e36,
        Kind::Char => f64::NAN,
    };
    x == default || x == AIA_MISSING || v.number("_FillValue").is_some_and(|f| x == f)
}

/// A whole variable of the scans as numbers, `scale_factor`/`add_offset` applied and fills as NaN (`None`: the file has no such variable).
pub(super) fn column(
    cdf: &Cdf,
    data: &mut dyn ReadAt,
    limits: &Limits,
    name: &str,
) -> Result<Option<Vec<f64>>> {
    let Some(v) = cdf.find(name) else {
        return Ok(None);
    };
    let mut x = cdf.read(data, limits, v, 0, cdf.elements(v))?;
    x.iter_mut()
        .filter(|y| is_fill(v, **y))
        .for_each(|y| *y = f64::NAN);
    scale(&mut x, v);
    Ok(Some(x))
}

/// A first point or a point count as a number of points: a negative, missing or fractional one is damage.
fn points(x: f64, what: &str) -> Result<u64> {
    if x.is_nan() || x < 0.0 || x.fract() != 0.0 {
        return Err(broken(&format!("{what} is negative, missing or not whole")));
    }
    Ok(x as u64)
}

impl AndiReader {
    /// First point and number of points of each scan: `scan_index` and `point_count` (or the next first point).
    pub(super) fn points_per_scan(&mut self) -> Result<usize> {
        let (cdf, limits) = (&self.cdf, self.limits);
        let sx = cdf
            .find("scan_index")
            .ok_or_else(|| broken("no scan_index"))?;
        let n = cdf.count(sx, &limits)?;
        let all = cdf.find("mass_values").map_or(0, |v| cdf.elements(v));
        let starts = cdf
            .read(&mut self.data, &limits, sx, 0, n as u64)?
            .iter()
            .map(|&x| points(if is_fill(sx, x) { f64::NAN } else { x }, "scan_index"))
            .collect::<Result<Vec<u64>>>()?;
        self.counts = match column(cdf, &mut self.data, &limits, "point_count")? {
            Some(c) if c.len() == n => c
                .iter()
                .map(|&x| points(x, "point_count"))
                .collect::<Result<_>>()?,
            Some(_) => return Err(broken("point_count and scan_index differ in length")),
            None => (0..n)
                .map(|k| {
                    let next = starts.get(k + 1).copied().unwrap_or(all);
                    next.saturating_sub(starts[k])
                })
                .collect(),
        };
        // a cut copy whose `numrecs` was counted from its bytes has scans past the points: the reader counts them as missing
        let mut end = 0u64;
        for (&a, &c) in starts.iter().zip(&self.counts).filter(|(_, c)| **c > 0) {
            if a < end {
                return Err(broken("two scans share points"));
            }
            end = a.saturating_add(c);
        }
        if end > all && !self.cdf.streaming {
            return Err(broken("a scan runs past the points of the file"));
        }
        self.inizi = starts;
        Ok(n)
    }
}

/// The highest m/z an instrument gives is far below this: m/z read with the wrong type or offset are not.
const MZ_MAX: f64 = 1e6;

/// The m/z of a scan are finite and between 0 and `MZ_MAX`.
pub(super) fn plausible(mz: &[f64]) -> Result<()> {
    if mz.iter().any(|&m| !(0.0..=MZ_MAX).contains(&m)) {
        return Err(broken("an m/z is not a number from 0 to 10^6"));
    }
    Ok(())
}

/// How far apart the TIC of the file and the sum of the points of a scan may be: a TIC taken before centroiding differs by a small factor, intensities read with the wrong type by millions.
const TIC_FACTOR: f64 = 1e3;

/// The intensities of a scan agree with its TIC (`NaN`: the file has none for it).
pub(super) fn agrees_with_tic(it: &[f64], tic: f64) -> Result<()> {
    let sum: f64 = it.iter().filter(|x| x.is_finite()).sum();
    if tic > 0.0 && sum > 0.0 && (sum / tic > TIC_FACTOR || tic / sum > TIC_FACTOR) {
        return Err(broken(
            "the points of a scan disagree with its total_intensity",
        ));
    }
    Ok(())
}
