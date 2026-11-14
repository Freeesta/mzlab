//! The isotope grouping `finder` of `features.detect`: the isotope finder of mzmine 4.10 (`isofinder`) on the apex MS1 spectrum of each feature,
//! most intense first. The rows whose trace covers that apex scan and whose m/z matches a peak of the winning pattern join the feature's group.
//! The output has the shape of the other groupings: `iso_group` = position + 1 of the feature the pattern was found on, `charge` of the pattern,
//! `FLAG_ISOTOPE` on every member but that feature; no row is removed.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};

use super::isofinder::select::detect;
use super::isofinder::{Config, ElementMode, Engine, Tol};
use crate::align_mzmine::IsoFinder;
use crate::sortc::SortC;
use crate::table::{FileFeatures, FLAG_ISOTOPE};
use crate::trace_base::filled;

/// Half width, m/z, of the part of the apex spectrum kept around a feature; wider than any pattern the finder walks.
const WINDOW: f64 = 12.0;

/// One spectrum as (m/z, intensity) columns.
type Spectrum = (Vec<f64>, Vec<f64>);

/// The `Config` of the finder.
pub fn config(s: &IsoFinder) -> Config {
    Config {
        elements: elements(&s.elements),
        max_charge: i32::from(s.max_charge),
        tol: Tol {
            abs: s.tol_da,
            ppm: s.tol_ppm,
        },
        require_c13: s.require_c13,
        mode: if s.auto {
            ElementMode::UserPlusAuto
        } else {
            ElementMode::UserDefined
        },
        ..Config::default()
    }
}

/// The symbols of a comma-separated list, blanks removed.
pub fn elements(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(String::from)
        .collect()
}

/// The apex spectra of the features, one per distinct apex scan, each kept within `WINDOW` of a feature at that scan; ascending m/z.
fn apex_spectra<R: ReadAt>(
    index: &mut Index<R>,
    si: usize,
    f: &FileFeatures,
) -> Result<(Vec<u32>, Vec<Spectrum>)> {
    let n_scans = index.sections[si].n_scans;
    let mut slot = filled(n_scans, u32::MAX)?;
    let mut near: Vec<Vec<f64>> = Vec::new();
    for i in 0..f.len() {
        let a = f.apex[i] as usize;
        if a >= n_scans {
            continue;
        }
        if slot[a] == u32::MAX {
            slot[a] = near.len() as u32;
            near.push(Vec::new());
        }
        near[slot[a] as usize].push(f.mz[i]);
    }
    near.iter_mut().for_each(|v| v.sortc_by(f64::total_cmp));
    let mut spec = vec![(Vec::new(), Vec::new()); near.len()];
    let n = index.sections[si].n_peaks;
    index.each_peak(si, 0, n, |pos, mz, y| {
        let Some(&k) = slot.get(pos).filter(|&&k| k != u32::MAX) else {
            return;
        };
        let at = &near[k as usize];
        let j = at.partition_point(|&x| x < mz - WINDOW);
        if j < at.len() && at[j] <= mz + WINDOW {
            spec[k as usize].0.push(mz);
            spec[k as usize].1.push(y);
        }
    })?;
    Ok((slot, spec))
}

/// Groups the features of `f` with the isotope finder of mzmine on the MS1 spectra of `index` (`polarity` as in `trace::Params`).
pub fn group<R: ReadAt>(
    index: &mut Index<R>,
    f: &mut FileFeatures,
    polarity: Option<i8>,
    s: &IsoFinder,
) -> Result<()> {
    let n = f.len();
    let mut gid = filled(n, 0u32)?;
    let mut charge = filled(n, 0i8)?;
    for flag in &mut f.flags {
        *flag &= !FLAG_ISOTOPE;
    }
    if let Some(si) = index.section_of(1, polarity).filter(|_| n > 0) {
        let cfg = config(s);
        let tol = cfg.tol;
        let engine = Engine::new(cfg).map_err(|_| Error::new("error.badRecipe"))?;
        let (slot, spec) = apex_spectra(index, si, f)?;
        let sign = polarity.map_or(0, |p| i32::from(p.signum()));
        let mut order: Vec<usize> = (0..n).collect();
        order.sortc_by(|&a, &b| f.height[b].total_cmp(&f.height[a]).then(a.cmp(&b)));
        let mut by_mz: Vec<usize> = (0..n).collect();
        by_mz.sortc_by(|&a, &b| f.mz[a].total_cmp(&f.mz[b]).then(a.cmp(&b)));
        let mut taken = filled(n, false)?;
        for &i in &order {
            let Some(&k) = slot.get(f.apex[i] as usize).filter(|&&k| k != u32::MAX) else {
                continue;
            };
            if taken[i] {
                continue;
            }
            let (mz, y) = &spec[k as usize];
            let Some(d) = detect(&engine, mz, y, f.mz[i], sign) else {
                continue;
            };
            let pat = &d.patterns[0];
            let apex = f.apex[i];
            let mut members = vec![i];
            for p in &pat.peaks {
                if tol.within(f.mz[i], p.0) {
                    continue;
                }
                let t = tol.for_mass(p.0);
                let lo = by_mz.partition_point(|&j| f.mz[j] < p.0 - t);
                let best = by_mz[lo..]
                    .iter()
                    .take_while(|&&j| f.mz[j] <= p.0 + t)
                    .filter(|&&j| {
                        !taken[j]
                            && !members.contains(&j)
                            && f.scan_lo[j] <= apex
                            && apex <= f.scan_hi[j]
                    })
                    .min_by(|&&a, &&b| (f.mz[a] - p.0).abs().total_cmp(&(f.mz[b] - p.0).abs()));
                if let Some(&j) = best {
                    members.push(j);
                }
            }
            if members.len() < 2 {
                continue;
            }
            for &m in &members {
                taken[m] = true;
                gid[m] = i as u32 + 1;
                charge[m] = pat.charge as i8;
                if m != i {
                    f.flags[m] |= FLAG_ISOTOPE;
                }
            }
        }
    }
    f.iso_group = gid;
    f.charge = charge;
    Ok(())
}
