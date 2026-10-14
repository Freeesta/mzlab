//! Provenance of an exported table (schema `provenienza/1`): the one place that decides which fields the first
//! line `# mzLab x.y.z · key=value · ...` of a CSV/TSV (or the "Provenance" sheet of an .xlsx) carries.
//! The page sends what it knows about the export as `key<TAB>value` lines, in the order of the line; the version is the crate's own.

pub const SCHEMA: &str = "provenienza/1";

/// `ctx` = the fields as lines `key<TAB>value`, already in the order of the line, trimmed, without empty values, control
/// characters or the dot `·` (the route in `lr-map.js` cleans them, so a value can never break the line).
/// Returns the schema name, a newline and the line `# mzLab x.y.z · key=value · ...`.
pub fn build(ctx: &str) -> String {
    let mut line = String::from(SCHEMA);
    line.push_str("\n# mzLab ");
    line.push_str(env!("CARGO_PKG_VERSION"));
    for (k, v) in ctx.lines().filter_map(|l| l.split_once('\t')) {
        line.push_str(" · ");
        line.push_str(k);
        line.push('=');
        line.push_str(v);
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_has_version_and_fields_in_order() {
        let out = build("file\ta.mzML\nsnip\t1 min\nsmooth\ton");
        let (schema, line) = out.split_once('\n').unwrap();
        assert_eq!(schema, SCHEMA);
        assert_eq!(
            line,
            format!(
                "# mzLab {} · file=a.mzML · snip=1 min · smooth=on",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[test]
    fn empty_and_odd_input() {
        let bare = format!("{SCHEMA}\n# mzLab {}", env!("CARGO_PKG_VERSION"));
        assert_eq!(build(""), bare);
        assert_eq!(build("no tab here\n"), bare);
    }
}
