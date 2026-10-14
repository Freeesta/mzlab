//! Provenance of an exported table (schema `provenienza/1`): the one place that decides which fields the first
//! line `# mzLab x.y.z · key=value · ...` of a CSV/TSV (or the "Provenance" sheet of an .xlsx) carries.
//! The page sends what it knows about the export as `key<TAB>value` lines, in the order of the line; the version is the crate's own.

macro_rules! schema {
    () => {
        "provenienza/1"
    };
}

pub const SCHEMA: &str = schema!();

/// `ctx` = the fields already written as ` · key=value` pieces in the order of the line (the route in `lr-map.js` trims the
/// values and drops the empty ones, the control characters and the dot `·`, so a value can never break the line).
/// Returns the schema name, a newline and the line `# mzLab x.y.z · key=value · ...`.
pub fn build(ctx: &str) -> String {
    let mut line = String::from(concat!(schema!(), "\n# mzLab ", env!("CARGO_PKG_VERSION")));
    line.push_str(ctx);
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_has_schema_version_and_fields() {
        let out = build(" · file=a.mzML · snip=1 min");
        let (schema, line) = out.split_once('\n').unwrap();
        assert_eq!(schema, SCHEMA);
        assert_eq!(
            line,
            format!(
                "# mzLab {} · file=a.mzML · snip=1 min",
                env!("CARGO_PKG_VERSION")
            )
        );
        assert_eq!(build("").lines().count(), 2);
    }
}
