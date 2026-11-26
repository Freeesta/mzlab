//! Task words, `see_also` and bulk equivalents of commands and Flow steps: what `commands --find` matches beyond the summaries, and what
//! `commands --json PATH` adds to a command. Written from an audit of agent tasks in a low-resolution LC-MS campaign (the fixtures of
//! `find_tests.rs`): every row names a command path or a step id that exists (`find_tests.rs` checks it).

pub(crate) struct Rel {
    /// A command path (`peak table`) or a Flow step id (`chrom.transitions`).
    pub name: &'static str,
    /// Words of the tasks it does that its summary does not say.
    pub words: &'static str,
    /// Related commands or steps.
    pub also: &'static [&'static str],
    /// The one call to use instead of many calls of this one; empty when there is none.
    pub bulk: &'static str,
    /// The command line `commands --find` shows; empty: the first example of the command, or the template line of the step.
    pub example: &'static str,
}

pub(super) const fn r(
    name: &'static str,
    words: &'static str,
    also: &'static [&'static str],
) -> Rel {
    Rel {
        name,
        words,
        also,
        bulk: "",
        example: "",
    }
}

impl Rel {
    pub(super) const fn bulk(mut self, b: &'static str) -> Self {
        self.bulk = b;
        self
    }
    pub(super) const fn ex(mut self, e: &'static str) -> Self {
        self.example = e;
        self
    }
    /// Whether `bulk` is another command (`mzlab-cli OTHER ...`) rather than this one's own batch use (`trace xic --mz M1,M2`).
    pub(crate) fn elsewhere(&self) -> bool {
        self.bulk
            .strip_prefix("mzlab-cli ")
            .is_some_and(|b| !b.starts_with(&format!("{} ", self.name)))
    }
}

pub(crate) use super::related_rows::RELATED;

/// The row of `name`, if it has one.
pub(crate) fn of(name: &str) -> Option<&'static Rel> {
    RELATED.iter().find(|r| r.name == name)
}
