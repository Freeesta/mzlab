//! `commands` and `schema` (the self-describing catalog) and `help-md`, the command list of the docs (hidden: it has no line in `--help`).

use super::Entry;
use crate::Outcome;

fn run(args: Vec<String>) -> Outcome {
    let text = match args.as_slice() {
        [] => super::list_md(),
        [a] if a == "json" => super::json_md(),
        [a] if a == "exits" => super::exits_md(),
        [a, ..] => return Outcome::fail(&format!("unknown argument {a}")),
    };
    Outcome::ok(text.into_bytes())
}

pub(crate) static TABLE: &[Entry] = &[
    Entry::new("commands", "mzlab-cli commands [--json] [PATH...]   every command: arguments, flags, output schema, exit codes (commands/1)", crate::catalog::run)
        .schema("commands/1")
        .exits(&[0, 2, 5])
        .examples(&["mzlab-cli commands --json", "mzlab-cli commands --json flow"]),
    Entry::new("schema", "mzlab-cli schema [--list | NAME]   the JSON Schema NAME (error/1, mzflow/2, ...) or their list (schemas/1)", crate::schema_cmd::run)
        .schema("schemas/1")
        .exits(&[0, 2, 5])
        .examples(&["mzlab-cli schema --list", "mzlab-cli schema mzflow/2"]),
    Entry::new("help-md", "", run).exits(&[0, 2]),
];
