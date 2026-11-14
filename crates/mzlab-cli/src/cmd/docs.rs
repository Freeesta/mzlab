//! `help-md`: the command list of the docs, printed from the table (hidden: it has no line in `--help`).

use super::Entry;
use crate::Outcome;

fn run(_: Vec<String>) -> Outcome {
    Outcome::ok(super::list_md().into_bytes())
}

pub(crate) static TABLE: &[Entry] = &[Entry {
    name: "help-md",
    aliases: &[],
    help: "",
    run,
}];
