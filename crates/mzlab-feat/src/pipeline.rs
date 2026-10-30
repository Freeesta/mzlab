//! The whole per-file analysis: traces, quality, isotopologues, MS2 with purity.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::isotopes::{group, IsoParams};
use crate::ms2::{link, Ms2Links, Ms2Params};
use crate::prof::time;
use crate::quality::{assess, QParams};
use crate::resolve::{assess_and_resolve, mark_mixtures, ResolveParams};
use crate::table::FileFeatures;
use crate::trace::{detect_with, Params};

/// Parameters of the steps after the detection.
#[derive(Debug, Clone)]
pub struct Steps {
    pub quality: QParams,
    pub iso: IsoParams,
    pub ms2: Ms2Params,
    pub resolve: ResolveParams,
    /// Cut the features with several separated maxima (`resolve`).
    pub split: bool,
}

impl Default for Steps {
    fn default() -> Self {
        Steps {
            quality: QParams::default(),
            iso: IsoParams::default(),
            ms2: Ms2Params::default(),
            resolve: ResolveParams::default(),
            split: true,
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
        if s.split {
            Ok(assess_and_resolve(index, &mut f, p, &s.quality, &s.resolve)?.multi)
        } else {
            assess(index, &mut f, p, &s.quality)?;
            Ok::<_, mzlab_core::error::Error>(Vec::new())
        }
    })?;
    if p.min_width > 0.0 || p.min_snr > 0.0 {
        crate::lrnoise::reject(&mut f, &mut multi, p);
    }
    time("detect: isotopes", || group(&mut f, &s.iso))?;
    let links = time("detect: ms2 links", || link(index, &mut f, p, &s.ms2))?;
    time("detect: mixtures", || {
        mark_mixtures(index, &mut f, &links, &multi, p, &s.resolve)
    })?;
    Ok((f, links))
}
