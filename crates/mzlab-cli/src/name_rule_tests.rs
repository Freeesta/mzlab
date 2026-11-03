//! The naming grammar of `docs/agenti/cli.md` («Names») as tests: every command, subcommand and flag printed by a `--help`, every Flow step id and every
//! step parameter follows the grammar, the length targets and the English-only rule, but for a closed list of known exceptions that can only get shorter.

use std::collections::BTreeSet;

use crate::{execute, flow::steps};

/// Names that break a rule today and stay on purpose (`docs/studi/NOMI_TERMINALE.md` §4 «Kept on purpose», §5). A name leaves this list when it is
/// renamed; nothing is added.
const KNOWN: &[&str] = &[
    // §4: a shorter name would need the reader to guess («missing» alone loses the meaning)
    "explain-missing",
    // §2.1: the subcommand that takes the official lists the user owns; a shorter verb would hide what is imported
    "import-official",
    // §2.3 and D1: persisted `spectra.match` parameter, Italian until a step version bump
    "min_prove",
    // §2.3 and D2: persisted parameters of the Attributio annotation steps, long but clear (not in the inventory)
    "putative_library_level3",
    "min_replicate_fraction",
    "library_conflict_margin",
    "suspect_fragments_min",
];

/// Roots of Italian words met in the inventory (§2): no name contains a word that starts with one of them.
const ITALIAN: &[&str] = &[
    "basso", "prove", "toll", "perdita", "regole", "rotture", "legami", "vista", "figura", "canal",
    "campion", "somma", "tutti",
];

/// Every command path whose `--help` is checked; the first word is a command (≤ 12), the others subcommands (≤ 8).
const COMMANDS: &[&[&str]] = &[
    &[],
    &["features"],
    &["flow", "run"],
    &["flow", "check"],
    &["steps"],
    &["llm"],
    &["msn-graph"],
    &["msn-lib"],
    &["msn-rules"],
    &["channels"],
    &["ion-pattern"],
    &["samples"],
    &["peak-edges"],
    &["explain-missing"],
    &["convert"],
    &["validate"],
    &["normative"],
    &["insilico"],
];

/// Subcommands that have no help of their own: they are named in the usage lines of their command.
const SUBCOMMANDS: &[&[&str]] = &[
    &["flow", "run"],
    &["flow", "check"],
    &["flow", "report"],
    &["flow", "record"],
    &["flow", "verify"],
    &["flow", "command"],
    &["llm", "tools"],
    &["llm", "replay"],
    &["llm", "context"],
    &["llm", "eval"],
    &["msn-graph", "figure"],
    &["msn-graph", "complete"],
    &["msn-graph", "compare"],
    &["msn-lib", "add"],
    &["msn-lib", "build"],
    &["msn-lib", "export"],
    &["msn-lib", "search"],
    &["msn-rules", "sum"],
    &["samples", "vars"],
    &["samples", "group"],
    &["normative", "pack"],
    &["normative", "listings"],
    &["normative", "import-official"],
    &["normative", "build"],
    &["insilico", "pack"],
];

fn kebab(name: &str) -> bool {
    let mut parts = name.split('-');
    let first = parts.next().unwrap_or("");
    let ok = |p: &str, lead: bool| {
        !p.is_empty()
            && p.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            && (!lead || p.as_bytes()[0].is_ascii_lowercase())
    };
    ok(first, true) && parts.all(|p| ok(p, false))
}

fn snake(name: &str) -> bool {
    let mut parts = name.split('_');
    let first = parts.next().unwrap_or("");
    let ok = |p: &str, lead: bool| {
        !p.is_empty()
            && p.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            && (!lead || p.as_bytes()[0].is_ascii_lowercase())
    };
    ok(first, true) && parts.all(|p| ok(p, false))
}

/// A step id is `family.noun` or `family.noun_noun`, ≤ 20 characters.
fn step_id_ok(id: &str) -> bool {
    id.len() <= 20
        && id.split_once('.').is_some_and(|(family, noun)| {
            !family.is_empty() && family.bytes().all(|b| b.is_ascii_lowercase()) && snake(noun)
        })
}

/// An Italian root at the start of a word of `name` (words split at `-`, `_` and `.`).
fn italian_root(name: &str) -> Option<&'static str> {
    name.split(['-', '_', '.'])
        .find_map(|w| ITALIAN.iter().find(|r| w.starts_with(**r)).copied())
}

/// Why `flag` (with its dashes) breaks the grammar, the length target (≤ 14 without the dashes) or the English-only rule.
fn flag_problem(flag: &str) -> Option<String> {
    let bare = flag.trim_start_matches('-');
    let dashes = flag.len() - bare.len();
    if !(1..=2).contains(&dashes) || !kebab(bare) {
        return Some(format!("{flag}: not kebab-case"));
    }
    if bare.len() > 14 {
        return Some(format!("{flag}: longer than 14"));
    }
    italian_root(bare).map(|r| format!("{flag}: Italian root «{r}»"))
}

/// Why a command (`depth` 0) or subcommand (`depth` 1) breaks the grammar or the length targets (≤ 12, ≤ 8).
fn command_problem(name: &str, depth: usize) -> Option<String> {
    let limit = if depth == 0 { 12 } else { 8 };
    if !kebab(name) {
        return Some(format!("{name}: not kebab-case"));
    }
    if name.len() > limit {
        return Some(format!("{name}: longer than {limit}"));
    }
    italian_root(name).map(|r| format!("{name}: Italian root «{r}»"))
}

fn step_problem(id: &str) -> Option<String> {
    if !step_id_ok(id) {
        return Some(format!("{id}: not family.noun[_noun] within 20"));
    }
    italian_root(id).map(|r| format!("{id}: Italian root «{r}»"))
}

fn param_problem(name: &str) -> Option<String> {
    if !snake(name) || name.len() > 20 {
        return Some(format!("{name}: not snake_case within 20"));
    }
    italian_root(name).map(|r| format!("{name}: Italian root «{r}»"))
}

/// The flags a help text prints, without the lines of «old names still accepted» (the only place an old name may appear).
fn printed_flags(help: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for line in help
        .lines()
        .filter(|l| !l.starts_with("old names still accepted"))
    {
        let b = line.as_bytes();
        for (i, &c) in b.iter().enumerate() {
            let boundary =
                i == 0 || matches!(b[i - 1], b' ' | b'[' | b'(' | b'|' | b',' | b'{' | b'`');
            if c != b'-' || !boundary || (i > 0 && b[i - 1] == b'-') {
                continue;
            }
            let dashes = if b.get(i + 1) == Some(&b'-') { 2 } else { 1 };
            let rest = &line[i + dashes..];
            if !rest.starts_with(|ch: char| ch.is_ascii_alphabetic()) {
                continue;
            }
            let end = rest
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
                .unwrap_or(rest.len());
            found.insert(format!(
                "{}{}",
                &line[i..i + dashes],
                rest[..end].trim_end_matches('-')
            ));
        }
    }
    found
}

fn help_of(path: &[&str]) -> String {
    let mut args: Vec<String> = path.iter().map(|s| s.to_string()).collect();
    args.push("--help".into());
    let out = execute(&args);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), out.stderr)
}

/// Every name the rules look at, as `(problem)` lines; `KNOWN` names are left out and reported separately in `seen`.
fn all_problems() -> (Vec<String>, BTreeSet<String>) {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    let mut note = |name: &str, p: Option<String>| {
        if let Some(p) = p {
            if KNOWN.contains(&name) {
                seen.insert(name.to_string());
            } else {
                problems.push(p);
            }
        }
    };
    for path in COMMANDS.iter().chain(SUBCOMMANDS) {
        for (depth, word) in path.iter().enumerate() {
            note(word, command_problem(word, depth.min(1)));
        }
    }
    for path in COMMANDS {
        let help = help_of(path);
        assert!(help.contains("mzlab-cli"), "no help for {path:?}");
        for flag in printed_flags(&help) {
            note(&flag, flag_problem(&flag));
        }
    }
    for step in steps() {
        note(step.id, step_problem(step.id));
        let own = step.params.iter();
        let of_alg = step.algorithms.iter().flat_map(|a| a.params.iter());
        for p in own.chain(of_alg) {
            note(p.name, param_problem(p.name));
        }
    }
    (problems, seen)
}

#[test]
fn every_printed_name_follows_the_grammar() {
    let (problems, _) = all_problems();
    assert!(
        problems.is_empty(),
        "names that break «Names» in cli.md:\n{}",
        problems.join("\n")
    );
}

#[test]
fn known_exceptions_still_break_a_rule() {
    let (_, seen) = all_problems();
    for name in KNOWN {
        assert!(
            seen.contains(*name),
            "{name} follows the rules now: drop it from KNOWN"
        );
    }
}

#[test]
fn the_help_prints_the_new_msn_rules_subcommand() {
    assert!(help_of(&[]).contains("msn-rules RECORD... | sum LINES..."));
}

// A negative case per rule: a name that breaks it is reported.

#[test]
fn a_flag_that_is_too_long_fails() {
    assert!(flag_problem("--troppo-lungo-davvero")
        .unwrap()
        .contains("longer than 14"));
    assert!(flag_problem("--out").is_none());
}

#[test]
fn a_flag_that_is_not_kebab_case_fails() {
    assert!(flag_problem("--areaNoise").is_some());
    assert!(flag_problem("--min_points").is_some());
    assert!(flag_problem("---x").is_some());
}

#[test]
fn a_command_over_its_length_target_fails() {
    assert!(command_problem("explain-missing", 0).is_some());
    assert!(command_problem("import-official", 1).is_some());
    assert!(command_problem("msn-graph", 0).is_none());
    assert!(command_problem("Flow", 0).is_some());
}

#[test]
fn a_step_id_off_the_pattern_or_too_long_fails() {
    assert!(step_problem("tp.dig_lr").is_none());
    assert!(step_problem("structure.reactivity").is_none());
    assert!(step_problem("Tp.generate").is_some());
    assert!(step_problem("tp").is_some());
    assert!(step_problem("tp.dig-lr").is_some());
    assert!(step_problem("structure.reactivityy").is_some());
}

#[test]
fn a_parameter_off_snake_case_or_too_long_fails() {
    assert!(param_problem("min_points").is_none());
    assert!(param_problem("areaNoise").is_some());
    assert!(param_problem("a_very_long_parameter_name").is_some());
}

#[test]
fn an_italian_root_in_any_kind_of_name_fails() {
    assert!(flag_problem("--mz-basso").unwrap().contains("basso"));
    assert!(param_problem("prove_min").unwrap().contains("prove"));
    assert!(step_problem("tp.somma").is_some());
    assert!(command_problem("canali", 0).is_some());
    // a word that merely contains a root is fine
    assert!(param_problem("configuration").is_none());
}

#[test]
fn old_names_are_only_allowed_on_the_old_names_line() {
    let flags = printed_flags("  --out PATH\nold names still accepted: --canale, --jsonl");
    assert_eq!(flags.into_iter().collect::<Vec<_>>(), ["--out"]);
    let flags = printed_flags("  --canale ID  the channel");
    assert!(flag_problem(flags.iter().next().unwrap()).is_some());
}
