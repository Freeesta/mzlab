//! `commands --find` for the nominal-resolution hint: the words of a user who loses low-mass products or sees one ion in two rows, and of one who asks whether
//! the nominal profile fits the data, reach `estimate` (which measures the signature and names the preset) among the first three answers.

use super::{search, words};

const TASKS: &[(&str, &str)] = &[
    ("does my data need the nominal preset", "estimate"),
    ("nominal resolution preset hint", "estimate"),
    ("which preset for nominal resolution data", "estimate"),
    (
        "low mass products lost nominal resolution",
        "features.detect",
    ),
    ("one ion in two rows nominal resolution", "features.detect"),
];

#[test]
fn the_words_of_the_nominal_question_reach_the_estimate_and_the_detect_preset_among_the_first_three(
) {
    for (task, want) in TASKS {
        let got: Vec<String> = search(&words(task))
            .into_iter()
            .take(3)
            .map(|(it, _, _)| it.name)
            .collect();
        assert!(
            got.iter().any(|g| g == want),
            "{task:?}: {want} not in {got:?}"
        );
    }
}
