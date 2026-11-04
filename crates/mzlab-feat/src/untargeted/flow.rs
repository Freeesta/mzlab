//! `features.detect` with the algorithm `adap_localmin`: the common parameters of the step and `adap_peak::AdapPeak` as the `UntParams` of
//! `pipeline::run_file` (mass detection, ADAP chromatograms, optional Savitzky-Golay smoothing, local-minimum resolver), the features as the table
//! of `trace::detect`. Called by `trace::detect_with`; the rest of the per-file analysis (quality, isotopes, MS2, families) runs on top as ever.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use super::pipeline::{run_file, to_file_features, UntParams};
use crate::adap_peak::AdapPeak;
use crate::table::FileFeatures;
use crate::trace::Params;

/// The pipeline parameters of `p` with the model `a`: `floor` is the noise of the mass detection and the group intensity, `min_height` the highest
/// point of a chromatogram and the absolute height of a peak, `min_points` the consecutive scans and the points of a peak (as mzmine sets them equal).
/// On the unit axis the builder tolerance is half a dalton.
pub fn params(p: &Params, a: &AdapPeak) -> UntParams {
    let mut u = UntParams {
        polarity: p.polarity,
        iso: None,
        slab_bytes: p.budget.min(UntParams::default().slab_bytes),
        noise_factor: a.noise_factor,
        smooth_window: a.smooth_window,
        ..UntParams::default()
    };
    u.adap.noise = p.floor;
    u.adap.min_group_intensity = p.floor;
    u.adap.min_highest = p.min_height;
    u.adap.min_consecutive = p.min_points as usize;
    (u.adap.tol_ppm, u.adap.tol_da) = if p.axis.is_da() {
        (0.0, 0.5)
    } else {
        (a.ppm, a.tol_da)
    };
    u.resolve.chrom_threshold = a.chrom_threshold;
    u.resolve.search_rt = a.search_rt;
    u.resolve.min_ratio = a.min_ratio;
    u.resolve.min_abs_height = p.min_height;
    u.resolve.min_points = p.min_points as usize;
    u.resolve.duration = a.duration;
    u
}

/// The features of one file by the ADAP route, as the table that `trace::detect` gives (sorted by m/z, `dt` the median scan interval in seconds).
pub fn detect<R: ReadAt>(index: &mut Index<R>, p: &Params, a: &AdapPeak) -> Result<FileFeatures> {
    let u = params(p, a);
    let list = run_file(index, &u)?;
    let mut f = to_file_features(&list);
    if let Some(si) = index.section_of(1, p.polarity) {
        f.dt = crate::post::median_interval(&index.arrays(si)?.rt);
    }
    Ok(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With the floor and height of mzmine (1e4, 3e4) the defaults of the algorithm are the detection of the `untargeted` recipe with no settings.
    #[test]
    fn the_defaults_at_the_mzmine_floor_and_height_are_those_of_the_untargeted_recipe() {
        let p = Params {
            polarity: None,
            floor: 1e4,
            min_height: 3e4,
            ..Params::default()
        };
        let (u, d) = (params(&p, &AdapPeak::default()), UntParams::default());
        assert_eq!(u.adap, d.adap);
        assert_eq!(u.resolve, d.resolve);
        assert_eq!(
            (u.polarity, u.baseline, u.slab_bytes),
            (d.polarity, d.baseline, d.slab_bytes)
        );
        assert_eq!((u.noise_factor, u.smooth_window), (0.0, 0));
    }
}
