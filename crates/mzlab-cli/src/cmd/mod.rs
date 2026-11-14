//! The table of commands: each family file lists its commands as `Entry` values; `find` looks a command up and `help` prints `--help` from the same rows.
//! A command with an empty `help` is listed on the line of another one.

mod features;
mod flow;
mod library;
mod llm;
mod mol;
mod spectra;
mod traces;

use crate::Outcome;

pub(crate) struct Entry {
    /// One word, or two for a subcommand (`flow run`).
    pub name: &'static str,
    /// Old spellings still accepted on input.
    pub aliases: &'static [&'static str],
    /// The usage line(s) after `mzlab-cli `, as printed by `--help`; empty when another entry's line covers this command.
    pub help: &'static str,
    pub run: fn(Vec<String>) -> Outcome,
}

/// The families in the order of the help text.
static FAMILIES: [&[Entry]; 7] = [
    features::TABLE,
    flow::TABLE,
    llm::TABLE,
    spectra::TABLE,
    mol::TABLE,
    traces::TABLE,
    library::TABLE,
];

pub(crate) fn entries() -> impl Iterator<Item = &'static Entry> {
    FAMILIES.iter().flat_map(|f| f.iter())
}

/// The entry of `args` and how many words of `args` its name takes; two-word names are tried in table order before their one-word fallback.
pub(crate) fn find(args: &[String]) -> Option<(&'static Entry, usize)> {
    let first = canonical(args.first()?);
    entries().find_map(|e| match e.name.split_once(' ') {
        Some((a, b)) => (a == first && args.get(1).is_some_and(|x| x == b)).then_some((e, 2)),
        None => (e.name == first).then_some((e, 1)),
    })
}

/// The current name of a command given by an old spelling.
pub(crate) fn canonical(word: &str) -> &str {
    entries()
        .find(|e| e.aliases.contains(&word))
        .map_or(word, |e| e.name)
}

const HEAD: &str = "mzlab-cli features: the feature table of one or more LC-MS files as CSV, from the engine of the mzLab app.";
const TAIL: &str = "  FILE...            mzML, or any single file the app reads (.raw, mzXML, .cdf, ...); several files form one series, in the order given
  --out PATH         a .csv file, or a folder (then features.csv is written in it); default: standard output
  --preset NAME      auto (default: both polarities as the panel does), pos or neg
  --params FILE.json the numbers of the panel Pipeline: {\"polarity\", \"mode\", \"floor\", \"min_height\", \"min_points\", \"ppm\", \"rt_tol\", \"fill\"}
  --help             this text

old names still accepted: canali, campioni, peaks-edges, nome-gruppo, variabili, --canale, --jsonl";

/// The text of `mzlab-cli --help`.
pub(crate) fn help() -> Outcome {
    let lines: Vec<&str> = entries()
        .map(|e| e.help)
        .filter(|h| !h.is_empty())
        .collect();
    let text = format!("{HEAD}\n\nusage: {}\n\n{TAIL}\n", lines.join("\n       "));
    Outcome::ok(text.into_bytes())
}

#[cfg(test)]
mod tests;
