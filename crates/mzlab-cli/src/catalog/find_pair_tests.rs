//! `commands --find` for the polarity pairing: the words of the task (acids, both polarities, neutral mass), not the command name, reach `table pair` and `features.pair`.

use super::{search, words};

#[test]
fn the_words_of_the_polarity_task_find_the_pairing() {
    for q in [
        "which species are acids in both polarities",
        "pair positive and negative features by neutral mass",
        "[M+H]+ and [M-H]- match with an RT tolerance",
    ] {
        let got: Vec<String> = search(&words(q))
            .into_iter()
            .take(5)
            .map(|(it, _, _)| it.name)
            .collect();
        for want in ["table pair", "features.pair"] {
            assert!(
                got.iter().any(|g| g == want),
                "{q:?}: {want} not in {got:?}"
            );
        }
    }
}
