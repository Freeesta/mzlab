//! The one table of exit codes of `mzlab-cli` and the one function that maps an engine error key onto it.
//! Every command, `error/1` and the docs take their codes from here.

/// Everything went well.
pub const OK: u8 = 0;
/// A data or domain error; outputs that could be written were written.
pub const DATA: u8 = 1;
/// A usage error: unknown command or option, a value that does not parse.
pub const USAGE: u8 = 2;
/// A plan, schema or integrity error (a plan that is not valid, a fingerprint that does not match).
pub const INTEGRITY: u8 = 3;
/// An input that is missing or unreadable.
pub const INPUT: u8 = 4;
/// A selector (sample, channel, node, step instance) that names nothing.
pub const SELECTOR: u8 = 5;
/// An output that cannot be written, or an external source that failed.
pub const OUTPUT: u8 = 6;

/// The exit code of the engine error `key`: the one mapping of keys onto the table (exact keys first, then prefixes). A key that names no
/// class is the engine refusing what the command line asked (a formula, a molecule, a setting it cannot use): usage (2). The errors about
/// the data a command read (nothing overlaps, a run cancelled) are listed as data (1).
pub fn code_of_key(key: &str) -> u8 {
    const INPUT_KEYS: &[&str] = &[
        "err.cli.input.missing",
        "err.cli.input.twice",
        "err.cli.file",
        "err.flow.io",
        "err.flow.input.missing",
        "err.flow.input.sha",
        "err.session.fileNotFound",
        "err.normative.input",
        "err.msn.rules.document",
        "error.badIndex",
    ];
    const OUTPUT_KEYS: &[&str] = &[
        "err.cli.out",
        "err.normative.io",
        "err.normative.fetch",
        "err.normative.large",
        "err.normative.parse",
    ];
    const SELECTOR_KEYS: &[&str] = &[
        "err.sample.unknown",
        "err.canali.unknown",
        "err.scan.unknown",
        "err.msn.node",
        "err.flow.step.unknown",
        "err.cli.command.unknown",
        "err.cli.schema.unknown",
        "err.flow.instance",
        "err.llm.tool.unknown",
    ];
    const INTEGRITY_PREFIXES: &[&str] = &[
        "err.flow.format",
        "err.flow.json",
        "err.flow.check",
        "err.flow.lineage",
        "err.flow.port.",
        "err.flow.param.unknown",
        "err.flow.algorithm.unknown",
        "err.flow.preset.unknown",
        "err.flow.ref.",
        "err.flow.instance.",
        "err.flow.step",
        "err.flow.input.",
        "err.flow.unknownStep",
        "err.mzrun.",
        "err.design.",
        "err.normative.pin",
        "err.normative.sources",
    ];
    const DATA_KEYS: &[&str] = &["err.pda.none", "err.flow.cancelled", "err.run.data"];
    match key {
        "err.run.integrity" => return INTEGRITY,
        "err.run.input" => return INPUT,
        "err.run.selector" => return SELECTOR,
        "err.run.output" => return OUTPUT,
        _ => {}
    }
    if INPUT_KEYS.contains(&key) || key.starts_with("err.file.") {
        INPUT
    } else if OUTPUT_KEYS.contains(&key) {
        OUTPUT
    } else if SELECTOR_KEYS.contains(&key) {
        SELECTOR
    } else if DATA_KEYS.contains(&key) {
        DATA
    } else if INTEGRITY_PREFIXES.iter().any(|p| key.starts_with(p)) {
        INTEGRITY
    } else {
        USAGE
    }
}

/// The generic key (`err.run.*`) for a command that failed with `code` and gave no key of its own: it maps back onto the same code.
pub fn generic_key(code: u8) -> &'static str {
    match code {
        INTEGRITY => "err.run.integrity",
        INPUT => "err.run.input",
        SELECTOR => "err.run.selector",
        OUTPUT => "err.run.output",
        USAGE => "err.cli.usage",
        _ => "err.run.data",
    }
}

/// The exit code of `e`.
pub fn code_of(e: &mzlab_core::Error) -> u8 {
    code_of_key(e.key)
}

/// For the tests of usage errors: the code a list of arguments earns, 4 when one names a path that is not there (`/no/such`, `/nonexistent`), else 2.
#[cfg(test)]
pub(crate) fn usage_or_missing(a: &[&str]) -> u8 {
    if a.iter().any(|x| x.starts_with("/no")) {
        INPUT
    } else {
        USAGE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_onto_the_table() {
        assert_eq!(code_of_key("err.file.cutShort"), INPUT);
        assert_eq!(code_of_key("err.sample.unknown"), SELECTOR);
        assert_eq!(code_of_key("err.flow.step.unknown"), SELECTOR);
        assert_eq!(code_of_key("err.flow.param.unknown"), INTEGRITY);
        assert_eq!(code_of_key("err.flow.param.range"), USAGE);
        assert_eq!(code_of_key("err.cli.set"), USAGE);
        assert_eq!(code_of_key("err.cli.file"), INPUT);
        assert_eq!(code_of_key("err.cli.out"), OUTPUT);
        assert_eq!(code_of_key("err.normative.input"), INPUT);
        assert_eq!(code_of_key("err.flow.cancelled"), DATA);
        for c in [DATA, USAGE, INTEGRITY, INPUT, SELECTOR, OUTPUT] {
            assert_eq!(code_of_key(generic_key(c)), c);
        }
        assert_eq!(code_of_key("err.flow.compare.few"), USAGE);
        assert_eq!(code_of_key("err.pda.none"), DATA);
    }
}
