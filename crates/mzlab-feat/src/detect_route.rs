//! The route of the detection of one file: the ADAP chromatograms of `adap_localmin` (`untargeted::flow`) or the ion traces of `trace`.
//!
//! The choice lives above `trace`, so that the traces do not depend on `untargeted` and its pipeline. A build without the feature
//! `untargeted` gives the ADAP recipe to `trace`, which refuses it.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::table::FileFeatures;
use crate::trace::{self, Params};

/// The features of one file and polarity by the route of `p.peak`.
pub fn detect<R: ReadAt>(index: &mut Index<R>, p: &Params) -> Result<FileFeatures> {
    detect_with(index, p, &mut |_, _| {})
}

/// `detect` telling `progress(done, total)` how far the traces are; the result is the same with any `progress`.
pub fn detect_with<R: ReadAt>(
    index: &mut Index<R>,
    p: &Params,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<FileFeatures> {
    #[cfg(feature = "untargeted")]
    if let crate::centwave::Peak::Adap(a) = &p.peak {
        return crate::untargeted::flow::detect(index, p, a);
    }
    trace::detect_with(index, p, progress)
}
