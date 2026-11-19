//! The table of commands: each family file lists its commands as `Entry` values; `find` looks a command up and `help` prints `--help` from the same rows.
//! A command with an empty `help` is listed on the line of another one.

mod completions;
mod db;
mod docs;
mod features;
mod file;
mod flow;
mod ion;
mod msn;
mod peak;

use crate::Outcome;

pub(crate) struct Entry {
    /// The path of command words (`flow run`, `msn lib add`): one to three words.
    pub name: &'static str,
    /// The usage line after `mzlab-cli `, as printed by `--help`; empty for a hidden command.
    pub help: &'static str,
    pub run: fn(Vec<String>) -> Outcome,
    /// The `schema` of the JSON document the command prints, `""` when it has none or several (`mzlab-cli schema NAME` prints it).
    pub schema: &'static str,
    /// The exit codes the command can give (`exit.rs`); empty: the catalog reads them from the usage line (`catalog::exits_of`).
    pub exits: &'static [u8],
    /// Command lines for the catalog (`commands --json`).
    pub examples: &'static [&'static str],
}

impl Entry {
    pub(crate) const fn new(
        name: &'static str,
        help: &'static str,
        run: fn(Vec<String>) -> Outcome,
    ) -> Self {
        Entry {
            name,
            help,
            run,
            schema: "",
            exits: &[],
            examples: &[],
        }
    }

    pub(crate) const fn exits(mut self, exits: &'static [u8]) -> Self {
        self.exits = exits;
        self
    }

    pub(crate) const fn examples(mut self, examples: &'static [&'static str]) -> Self {
        self.examples = examples;
        self
    }

    pub(crate) const fn schema(mut self, schema: &'static str) -> Self {
        self.schema = schema;
        self
    }
}

/// `word` first among `rest`: the arguments of a subcommand whose entry sits in the table and whose code still reads its own word.
pub(crate) fn pre(word: &str, mut rest: Vec<String>) -> Vec<String> {
    rest.insert(0, word.to_string());
    rest
}

/// The families in the order of the help text.
static FAMILIES: [&[Entry]; 9] = [
    features::TABLE,
    flow::TABLE,
    file::TABLE,
    peak::TABLE,
    msn::TABLE,
    ion::TABLE,
    db::TABLE,
    completions::TABLE,
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
    "flow command",
    "explain-missing",
    "flow steps",
    "commands",
    "file validate",
    "file convert",
    "file pair",
    "file list",
    "ion elements",
    "peak table",
    "msn lib export",
    "db normative build",
    "db insilico pack",
];

/// Commands whose stdout is JSON anyway: `--json` changes nothing, except that a failure is an `error/1` as well.
const JSON_DEFAULT: &[&str] = &[
    "flow report",
    "flow record",
    "msn lib add",
    "msn lib build",
    "msn lib search",
    "msn rules",
    "msn rules sum",
    "msn graph",
    "msn complete",
    "msn compare",
    "msn figure",
    "mol label",
    "mol anal",
    "mol check",
    "mol cip",
    "mol hilite",
    "mol sel",
    "mol link",
    "mol cut",
    "mol mech",
    "mol search",
    "file channels",
    "file items",
    "file info",
    "file inventory",
    "trace chrom",
    "trace xic",
    "spectrum get",
    "spectrum list",
    "file verify",
    "spectrum dda-window",
    "ion pattern",
    "ion mz",
    "ion match",
    "trace background",
    "db ref-list",
    "peak edges",
    "peak auto",
    "peak auto-lr",
    "peak area",
    "peak window",
    "schema",
    "llm tools",
    "llm inspect",
    "llm params",
    "llm context",
    "llm replay",
    "llm eval",
];

/// Whether the stdout of `name` is JSON without a flag (`dad` and `sample` commands too: they take `--json` as a no-op).
pub(crate) fn json_by_default(name: &str) -> bool {
    JSON_DEFAULT.contains(&name) || name.starts_with("dad ") || name.starts_with("sample ")
}

/// Whether `name` reads `--json` itself.
pub(crate) fn reads_json(name: &str) -> bool {
    JSON_FLAG.contains(&name) || name.starts_with("dad ") || name.starts_with("sample ")
}

/// The entry of `args` and how many words of `args` its name takes: the longest name that `args` start with.
pub(crate) fn find(args: &[String]) -> Option<(&'static Entry, usize)> {
    entries()
        .filter_map(|e| {
            let words: Vec<&str> = e.name.split(' ').collect();
            (args.len() >= words.len() && words.iter().zip(args).all(|(w, a)| w == a))
                .then_some((e, words.len()))
        })
        .max_by_key(|(_, n)| *n)
}

/// The group (`flow`, `msn lib`) that `args` start with and that has subcommands: its path and the entries under it.
pub(crate) fn group_of(args: &[String]) -> Option<(String, Vec<&'static Entry>)> {
    let first = args.first()?;
    let mut path = first.clone();
    let mut inside: Vec<&Entry> = Vec::new();
    for k in 1..=2 {
        let p = format!("{path} ");
        let now: Vec<&Entry> = entries().filter(|e| e.name.starts_with(&p)).collect();
        if now.is_empty() {
            break;
        }
        inside = now;
        match args.get(k).filter(|w| !w.starts_with('-')) {
            Some(w) if entries().any(|e| e.name.starts_with(&format!("{path} {w} "))) => {
                path = format!("{path} {w}")
            }
            _ => break,
        }
    }
    (!inside.is_empty()).then_some((path, inside))
}

/// The command list of `docs/agenti/cli.md` (between its marker lines), written by `mzlab-cli help-md` and `tools/cli_doc.py`: one item per group
/// with its subcommands (`flow run|check|...`), wrapped under 480 characters.
pub(crate) fn list_md() -> String {
    let mut items: Vec<(String, Vec<&str>)> = Vec::new();
    for e in entries().filter(|e| e.name != "help-md") {
        let (key, last) = e.name.rsplit_once(' ').unwrap_or(("", e.name));
        match items.last_mut() {
            Some((k, v)) if k == key && !key.is_empty() => v.push(last),
            _ => items.push((key.to_string(), vec![last])),
        }
    }
    let mut out = String::new();
    let mut line = String::new();
    for (key, subs) in items {
        let item = if key.is_empty() {
            format!("`{}`", subs[0])
        } else {
            format!("`{key} {}`", subs.join("|"))
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
  mzlab-cli flow steps --json                  the catalog of the Flow steps: ids, parameters, ports (what an analysis can be made of)
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

/// The help of `e` for `--help`/`-h` anywhere in its arguments: nothing but `--help` is given to the command, so no file named before the flag is ever
/// read; a command that has no text of its own for it prints its usage line from the table. Always exit 0, on stdout.
pub(crate) fn help_of(e: &Entry, _rest: &[String]) -> Outcome {
    let own = (e.run)(vec!["--help".to_string()]);
    if own.code == 0
        && !own.stdout.is_empty()
        && own.stderr.is_empty()
        && !own.stdout.starts_with(b"{")
    {
        return own;
    }
    let text = if e.help.is_empty() {
        format!("usage: mzlab-cli {} ...\n", e.name)
    } else {
        format!("usage: {}\n", e.help)
    };
    Outcome::ok(format!("{text}\n{}\n", crate::citation::reminder()).into_bytes())
}

/// `mzlab-cli GROUP --help`, or a group word with no subcommand: the usage lines of the group.
pub(crate) fn group_help(inside: &[&Entry]) -> String {
    let lines: Vec<&str> = inside
        .iter()
        .map(|e| e.help)
        .filter(|h| !h.is_empty())
        .collect();
    format!(
        "usage: {}\n\n{}\n",
        lines.join("\n       "),
        crate::citation::reminder()
    )
}

#[cfg(test)]
mod tests;
