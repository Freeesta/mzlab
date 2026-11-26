//! The hint of a loop of calls (AGENT-DISCOVERY): a command that has a one-call equivalent for many targets, started over and over by the same parent
//! process (a script, a shell loop, `xargs -P`), says once on stderr which one call does the whole list. State is needed because only the count of calls
//! of one parent tells a loop from a single call: one append-only file per parent and command in the temp folder (every call appends one byte, so
//! parallel calls never lose a count) and a marker made with `create_new`, so the hint is said exactly once. The static routes (help, `related`,
//! the task table) need no state and are the main ones; this is the nudge for a loop that never read them. Stdout and the exit code never change,
//! `MZLAB_NO_HINTS` set to any value silences it. Unix only (the parent is
//! tagged by its id and `/proc` start time; elsewhere there is no hint). Called by the binary (`main.rs`), never by `execute`.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::convert::Notice;

/// Calls from one parent after which the hint is said, once.
pub const FIRST: u64 = 10;
/// A pause longer than this (seconds) between two calls starts the count again (a parent process id can be reused).
pub const GAP_S: u64 = 600;
/// State files untouched for this long (seconds) are deleted by the next call that starts a count.
pub const STALE_S: u64 = 86_400;

/// One looped command and the call that replaces the loop.
pub struct Rule {
    pub command: &'static str,
    /// The lang key of the hint (`mzlab/web/lang/en/`): the text names the one call that replaces the loop.
    pub key: &'static str,
    /// Words for `commands --find`.
    pub find: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        command: "peak auto-lr",
        key: "note.cli.repeat.targets",
        find: "many targets files area",
    },
    Rule {
        command: "tp.dig_lr",
        key: "note.cli.repeat.dig",
        find: "dig kinetics series",
    },
    Rule {
        command: "trace xic",
        key: "note.cli.repeat.xic",
        find: "many masses sweep",
    },
    Rule {
        command: "raw xic",
        key: "note.cli.repeat.rawxic",
        find: "many masses every file",
    },
];

/// The rule of a command line: `peak auto-lr ...`, or `flow run PLAN` whose plan holds a `tp.dig_lr` step.
pub fn rule_of(args: &[String]) -> Option<&'static Rule> {
    let at = |i: usize| args.get(i).map(String::as_str);
    match (at(0), at(1)) {
        (Some("peak"), Some("auto-lr")) => RULES.first(),
        (Some("flow"), Some("run")) => {
            let plan = args.iter().skip(2).find(|a| !a.starts_with('-'))?;
            let text = std::fs::read_to_string(plan)
                .ok()
                .filter(|t| t.len() < 4_000_000)?;
            text.contains("\"tp.dig_lr@").then(|| &RULES[1])
        }
        (Some("trace"), Some("xic")) => {
            (one_mass(args) && !args.iter().any(|a| a == "--sweep")).then(|| &RULES[2])
        }
        (Some("raw"), Some("xic")) => one_mass(args).then(|| &RULES[3]),
        _ => None,
    }
}

/// The call names one mass (`--mz 364.4`, no comma): a list or a sweep is already the one call and is never counted.
fn one_mass(args: &[String]) -> bool {
    let v = args.iter().enumerate().find_map(|(i, a)| {
        a.strip_prefix("--mz=").or_else(|| {
            (a == "--mz")
                .then(|| args.get(i + 1).map(String::as_str))
                .flatten()
        })
    });
    v.is_some_and(|m| !m.contains(','))
}

/// One counted call.
#[derive(Debug, PartialEq, Eq)]
pub struct Tick {
    /// Calls so far (this one included).
    pub n: u64,
    /// This call says the hint: the count reached `FIRST` and no other call said it yet.
    pub say: bool,
}

fn age(f: &Path, now: u64) -> Option<u64> {
    let m = std::fs::metadata(f).ok()?.modified().ok()?;
    let t = m.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some(now.saturating_sub(t))
}

/// Deletes the state files nobody touched for `STALE_S`.
fn sweep(dir: &Path, now: u64) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if age(&e.path(), now).is_some_and(|a| a > STALE_S) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Counts this call of `rule` made by the process `parent` (its id and start, see `parent_tag`) at time `now` (seconds) in `dir`; `None` when the state
/// cannot be kept. The count is the length of an append-only file: every call writes one byte, so calls in parallel are all counted; a count above
/// `FIRST` says the hint if this call is the one that creates the marker file.
pub fn tick(dir: &Path, parent: &str, rule: &Rule, now: u64) -> Option<Tick> {
    std::fs::create_dir_all(dir).ok()?;
    let base = format!("{parent}-{}", rule.command.replace([' ', '.'], "_"));
    let (count, said) = (dir.join(&base), dir.join(format!("{base}.said")));
    if age(&count, now).is_some_and(|a| a > GAP_S) {
        let _ = std::fs::remove_file(&count);
        let _ = std::fs::remove_file(&said);
    }
    if !count.exists() {
        sweep(dir, now);
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&count)
        .ok()?;
    f.write_all(b".").ok()?;
    let n = f.metadata().ok()?.len();
    let say = n >= FIRST
        && std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&said)
            .is_ok();
    Some(Tick { n, say })
}

/// The terminal line of the hint for the `n`-th call of `rule`: its lang key's text, with the key at the end.
pub fn line(rule: &Rule, n: u64) -> String {
    crate::error_text::warning(&Notice {
        key: rule.key,
        params: vec![
            ("command", rule.command.to_string()),
            ("count", n.to_string()),
            ("find", rule.find.to_string()),
        ],
    })
}

/// The parent process as a name for its state: the id and, where the system tells it, the moment it started, so that a later process that gets the
/// same id has a count of its own.
#[cfg(unix)]
fn parent_tag() -> String {
    let id = std::os::unix::process::parent_id();
    let started = std::fs::read_to_string(format!("/proc/{id}/stat"))
        .ok()
        .and_then(|t| {
            let (_, rest) = t.rsplit_once(')')?;
            rest.split_whitespace().nth(19).map(String::from)
        })
        .unwrap_or_else(|| "0".into());
    format!("{id}-{started}")
}

fn state_dir() -> PathBuf {
    std::env::var_os("MZLAB_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("mzlab-hints"))
}

/// The hint line for this process's command line, or `None` (not a looped command, not due, silenced, or no place to count).
pub fn hint(args: &[String]) -> Option<String> {
    if std::env::var_os("MZLAB_NO_HINTS").is_some() {
        return None;
    }
    #[cfg(unix)]
    {
        let rule = rule_of(args)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        let t = tick(&state_dir(), &parent_tag(), rule, now)?;
        t.say.then(|| format!("{}\n", line(rule, t.n)))
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        None
    }
}

#[cfg(test)]
#[path = "repeat_hint_tests.rs"]
mod tests;
