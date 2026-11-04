//! The whole per-file analysis: traces, quality, isotopologues, MS2 with purity.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::align_mzmine::IsoGrouper;
use crate::isotopes::{group, IsoParams};
use crate::ms2::{link, Ms2Links, Ms2Params};
use crate::prof::time;
use crate::quality::{assess, QParams};
use crate::resolve::{assess_and_resolve, mark_mixtures, ResolveParams};
use crate::table::FileFeatures;
use crate::trace::{detect_with, Params};

/// Parameters of the steps after the detection.
#[derive(Clone)]
pub struct Steps {
    pub quality: QParams,
    pub iso: IsoParams,
    pub ms2: Ms2Params,
    pub resolve: ResolveParams,
    /// Cut the features with several separated maxima (`resolve`).
    pub split: bool,
    /// The isotope grouper of mzmine in place of `iso` (`features.detect`, `isotopes` = `grouper`) and its settings, kept when it is off.
    pub use_grouper: bool,
    pub grouper: IsoGrouper,
}

/// The derived text with `grouper` written only when chosen: the recipe fingerprint of the default stays the same.
impl std::fmt::Debug for Steps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("Steps");
        d.field("quality", &self.quality)
            .field("iso", &self.iso)
            .field("ms2", &self.ms2)
            .field("resolve", &self.resolve)
            .field("split", &self.split);
        if self.use_grouper {
            d.field("grouper", &self.grouper);
        }
        d.finish()
    }
}

impl Default for Steps {
    fn default() -> Self {
        Steps {
            quality: QParams::default(),
            iso: IsoParams::default(),
            ms2: Ms2Params::default(),
            resolve: ResolveParams::default(),
            split: true,
            use_grouper: false,
            grouper: IsoGrouper::default(),
        }
    }
}

/// `trace::detect`, then the quality of every peak, the isotope groups and the MS2 scans linked to the peaks.
pub fn analyse<R: ReadAt>(
    index: &mut Index<R>,
    p: &Params,
    s: &Steps,
) -> Result<(FileFeatures, Ms2Links)> {
    analyse_with(index, p, s, &mut |_, _| {})
}

/// `analyse` telling `progress(done, total)` how far the detection is; the result is the same with any `progress`.
pub fn analyse_with<R: ReadAt>(
    index: &mut Index<R>,
    p: &Params,
    s: &Steps,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<(FileFeatures, Ms2Links)> {
    let mut f = time("detect: traces", || detect_with(index, p, progress))?;
    let mut multi = time("detect: quality + resolve", || {
        // the local-minimum resolver of `adap_localmin` has already cut the traces
        if s.split && !matches!(p.peak, crate::centwave::Peak::Adap(_)) {
            Ok(assess_and_resolve(index, &mut f, p, &s.quality, &s.resolve)?.multi)
        } else {
            assess(index, &mut f, p, &s.quality)?;
            Ok::<_, mzlab_core::error::Error>(Vec::new())
        }
    })?;
    if p.min_width > 0.0 || p.min_snr > 0.0 {
        crate::lrnoise::reject(&mut f, &mut multi, p);
    }
    time("detect: isotopes", || isotopes(&mut f, s))?;
    let links = time("detect: ms2 links", || link(index, &mut f, p, &s.ms2))?;
    time("detect: mixtures", || {
        mark_mixtures(index, &mut f, &links, &multi, p, &s.resolve)
    })?;
    Ok((f, links))
}

/// The isotope groups of one file: the averagine grouping of `isotopes.rs`, or the isotope grouper of mzmine when `grouper` is chosen.
pub(crate) fn isotopes(f: &mut FileFeatures, s: &Steps) -> Result<()> {
    if s.use_grouper {
        grouper(f, &s.grouper)
    } else {
        group(f, &s.iso)
    }
}

#[cfg(feature = "untargeted")]
fn grouper(f: &mut FileFeatures, g: &IsoGrouper) -> Result<()> {
    crate::untargeted::flow_iso::group(f, g)
}

#[cfg(not(feature = "untargeted"))]
fn grouper(_: &mut FileFeatures, _: &IsoGrouper) -> Result<()> {
    Err(mzlab_core::error::Error::new("error.badRecipe"))
}
