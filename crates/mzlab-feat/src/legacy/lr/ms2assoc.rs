//! The rule that ties an MS2 scan to a feature (`docs/agenti/mzfinder-ms2-regola.md`): the centre of the precursor lies within the m/z tolerance of the
//! feature AND the time of the scan lies inside the chromatographic peak of the feature (from its start to its end, not only near the apex). Pure
//! functions, no index: the Dig uses `tolerance` and `overlap` for the matches of the traces, and `associate` for the scans of one match.

/// Floor of the tolerance, ppm (a window that the file declares smaller than this is read as this).
pub const PPM_FLOOR: f64 = 10.0;

/// The m/z tolerance used for a precursor, and where it comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tol {
    /// Da.
    pub da: f64,
    /// Half the isolation window written in the file; `false`: the window is not known and the tolerance of the table was used.
    pub from_window: bool,
}

/// `max(half the isolation window, ppm floor)`; without the window `max(table tolerance, ppm floor)` (declared by `from_window = false`).
pub fn tolerance(half_window: Option<f64>, table_tol: f64, mz: f64) -> Tol {
    let floor = mz.abs() * PPM_FLOOR * 1e-6;
    let known = half_window.filter(|h| h.is_finite() && *h > 0.0);
    Tol {
        da: known.unwrap_or(table_tol).max(floor),
        from_window: known.is_some(),
    }
}

/// Distance of a precursor to the m/z of a feature: Da and ppm of the feature (unsigned).
pub fn delta(precursor: f64, mz: f64) -> (f64, f64) {
    let d = (precursor - mz).abs();
    (d, if mz != 0.0 { d / mz.abs() * 1e6 } else { 0.0 })
}

/// The chromatographic peak of a feature, minutes.
#[derive(Debug, Clone, Copy)]
pub struct Feature {
    pub mz: f64,
    pub apex: f64,
    pub lo: f64,
    pub hi: f64,
}

/// An MS2 scan: its time, the centre and the half-width of the isolation window (when the file says it) and its intensity (the precursor intensity when
/// the scan has it, else the TIC of the scan).
#[derive(Debug, Clone, Copy)]
pub struct Ms2Scan {
    pub rt: f64,
    pub precursor: f64,
    pub half_window: Option<f64>,
    pub intensity: f64,
}

/// The scans of a feature.
#[derive(Debug, Clone, PartialEq)]
pub struct Assoc {
    /// Position (in the input) of the representative scan.
    pub rep: usize,
    /// Scans inside the peak that fit the m/z (the representative included).
    pub n_in: usize,
    /// Distance of the representative's precursor to the feature, Da and ppm.
    pub dmz: f64,
    pub dppm: f64,
    /// Time of the representative minus the apex of the feature, minutes.
    pub drt: f64,
    pub tol: Tol,
}

/// Whether the scan is a scan of the feature: m/z within the tolerance and time between the start and the end of the peak.
pub fn fits(f: &Feature, s: &Ms2Scan, table_tol: f64) -> Option<Tol> {
    let t = tolerance(s.half_window, table_tol, f.mz);
    (delta(s.precursor, f.mz).0 <= t.da && s.rt >= f.lo && s.rt <= f.hi).then_some(t)
}

/// The scans of `scans` that belong to `f`, and the one that represents them: the scan with the highest intensity (the precursor intensity, else the
/// TIC); a tie goes to the scan closest to the apex of the feature. `None` when no scan fits.
pub fn associate(f: &Feature, scans: &[Ms2Scan], table_tol: f64) -> Option<Assoc> {
    let mut best: Option<(usize, Tol)> = None;
    let mut n_in = 0;
    for (i, s) in scans.iter().enumerate() {
        let Some(t) = fits(f, s, table_tol) else {
            continue;
        };
        n_in += 1;
        let better = best.is_none_or(|(b, _)| {
            let o = &scans[b];
            s.intensity > o.intensity
                || (s.intensity == o.intensity && (s.rt - f.apex).abs() < (o.rt - f.apex).abs())
        });
        if better {
            best = Some((i, t));
        }
    }
    best.map(|(rep, tol)| {
        let s = &scans[rep];
        let (dmz, dppm) = delta(s.precursor, f.mz);
        Assoc {
            rep,
            n_in,
            dmz,
            dppm,
            drt: s.rt - f.apex,
            tol,
        }
    })
}

/// The part of `[a_lo, a_hi]` that lies in `[b_lo, b_hi]`, if any.
pub fn overlap(a_lo: f64, a_hi: f64, b_lo: f64, b_hi: f64) -> Option<(f64, f64)> {
    let (lo, hi) = (a_lo.max(b_lo), a_hi.min(b_hi));
    (lo <= hi).then_some((lo, hi))
}
