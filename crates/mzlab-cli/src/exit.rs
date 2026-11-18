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

/// The exit code of the engine error `key`: the table above, by key.
pub fn code_of_key(key: &str) -> u8 {
    match key {
        "err.cli.input.missing" | "err.cli.file" | "err.flow.io" | "error.badIndex" => INPUT,
        "err.cli.out" | "err.flow.cancelled" => OUTPUT,
        "err.sample.unknown" | "err.canali.unknown" => SELECTOR,
        k if k.starts_with("err.cli.") => USAGE,
        k if k.starts_with("err.file.") => INPUT,
        _ => DATA,
    }
}

/// The exit code of `e`.
pub fn code_of(e: &mzlab_core::Error) -> u8 {
    code_of_key(e.key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_map_onto_the_table() {
        assert_eq!(code_of_key("err.file.cutShort"), INPUT);
        assert_eq!(code_of_key("err.sample.unknown"), SELECTOR);
        assert_eq!(code_of_key("err.cli.set"), USAGE);
        assert_eq!(code_of_key("err.cli.file"), INPUT);
        assert_eq!(code_of_key("err.flow.compare.few"), DATA);
    }
}
