//! What `features` chose by itself for the first file, said on stderr: the resolution class and the unit-axis numbers measured on it, and a warning when
//! unit-resolution data are in profile mode (pieces of one peak cut by the border of a 1 Da channel come out as extra rows). stdout is untouched.

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_feat::axis::MassAxis;
use mzlab_feat::recipe::Recipe;

use crate::convert::Notice;

/// The share of profile scans at which the file counts as profile (the one `recipe_lr::profile_floor` uses).
const PROFILE_SHARE: f64 = 0.5;

/// `par::index_and_detect`, then the lines of the first file on stderr (nothing when its index cannot be opened: the run is not about to fail on a note).
pub(crate) fn index_and_detect(
    files: &[std::path::PathBuf],
    labels: &[String],
    idxs: &[std::path::PathBuf],
    params: &str,
) -> mzlab_core::Result<crate::par::Detected> {
    let done = crate::par::index_and_detect(files, labels, idxs, params)?;
    if let Ok(index) = crate::open(&idxs[0]) {
        for l in lines(&done.0, &index, &files[0].display().to_string()) {
            eprintln!("{l}");
        }
    }
    Ok(done)
}

/// The lines for stderr, none for a high-resolution file: the progress-style line of the recipe and, for unit-resolution profile data, the warning.
pub(crate) fn lines<R: ReadAt>(recipe: &Recipe, index: &Index<R>, name: &str) -> Vec<String> {
    let MassAxis::Da { defect } = recipe.detect.axis else {
        return Vec::new();
    };
    let ms1: Vec<_> = index.scans.iter().filter(|s| s.level == 1).collect();
    let profile = ms1.iter().filter(|s| s.profile).count();
    let mut out = vec![format!(
        "recipe {name}: unit resolution (1 Da channels centred on n{defect:+.3}), floor {} counts, minimum height {} counts, {profile} of {} MS1 scans in profile mode",
        recipe.detect.floor,
        recipe.detect.min_height,
        ms1.len()
    )];
    if !ms1.is_empty() && profile as f64 >= PROFILE_SHARE * ms1.len() as f64 {
        let n = Notice {
            key: "warn.features.profileUnit",
            params: vec![
                ("profile", profile.to_string()),
                ("scans", ms1.len().to_string()),
            ],
        };
        out.push(format!(
            "warning: {name}: {}",
            crate::error_text::warning(&n)
        ));
    }
    out
}

#[cfg(test)]
#[path = "features_note_tests.rs"]
mod tests;
