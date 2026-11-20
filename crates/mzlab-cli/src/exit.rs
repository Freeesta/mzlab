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
/// An input that is still being written (a `.wiff` pair whose size changes): nothing was read; try again when the acquisition has ended.
pub const GROWING: u8 = 8;

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
    "err.mol.draw.not_found",
    "err.mol.draw.not_found_bare",
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
    "err.llm.eval.case",
    "err.llm.replay.format",
    "err.normative.pin",
    "err.normative.sources",
];
const DATA_KEYS: &[&str] = &[
    "err.pda.none",
    "err.flow.cancelled",
    "err.run.data",
    "err.llm.eval.loop",
    "err.llm.replay.mismatch",
    "err.llm.replay.exhausted",
];

/// The chemistry keys that are not usage, one rule: content that cannot be read (a request, a molfile, a structure list) is input (4); a valid
/// request the chemistry cannot satisfy, or one over a size limit (`*.tooBig`, `error.sub.dbSize`), is data (1); formula, adduct and SMILES
/// syntax of an argument or option falls through to usage (2).
fn chem_code(key: &str) -> Option<u8> {
    const CHEM_DATA_KEYS: &[&str] = &[
        "err.formula.mzLow",
        "err.formula.noIon",
        "err.iso.element",
        "err.iso.adductTooMany",
        "error.grafo.delta.neg",
        "error.grafo.tooBig",
        "error.grafo.element",
        "err.mol.draw.element",
        "error.grafo.mixture",
        "error.grafo.ring",
        "error.grafo.mech.atom",
        "error.grafo.cut.bond",
        "error.grafo.cut.aromatic",
        "error.grafo.cut.order",
        "error.grafo.cut.radical",
        "error.check.tooBig",
        "error.sdf.tooBig",
        "error.open.tooBig",
        "error.sub.dbSize",
        "error.anal.abundance",
        "error.anal.empty",
        "error.hilite.query",
        "err.mol.draw.tooBig",
    ];
    const CHEM_INPUT_KEYS: &[&str] = &[
        "error.label.request",
        "error.anal.request",
        "error.check.request",
        "error.check.molfile",
        "error.cip.request",
        "error.cip.molfile",
        "error.open.request",
        "error.hilite.request",
        "error.hilite.molfile",
        "error.link.request",
        "error.sel.request",
        "error.sdf.molfile",
        "error.grafo.molfile",
        "error.grafo.mech.request",
        "error.sub.db",
        "error.sub.column",
        "err.search.request",
        "err.mol.draw.db",
        "err.mol.draw.molfile",
        "err.mol.draw.request",
    ];
    if CHEM_DATA_KEYS.contains(&key) {
        Some(DATA)
    } else if CHEM_INPUT_KEYS.contains(&key) {
        Some(INPUT)
    } else {
        None
    }
}

/// The exit code of the engine error `key`: the one mapping of keys onto the table (exact keys first, then prefixes). A key that names no
/// class is the engine refusing what the command line asked (a formula, a molecule, a setting it cannot use): usage (2). The errors about
/// the data a command read (nothing overlaps, a run cancelled) are listed as data (1).
pub fn code_of_key(key: &str) -> u8 {
    match key {
        "err.run.integrity" => return INTEGRITY,
        "err.run.input" => return INPUT,
        "err.run.selector" => return SELECTOR,
        "err.run.output" => return OUTPUT,
        _ => {}
    }
    if key == "err.file.wiffGrowing" {
        GROWING
    } else if INPUT_KEYS.contains(&key)
        || key.starts_with("err.file.")
        || chem_code(key) == Some(INPUT)
    {
        INPUT
    } else if OUTPUT_KEYS.contains(&key) {
        OUTPUT
    } else if SELECTOR_KEYS.contains(&key) {
        SELECTOR
    } else if DATA_KEYS.contains(&key) || chem_code(key) == Some(DATA) {
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
        GROWING => "err.file.wiffGrowing",
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
        assert_eq!(code_of_key("err.file.wiffGrowing"), GROWING);
        for c in [DATA, USAGE, INTEGRITY, INPUT, SELECTOR, OUTPUT, GROWING] {
            assert_eq!(code_of_key(generic_key(c)), c);
        }
        assert_eq!(code_of_key("err.flow.compare.few"), USAGE);
        assert_eq!(code_of_key("err.pda.none"), DATA);
    }

    #[test]
    fn chemistry_keys_split_syntax_impossible_and_unreadable() {
        for k in [
            "err.formula.unreadable",
            "err.formula.element",
            "err.adduct.unknown",
            "error.grafo.smiles",
            "error.grafo.cut.mode",
            "error.sub.query",
        ] {
            assert_eq!(code_of_key(k), USAGE, "{k}");
        }
        for k in [
            "err.formula.mzLow",
            "error.grafo.cut.aromatic",
            "error.hilite.query",
            "error.grafo.cut.bond",
            "err.iso.adductTooMany",
            "error.sub.dbSize",
            "error.grafo.tooBig",
        ] {
            assert_eq!(code_of_key(k), DATA, "{k}");
        }
        for k in [
            "error.label.request",
            "error.grafo.molfile",
            "error.sdf.molfile",
            "error.check.molfile",
            "err.search.request",
            "error.sub.db",
        ] {
            assert_eq!(code_of_key(k), INPUT, "{k}");
        }
    }
}
