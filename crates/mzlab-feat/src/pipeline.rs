//! The whole per-file analysis: traces, quality, isotopologues, MS2 with purity.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::isotopes::{group, IsoParams};
use crate::ms2::{link, Ms2Links, Ms2Params};
use crate::quality::{assess, QParams};
use crate::table::FileFeatures;
use crate::trace::{detect, Params};

/// Parameters of the steps after the detection.
#[derive(Debug, Clone, Default)]
pub struct Steps {
    pub quality: QParams,
    pub iso: IsoParams,
    pub ms2: Ms2Params,
}

/// `trace::detect`, then the quality of every peak, the isotope groups and the MS2 scans linked to the peaks.
pub fn analyse<R: ReadAt>(
    index: &mut Index<R>,
    p: &Params,
    s: &Steps,
) -> Result<(FileFeatures, Ms2Links)> {
    let mut f = detect(index, p)?;
    assess(index, &mut f, p, &s.quality)?;
    group(&mut f, &s.iso);
    let links = link(index, &mut f, p, &s.ms2)?;
    Ok((f, links))
}
