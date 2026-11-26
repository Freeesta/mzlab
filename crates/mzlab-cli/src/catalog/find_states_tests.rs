//! `commands --find` for the value states (detected, forced, absent): the words of a user asking whether a peak is real, whether an area was found or
//! only integrated, or whether a peak sits in a blank reach a command that says the state, among the first three answers.

use super::{search, words};

const TASKS: &[(&str, &str)] = &[
    ("forced integration", "quant targets"),
    ("detected versus integrated", "quant targets"),
    ("detected versus integrated", "peak table"),
    ("is this peak real or noise", "raw xic"),
    ("is this real", "raw xic"),
    (
        "state of the peak detected or forced in a peak table",
        "peak table",
    ),
    ("S/N and state of every peak", "peak table"),
];

#[test]
fn the_words_of_the_state_question_find_a_command_that_says_the_state_among_the_first_three() {
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
