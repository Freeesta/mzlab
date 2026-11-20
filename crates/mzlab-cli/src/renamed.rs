//! The command names of earlier builds and what they are called now (the noun-verb tree: `channels` is `file channels`). Typing an old name is exit 2
//! with the new name in the message and in the `hint` of `error/1`, so an agent that learned the old tree finds the new one without a catalog.
//! The same table writes the list of `docs/agenti/cli-agenti.md` (`help-md renames`); `tests` check that every target is a real command.

/// `(old name, new command path)`; an empty path says the command was removed (with its reason in `REMOVED`).
pub(crate) static RENAMED: &[(&str, &str)] = &[
    ("background", "trace background"),
    ("channels", "file channels"),
    ("convert", "file convert"),
    ("dda-window", "spectrum dda-window"),
    ("elements", "ion elements"),
    ("folder", "file list"),
    ("insilico", "db insilico pack"),
    ("ion-mz", "ion mz"),
    ("ion-pattern", "ion pattern"),
    ("items", "file items"),
    ("manual-table", "peak table"),
    ("match-mz", "ion match"),
    ("mol-anal", "mol anal"),
    ("mol-check", "mol check"),
    ("mol-cip", "mol cip"),
    ("mol-cut", "mol cut"),
    ("mol-export", "mol export"),
    ("mol-hilite", "mol hilite"),
    ("mol-label", "mol label"),
    ("mol-link", "mol link"),
    ("mol-mech", "mol mech"),
    ("mol-search", "mol search"),
    ("mol-sel", "mol sel"),
    ("msn-graph", "msn graph"),
    ("msn-lib", "msn lib"),
    ("msn-rules", "msn rules"),
    ("normative", "db normative"),
    ("pair", "file pair"),
    ("peak-area", "peak area"),
    ("peak-auto", "peak auto"),
    ("peak-auto-lr", "peak auto-lr"),
    ("peak-edges", "peak edges"),
    ("peak-split", "peak split"),
    ("peak-window", "peak window"),
    ("ref-list", "db ref-list"),
    ("samples", "sample"),
    ("steps", "flow steps"),
    ("validate", "file validate"),
    ("wiff", "file inventory"),
    ("explain-missing", ""),
];

/// Why a command with an empty path above is gone.
const REMOVED: &[(&str, &str)] = &[("explain-missing", "the Tabui tab it explained")];

/// The new path of the old name `word`, or the reason it was removed.
pub(crate) enum Fate {
    Now(&'static str),
    Removed(&'static str),
}

/// What became of the command an argument list starts with (`None` when it was never a command name).
pub(crate) fn fate(args: &[String]) -> Option<(&'static str, Fate)> {
    let word = args.first()?;
    let (old, now) = RENAMED.iter().find(|(old, _)| old == word)?;
    if now.is_empty() {
        let why = REMOVED.iter().find(|(o, _)| o == old)?.1;
        return Some((old, Fate::Removed(why)));
    }
    Some((old, Fate::Now(now)))
}

/// The failure of a first word that names no command: the new path of an old name or the reason it is gone, else plain «unknown command».
pub(crate) fn unknown_command(args: &[String]) -> crate::Outcome {
    match fate(args) {
        Some((old, fate)) => failure(old, &fate),
        None => crate::Outcome::fail_error(&mzlab_core::Error::new("err.cli.usage").with(
            "message",
            format!(
                "unknown command {}",
                args.first().map_or("", |a| a.as_str())
            ),
        )),
    }
}

/// The failure of a typed old name: exit 2, the new path in the message and the hint (`err.cli.command.renamed`, `err.cli.command.removed`).
pub(crate) fn failure(old: &str, fate: &Fate) -> crate::Outcome {
    use mzlab_core::Error;
    match fate {
        Fate::Now(now) => crate::Outcome::fail_error(
            &Error::new("err.cli.command.renamed")
                .with("old", old.to_string())
                .with("now", now.to_string()),
        )
        .with_hint(&format!("mzlab-cli {now} --help")),
        Fate::Removed(why) => crate::Outcome::fail_error(
            &Error::new("err.cli.command.removed")
                .with("old", old.to_string())
                .with("why", why.to_string()),
        )
        .with_hint("mzlab-cli commands --json lists the commands of this build"),
    }
}

/// The list of `docs/agenti/cli-agenti.md`: one line per old name, the new path or the removal.
pub(crate) fn list_md() -> String {
    let mut out = String::new();
    for (old, now) in RENAMED {
        if now.is_empty() {
            out += &format!("| `{old}` | removed (no replacement) |\n");
        } else {
            out += &format!("| `{old}` | `{now}` |\n");
        }
    }
    format!("| old name | now |\n|---|---|\n{out}")
}

#[cfg(test)]
#[path = "renamed_tests.rs"]
mod tests;
