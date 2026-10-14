//! Provenance of an exported table (schema `provenienza/1`): the one place that decides which fields the first
//! line `# mzLab x.y.z · key=value · ...` of a CSV/TSV (or the "Provenance" sheet of an .xlsx) carries.
//! The page sends what it knows about the export as `key<TAB>value` lines, in the order of the line; the version is the crate's own.

pub const SCHEMA: &str = "provenienza/1";

/// Control characters become spaces and the separator dot `·` becomes `-`, so a value can never break the line.
fn clean(s: &str, out: &mut String) {
    for c in s.chars() {
        out.push(match c {
            '·' => '-',
            c if c < ' ' || c == '\x7f' => ' ',
            c => c,
        });
    }
}

/// `ctx` = the fields as lines `key<TAB>value`, already in the order of the line (the page sends every value as text);
/// a line without a tab or with an empty value is left out; the page trims the values.
/// Returns the schema name, a newline and the line `# mzLab x.y.z · key=value · ...`.
pub fn build(ctx: &str) -> String {
    let mut line = String::from(SCHEMA);
    line.push_str("\n# mzLab ");
    line.push_str(env!("CARGO_PKG_VERSION"));
    for (k, s) in ctx.lines().filter_map(|l| l.split_once('\t')) {
        if !s.is_empty() {
            line.push_str(" · ");
            line.push_str(k);
            line.push('=');
            clean(s, &mut line);
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_has_version_and_fields_in_order() {
        let out = build("file\ta.mzML\nsnip\t1 min\nsmooth\ton\nnote\t");
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
    fn control_characters_and_separators_cannot_break_the_line() {
        let out = build("file\ta\u{b}b\u{c}c · d");
        assert_eq!(out.lines().count(), 2);
        assert!(out.ends_with("file=a b c - d"));
    }

    #[test]
    fn empty_and_odd_input() {
        let bare = format!("{SCHEMA}\n# mzLab {}", env!("CARGO_PKG_VERSION"));
        assert_eq!(build(""), bare);
        assert_eq!(build("no tab here\nfile\t\n"), bare);
    }
}
