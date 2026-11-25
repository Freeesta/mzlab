//! What the feature recipe chose by itself for a file: its mass axis and, on unit resolution, the channel defect, the floor and the minimum height, with
//! the notices a reader of the table needs (profile scans at unit resolution cut one peak at a channel border). One function for the page, the app and
//! the CLI: `estimate_of` puts the notices in `estimates/1`, `mzlab-cli features` says them on stderr and in `features-notes/1` (`--notes`, or beside `--out`).

use mzlab_core::flow::value_text;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Value;
use mzlab_feat::axis::MassAxis;
use mzlab_feat::recipe::Recipe;

/// Key of the notice for unit-resolution data that are mostly profile scans; its parameters are `profile` and `scans`.
pub const PROFILE_UNIT: &str = "warn.features.profileUnit";

/// Key of the note that says the profile scans of a unit-resolution file were centroided before detection (`centroid` of `features.detect`); same parameters.
pub const CENTROIDED: &str = "note.features.centroided";

/// Key of the notice for a file whose scans are labelled profile but whose points are sparse ion counts: `auto` did not centroid it. Parameters `profile`,
/// `scans`, `runs` (share of points in runs, %) and `fine` (share of fine-grid spacings, %).
pub const LABEL_MISMATCH: &str = "warn.features.profileLabel";

/// A notice: a lang key and its whole-number parameters, in the order the text names them.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    pub key: &'static str,
    pub params: Vec<(&'static str, usize)>,
}

/// The recipe numbers of a unit-resolution file and what was counted on it.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// `true` for the 1 Da channel axis; `false` for the ppm axis, which has no notice.
    pub unit: bool,
    pub defect: f64,
    pub floor: f64,
    pub min_height: f64,
    /// MS1 scans in profile mode, and all MS1 scans.
    pub profile: usize,
    pub scans: usize,
    pub notices: Vec<Notice>,
}

/// The notices of `profile` profile scans among `scans` MS1 scans at unit resolution: the file counts as profile from half of its MS1 scans, the share
/// at which the recipe itself takes a per-point floor.
pub fn notices_of(profile: usize, scans: usize) -> Vec<Notice> {
    if mzlab_feat::lrscreen::is_profile(profile, scans) {
        vec![Notice {
            key: PROFILE_UNIT,
            params: vec![("profile", profile), ("scans", scans)],
        }]
    } else {
        Vec::new()
    }
}

/// The numbers of a label mismatch and the cut-offs they were judged by, whole numbers: what the notice, the report and the log say.
fn label_params(c: &mzlab_feat::lrscreen::LabelCheck) -> Vec<(&'static str, usize)> {
    let k = mzlab_feat::lrscreen::cuts();
    vec![
        ("profile", c.profile),
        ("scans", c.scans),
        ("probed", c.probed),
        ("points", c.points),
        ("spacings", c.spacings),
        ("runs", c.runs_pct),
        ("fine", c.fine_pct),
        ("runs_cut", k.runs_pct),
        ("fine_cut", k.fine_pct),
        ("run_points", k.run_points),
        ("run_step", k.run_step_mda),
        ("fine_step", k.fine_step_mda),
    ]
}

/// The warning of profile scans becomes the note that they were centroided when the recipe centroids this file; with `centroid` off the warning stays;
/// when the label is contradicted by the points (sparse ion counts) and `auto` left the file alone, the notice says that with the numbers.
fn screened<R: ReadAt>(mut n: Vec<Notice>, recipe: &Recipe, index: &mut Index<R>) -> Vec<Notice> {
    if n.is_empty() {
        return n;
    }
    if recipe.screen.centroid == mzlab_feat::lrscreen::Switch::Auto
        && let Some(c) = mzlab_feat::lrscreen::label_check(index).filter(|c| c.sparse)
    {
        return vec![Notice {
            key: LABEL_MISMATCH,
            params: label_params(&c),
        }];
    }
    if mzlab_feat::lrscreen::centroid_wanted(index, recipe) {
        n.iter_mut().for_each(|x| x.key = CENTROIDED);
    }
    n
}

/// The note of the file `index` measured with `recipe`.
pub fn note_of<R: ReadAt>(recipe: &Recipe, index: &mut Index<R>) -> Note {
    let ms1 = || index.scans.iter().filter(|s| s.level == 1);
    let (profile, scans) = (ms1().filter(|s| s.profile).count(), ms1().count());
    let d = &recipe.detect;
    match d.axis {
        MassAxis::Da { defect } => Note {
            unit: true,
            defect,
            floor: d.floor,
            min_height: d.min_height,
            profile,
            scans,
            notices: screened(notices_of(profile, scans), recipe, index),
        },
        MassAxis::Ppm => Note {
            unit: false,
            defect: 0.0,
            floor: d.floor,
            min_height: d.min_height,
            profile,
            scans,
            notices: Vec::new(),
        },
    }
}

/// The notices as the JSON value `[{"key", "params": {name: number}}]`, the one shape of `estimates/1` and of the notes file of the CLI.
pub fn notices_value(notices: &[Notice]) -> Value {
    Value::Arr(
        notices
            .iter()
            .map(|n| {
                let params = n
                    .params
                    .iter()
                    .map(|(k, v)| (k.to_string(), Value::Num(*v as f64)))
                    .collect();
                Value::Obj(vec![
                    ("key".into(), Value::Str(n.key.into())),
                    ("params".into(), Value::Obj(params)),
                ])
            })
            .collect(),
    )
}

/// The JSON text of the notices (`notices_value`).
pub fn notices_text(notices: &[Notice]) -> String {
    value_text(&notices_value(notices))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_share_of_profile_scans_is_tested_on_both_sides_of_one_half() {
        assert!(notices_of(19, 40).is_empty());
        assert_eq!(notices_of(20, 40).len(), 1);
        assert_eq!(notices_of(21, 40).len(), 1);
        assert!(notices_of(0, 40).is_empty());
        assert!(notices_of(0, 0).is_empty());
        assert_eq!(
            notices_of(1, 1)[0].params,
            vec![("profile", 1), ("scans", 1)]
        );
    }

    #[test]
    fn the_notices_are_the_json_array_the_estimate_carries() {
        assert_eq!(notices_text(&[]), "[]");
        assert_eq!(
            notices_text(&notices_of(20, 40)),
            r#"[{"key":"warn.features.profileUnit","params":{"profile":20,"scans":40}}]"#
        );
    }
}
