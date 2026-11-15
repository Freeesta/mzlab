//! The parameters of the detection of the ion traces (`trace::Params`) and of the peak models they name (`Peak`, `CentWave`), apart from the code
//! that runs them so that `channels`, `quality` and `centwave` read them without depending on `trace`. `trace` and `centwave` re-export them.

use crate::axis::MassAxis;
use crate::broad::BroadParams;
use crate::trace_base::{BYTES_PER_PEAK, FLOOR, MIN_HEIGHT, SLAB_PEAKS};

pub const MIN_POINTS: u32 = 5;
pub const LONG_TRACE: u32 = 400;
/// Scans a trace may miss between two of its points (0 = consecutive scans), as `gapfill::Params::max_gap`.
pub const MAX_GAP: u32 = 1;

#[derive(Clone)]
pub struct Params {
    /// Polarity of the MS1 scans (1 or -1); `None` = all the MS1 scans.
    pub polarity: Option<i8>,
    pub floor: f64,
    pub min_points: u32,
    pub min_height: f64,
    /// Missed scans allowed between two points of a trace (0 = consecutive).
    pub max_gap: u32,
    pub long_trace: u32,
    /// Two rows of different grids are one ion when their scans overlap by this share of the shorter one (0 = only the apex and edges test of `merge_grids`).
    pub merge_overlap: f64,
    /// Pieces of neighbouring channels join when they touch whatever their height, and pieces down to a quarter of `min_height` take part (`channels::stitch`).
    pub stitch_wide: bool,
    /// Noise rows of the unit profile: a row narrower than this share of the typical peak width of its file is dropped (0 = off, `lrnoise`).
    pub min_width: f64,
    /// A row whose apex is below this many times the noise of its flanks is dropped (0 = off, `lrnoise`).
    pub min_snr: f64,
    /// Axis of the channels of m/z (`Da` = unit resolution).
    pub axis: MassAxis,
    /// Working memory of the detection, bytes: it sets the size of the slabs (at most `SLAB_PEAKS` peaks).
    pub budget: usize,
    /// How the traces become peaks; only the wavelet model shows in the text of the recipe.
    pub peak: Peak,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            polarity: Some(1),
            floor: FLOOR,
            min_points: MIN_POINTS,
            min_height: MIN_HEIGHT,
            max_gap: MAX_GAP,
            long_trace: LONG_TRACE,
            merge_overlap: 0.0,
            stitch_wide: false,
            min_width: 0.0,
            min_snr: 0.0,
            axis: MassAxis::Ppm,
            budget: SLAB_PEAKS * BYTES_PER_PEAK,
            peak: Peak::Minima,
        }
    }
}

/// How the segments of a slab become peaks: the cut of the runs at their gaps (`masstrace`), the wavelet model, or the ADAP chromatograms and the
/// local-minimum resolver of mzmine (`adap_localmin`, which builds its own traces from the index).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Peak {
    #[default]
    Minima,
    CentWave(CentWave),
    Adap(crate::adap_peak::AdapPeak),
}

/// Parameters of the wavelet model (descriptor `features.detect`, algorithm `centwave`).
#[derive(Clone, PartialEq)]
pub struct CentWave {
    /// Half window of m/z (ppm; on the Da axis 0.5 Da) around a trace where its intensities are summed per scan.
    pub ppm: f64,
    /// Range of the full width at half maximum of the peaks, seconds.
    pub width_s: (f64, f64),
    /// Minimum height over the baseline, in units of the noise of the trace (never less than `Params::floor`).
    pub snr: f64,
    /// Broad bands (30-300 s) on the residue of the narrow peaks; off by default.
    pub broad: BroadParams,
}

impl Default for CentWave {
    fn default() -> Self {
        CentWave {
            ppm: 25.0,
            width_s: (5.0, 30.0),
            snr: 10.0,
            broad: BroadParams::default(),
        }
    }
}

// The text of the recipe is built from this: the broad pass adds nothing while it is off, so the fingerprints made before it exist keep their name.
impl std::fmt::Debug for CentWave {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("CentWave");
        d.field("ppm", &self.ppm)
            .field("width_s", &self.width_s)
            .field("snr", &self.snr);
        if self.broad.enabled {
            d.field("broad", &self.broad);
        }
        d.finish()
    }
}

impl CentWave {
    /// Sets the range of the full width at half maximum of the peaks, seconds.
    pub fn with_width_s(mut self, width_s: (f64, f64)) -> Self {
        self.width_s = width_s;
        self
    }
}
