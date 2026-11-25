//! The hint of a loop of calls (AGENT-DISCOVERY): a command that has a one-call equivalent for many targets, started over and over by the same parent
//! process (a script, a shell loop), says on stderr which one call does the whole list. The count lives in a small file per parent process in the temp
//! folder; stdout and the exit code never change, and `MZLAB_NO_HINTS=1` silences it. Called by the binary (`main.rs`), never by `execute`.

use std::path::{Path, PathBuf};

use crate::convert::Notice;

/// Calls from one parent after which the hint is said once, and again at every multiple of `EVERY`.
pub const FIRST: u64 = 10;
pub const EVERY: u64 = 1000;
/// A pause longer than this (seconds) between two calls starts the count again.
pub const GAP_S: u64 = 600;

/// One looped command and the call that replaces the loop.
pub struct Rule {
    pub command: &'static str,
    /// What to run instead (without `mzlab-cli`).
    pub bulk: &'static str,
    /// Words for `commands --find`.
    pub find: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        command: "peak auto-lr",
        bulk: "quant targets FILE... --targets TARGETS.csv (an m/z and RT window per row, every file, area and S N, one call)",
        find: "many targets files area",
    },
    Rule {
        command: "tp.dig_lr",
        bulk: "one flow run of one plan: tp.dig_lr digs the whole series in one run, every file in index (recipes.md)",
        find: "dig kinetics series",
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
        _ => None,
    }
}

/// Counts this call of `rule` made by `parent` at time `now` (seconds) in `dir`; the number of calls so far, or `None` when the state cannot be kept.
pub fn tick(dir: &Path, parent: u32, rule: &Rule, now: u64) -> Option<u64> {
    std::fs::create_dir_all(dir).ok()?;
    let file = dir.join(format!(
        "{parent}-{}",
        rule.command.replace([' ', '.'], "_")
    ));
    let (n, last) = std::fs::read_to_string(&file)
        .ok()
        .and_then(|t| {
            let mut w = t.split_whitespace().map(|w| w.parse::<u64>().ok());
            Some((w.next()??, w.next()??))
        })
        .unwrap_or((0, now));
    let n = if now.saturating_sub(last) > GAP_S {
        1
    } else {
        n + 1
    };
    std::fs::write(&file, format!("{n} {now}")).ok()?;
    Some(n)
}

/// Whether the `n`-th call says the hint.
pub fn due(n: u64) -> bool {
    n == FIRST || (n > FIRST && n.is_multiple_of(EVERY))
}

/// The terminal line of the hint for the `n`-th call of `rule`.
pub fn line(rule: &Rule, n: u64) -> String {
    crate::error_text::warning(&Notice {
        key: "note.cli.repeat",
        params: vec![
            ("command", rule.command.to_string()),
            ("count", n.to_string()),
            ("bulk", rule.bulk.to_string()),
            ("find", rule.find.to_string()),
        ],
    })
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
        let n = tick(&state_dir(), std::os::unix::process::parent_id(), rule, now)?;
        due(n).then(|| format!("{}\n", line(rule, n)))
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
