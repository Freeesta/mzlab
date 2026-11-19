//! `help-md`: the command list of the docs, printed from the table (hidden: it has no line in `--help`).

use super::Entry;
use crate::Outcome;

fn run(args: Vec<String>) -> Outcome {
    if let Some(a) = args.first() {
        return Outcome::fail(&format!("unknown argument {a}"));
    }
    Outcome::ok(super::list_md().into_bytes())
}

pub(crate) static TABLE: &[Entry] = &[Entry::new("help-md", "", run)];
