//! The parameters of the steps after the detection (`Steps`): plain data on the settings of quality, isotopologues, MS2 and resolution, so the
//! recipe can carry them without reaching the pipeline that runs them. The old path `mzlab_feat::pipeline::Steps` stays as a re-export.

use crate::align_mzmine::{IsoFinder, IsoGrouper};
use crate::isotopes::IsoParams;
use crate::ms2::Ms2Params;
use crate::quality::QParams;
use crate::resolve::ResolveParams;

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
    /// The isotope finder of mzmine in place of `iso` (`isotopes` = `finder`) and its settings, kept when it is off.
    pub use_finder: bool,
    pub finder: IsoFinder,
}

/// The derived text with `grouper` and `finder` written only when chosen: the recipe fingerprint of the default stays the same.
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
        if self.use_finder {
            d.field("finder", &self.finder);
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
            use_finder: false,
            finder: IsoFinder::default(),
        }
    }
}
