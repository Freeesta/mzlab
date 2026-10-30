//! The text of `trace::Params` in the recipe (its fingerprint). Used by `recipe` for the name of a `.mzfeatf`.

use crate::trace::{Params, Peak};

// The text of the recipe is built from this: the default peak model adds nothing, so the fingerprints of the files made before it exist keep their name.
impl std::fmt::Debug for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("Params");
        d.field("polarity", &self.polarity)
            .field("floor", &self.floor)
            .field("min_points", &self.min_points)
            .field("min_height", &self.min_height)
            .field("max_gap", &self.max_gap)
            .field("long_trace", &self.long_trace)
            .field("axis", &self.axis)
            .field("budget", &self.budget);
        if self.merge_overlap != 0.0 {
            d.field("merge_overlap", &self.merge_overlap);
        }
        if self.min_width != 0.0 {
            d.field("min_width", &self.min_width);
        }
        if self.min_snr != 0.0 {
            d.field("min_snr", &self.min_snr);
        }
        if self.peak != Peak::Minima {
            d.field("peak", &self.peak);
        }
        d.finish()
    }
}
