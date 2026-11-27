//! `commands --find` for the compound list (`mol find`): the words of a person who looks for a pesticide or a drug by name reach the command.

use super::{search, words};

#[test]
fn a_compound_lookup_is_found_from_the_task() {
    for q in [
        "is this pesticide in the built-in compound list",
        "look up a compound by name or synonym",
        "which formula and exact mass has a herbicide",
    ] {
        let got: Vec<String> = search(&words(q))
            .into_iter()
            .take(3)
            .map(|(it, _, _)| it.name)
            .collect();
        assert!(got.iter().any(|g| g == "mol find"), "{q:?}: mol find not in {got:?}");
    }
}
