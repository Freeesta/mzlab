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

/// Commands that read `--json` themselves (a JSON form beside their text or CSV).
const JSON_FLAG: &[&str] = &[
    "flow run",
    "flow check",
    "flow verify",
    "steps",
    "validate",
    "pair",
    "samples",
    "dad",
    "elements",
    "folder",
    "manual-table",
];

/// Commands whose stdout is JSON anyway: `--json` changes nothing, except that a failure is an `error/1` as well.
const JSON_DEFAULT: &[&str] = &[
    "flow report",
    "flow record",
    "msn-lib",
    "msn-rules",
    "msn-graph",
    "llm",
    "mol-label",
    "mol-anal",
    "mol-check",
    "mol-cip",
    "mol-hilite",
    "mol-sel",
    "mol-link",
    "mol-cut",
    "mol-mech",
    "mol-search",
    "channels",
    "items",
    "dda-window",
    "ion-pattern",
    "ion-mz",
    "match-mz",
    "background",
    "wiff",
    "peak-edges",
    "peak-auto",
    "peak-auto-lr",
    "peak-area",
    "peak-window",
];

/// Whether the stdout of `name` is JSON without a flag (`dad` and `samples` too: they take `--json` as a no-op).
pub(crate) fn json_by_default(name: &str) -> bool {
    JSON_DEFAULT.contains(&name) || matches!(name, "dad" | "samples")
}

/// Whether `name` reads `--json` itself.
pub(crate) fn reads_json(name: &str) -> bool {
    JSON_FLAG.contains(&name)
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

const TOP: &str = "mzlab-cli: LC-MS analysis from the command line, the engine of the mzLab app; the files never leave this computer.

Start here:
  mzlab-cli steps --json                  the catalog of the Flow steps: ids, parameters, ports (what an analysis can be made of)
  mzlab-cli flow check PLAN.json --json   is a plan valid, with every problem named
  mzlab-cli flow run PLAN.json --in FILE... --out FOLDER   run a whole plan
  mzlab-cli COMMAND --help                the help of one command: on stdout, exit 0

For scripts and agents: --json is accepted by every command (the data as JSON on stdout; a failure is one error/1 document on stdout, the
text on stderr); progress goes to stderr. Exit codes: 0 ok; 1 data or domain error (partial outputs written); 2 usage; 3 plan, schema or
integrity; 4 input missing or unreadable; 5 selector not found (sample, channel, node, step instance); 6 output write or external source failure.";

const HEAD: &str = "mzlab-cli features: the feature table of one or more LC-MS files as CSV, from the engine of the mzLab app.";
const TAIL: &str = "  FILE...            mzML, or any single file the app reads (.raw, mzXML, .cdf, ...); several files form one series, in the order given
  --sample LIST      of a file with several samples (.wiff) only these, by id: 1,3,5 or 2-4, each one a member of the series (the others are
                     not read; an id the file does not have: exit 5); without it the first sample of such a file
  --out PATH         a .csv file, or a folder (then features.csv is written in it); default: standard output
  --preset NAME      auto (default: both polarities as the panel does), pos or neg
  --params FILE.json the numbers of the panel Pipeline: {\"polarity\", \"mode\", \"floor\", \"min_height\", \"min_points\", \"ppm\", \"rt_tol\", \"fill\"}
  --help             this text";

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

/// The text of `mzlab-cli --help`: where to start, the exit codes and the usage line of every command.
pub(crate) fn help() -> Outcome {
    let lines: Vec<&str> = entries()
        .map(|e| e.help)
        .filter(|h| !h.is_empty())
        .collect();
    let text = format!(
        "{TOP}\n\nusage: {}\n\n{}\n",
        lines.join("\n       "),
        crate::citation::reminder()
    );
    Outcome::ok(text.into_bytes())
}

/// The text of `mzlab-cli features --help`.
pub(crate) fn features_help() -> Outcome {
    let line = entries()
        .find(|e| e.name == "features")
        .map_or("", |e| e.help);
    let text = format!(
        "{HEAD}\n\nusage: {line}\n\n{TAIL}\n\n{}\n",
        crate::citation::reminder()
    );
    Outcome::ok(text.into_bytes())
}

/// `--help` or `-h` among the arguments of a command (before a bare `--`).
pub(crate) fn wants_help(rest: &[String]) -> bool {
    rest.iter()
        .take_while(|a| *a != "--")
        .any(|a| a == "--help" || a == "-h")
}

/// The subcommand words an entry lists in its usage lines (`mzlab-cli samples vars|group|list|select ...`).
pub(crate) fn subwords(e: &Entry) -> Vec<&str> {
    let marker = format!("mzlab-cli {} ", e.name);
    e.help
        .split("\n       ")
        .filter_map(|l| l.strip_prefix(marker.as_str()))
        .filter_map(|r| r.split(' ').next())
        .filter(|w| {
            w.contains('|')
                && w.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '|' || c == '-')
        })
        .flat_map(|w| w.split('|'))
        .collect()
}

/// The help of `e` for `--help`/`-h` anywhere in its arguments `rest`: nothing but `--help` (after a subcommand word the usage line lists) is
/// given to the command, so no file named before the flag is ever read; a command that has no text of its own for it prints its usage lines
/// from the table. Always exit 0, on stdout.
pub(crate) fn help_of(e: &Entry, rest: &[String]) -> Outcome {
    let mut ask = Vec::new();
    if let Some(w) = rest.first().filter(|w| subwords(e).contains(&w.as_str())) {
        ask.push(w.clone());
    }
    ask.push("--help".to_string());
    let own = (e.run)(ask);
    if own.code == 0 && !own.stdout.is_empty() && own.stderr.is_empty() {
        return own;
    }
    let lines: Vec<&str> = if !e.help.is_empty() {
        vec![e.help]
    } else {
        let group = format!("{} ", e.name);
        entries()
            .filter(|x| x.name.starts_with(&group) && !x.help.is_empty())
            .map(|x| x.help)
            .collect()
    };
    let text = if lines.is_empty() {
        format!("usage: mzlab-cli {} ...\n", e.name)
    } else {
        format!("usage: {}\n", lines.join("\n       "))
    };
    Outcome::ok(format!("{text}\n{}\n", crate::citation::reminder()).into_bytes())
}

#[cfg(test)]
mod tests;
