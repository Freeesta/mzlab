//! Parameters of the peak model `adap_localmin` of `features.detect` (`Peak::Adap`): the mass detection, the ADAP chromatogram builder, the optional
//! Savitzky-Golay smoothing and the local-minimum resolver of `untargeted/` (mzmine 3.9.0). Plain numbers with no feature gate, so the recipe and the
//! descriptor exist in every build; the run is `untargeted::flow_detect` (feature `untargeted`). Defaults are those of `untargeted::pipeline::UntParams`.

/// The settings of `adap_localmin` that the common parameters of `features.detect` do not carry (`floor`, `min_height`, `min_points` are shared).
#[derive(Debug, Clone, PartialEq)]
pub struct AdapPeak {
    /// Factor of lowest signal: a point below this many times the lowest positive intensity of its scan is not taken; 0 = the absolute `floor`.
    pub noise_factor: f64,
    /// m/z tolerance of the chromatogram builder, ppm, and its absolute lower bound, Da (the larger one wins).
    pub ppm: f64,
    pub tol_da: f64,
    /// Savitzky-Golay window in scans before the minimum search (0 = no smoothing); height, area and m/z stay those of the raw trace.
    pub smooth_window: usize,
    /// Resolver: quantile threshold of the trace, minimum search range (min), top/edge ratio, peak duration range (min).
    pub chrom_threshold: f64,
    pub search_rt: f64,
    pub min_ratio: f64,
    pub duration: (f64, f64),
}

impl Default for AdapPeak {
    fn default() -> Self {
        AdapPeak {
            noise_factor: 0.0,
            ppm: 5.0,
            tol_da: 0.002,
            smooth_window: 0,
            chrom_threshold: 0.85,
            search_rt: 0.1,
            min_ratio: 1.7,
            duration: (0.0, 1.0),
        }
    }
}
