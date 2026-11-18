//! The table of commands: each family file lists its commands as `Entry` values; `find` looks a command up and `help` prints `--help` from the same rows.
//! A command with an empty `help` is listed on the line of another one.

mod docs;
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
static FAMILIES: [&[Entry]; 8] = [
    features::TABLE,
    flow::TABLE,
    llm::TABLE,
    spectra::TABLE,
    mol::TABLE,
    traces::TABLE,
    library::TABLE,
    docs::TABLE,
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

/// The command list of `docs/agenti/cli.md` (between its marker lines), written by `mzlab-cli help-md` and `tools/cli_doc.py`: the names in table order, wrapped under 480 characters.
pub(crate) fn list_md() -> String {
    let mut out = String::new();
    let mut line = String::new();
    for e in entries().filter(|e| e.name != "help-md" && e.name != "flow") {
        let old: Vec<String> = e.aliases.iter().map(|a| format!("`{a}`")).collect();
        let item = if old.is_empty() {
            format!("`{}`", e.name)
        } else {
            format!("`{}` (old: {})", e.name, old.join(", "))
        };
        if !line.is_empty() && line.len() + item.len() + 2 > 480 {
            out += &format!("{line}\n");
            line.clear();
        }
        line += &(if line.is_empty() {
            item
        } else {
            format!(", {item}")
        });
    }
    out + &line + "\n"
}

const HEAD: &str = "mzlab-cli features: the feature table of one or more LC-MS files as CSV, from the engine of the mzLab app.";
const TAIL: &str = "  FILE...            mzML, or any single file the app reads (.raw, mzXML, .cdf, ...); several files form one series, in the order given
  --sample LIST      of a file with several samples (.wiff) only these, by id: 1,3,5 or 2-4, each one a member of the series (the others are
                     not read; an id the file does not have: exit 5); without it the first sample of such a file
  --out PATH         a .csv file, or a folder (then features.csv is written in it); default: standard output
  --preset NAME      auto (default: both polarities as the panel does), pos or neg
  --params FILE.json the numbers of the panel Pipeline: {\"polarity\", \"mode\", \"floor\", \"min_height\", \"min_points\", \"ppm\", \"rt_tol\", \"fill\"}
  --help             this text

old names still accepted: peaks-edges, --jsonl";

/// `mzlab-cli`, `--help`/`-h` and `--version`/`-V`: the answers that name no command.
pub(crate) fn top_level(args: &[String]) -> Option<Outcome> {
    match args.first().map(String::as_str) {
        None | Some("--help" | "-h") => Some(help()),
        Some("--version" | "-V") => Some(Outcome::ok(
            format!(
                "mzlab-cli {}\n{}\n",
                crate::citation::VERSION,
                crate::citation::reminder()
            )
            .into_bytes(),
        )),
        _ => None,
    }
}

/// The text of `mzlab-cli --help`.
pub(crate) fn help() -> Outcome {
    let lines: Vec<&str> = entries()
        .map(|e| e.help)
        .filter(|h| !h.is_empty())
        .collect();
    let text = format!(
        "{HEAD}\n\nusage: {}\n\n{TAIL}\n\n{}\n",
        lines.join("\n       "),
        crate::citation::reminder()
    );
    Outcome::ok(text.into_bytes())
}

#[cfg(test)]
mod tests;
