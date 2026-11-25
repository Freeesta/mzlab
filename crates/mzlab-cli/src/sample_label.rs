//! The label of a sample of a multi-sample file (`features --sample`): the column of the table. The sample name, never the file name, with the extension of
//! its file kept so that every table cuts that and not a dot of the name; the characters `/ \ : * ? " < > | #` become `_` and the name is cut at 120 characters, as the site does; members whose names collide take `NAME (FILE #ID)`. Pure, so the Flow uses the same rule.

/// One member of a series taken from a multi-sample file: its file name, sample id and sample name.
pub(crate) struct Member<'a> {
    pub file: &'a str,
    pub id: u32,
    pub sample: &'a str,
}

fn extension(file: &str) -> &str {
    file.rsplit_once('.')
        .filter(|(head, _)| !head.is_empty())
        .map_or("", |(_, e)| e)
}

fn base(file: &str) -> &str {
    file.rsplit_once('.')
        .filter(|(head, _)| !head.is_empty())
        .map_or(file, |(h, _)| h)
}

fn with_extension(name: &str, file: &str) -> String {
    match extension(file) {
        "" => name.to_string(),
        e => format!("{name}.{e}"),
    }
}

/// A sample name as a label: trimmed, the characters a file name cannot hold (and a path separator, which every reader of a table takes for a folder) become `_`, at
/// most 120 characters: the rule of the site for the mzML of a sample (`wiffName` of `conversione.js`; `tests/test_label_parity.py` runs both on the same names).
fn clean(name: &str) -> String {
    name.trim()
        .chars()
        .map(|c| if "/\\:*?\"<>|#".contains(c) { '_' } else { c })
        .take(120)
        .collect()
}

/// The labels of `members` (one per member, in order) and whether each had to take the tie-break (`NAME (FILE #ID)`). An empty sample name is `FILE #ID`.
pub(crate) fn labels(members: &[Member]) -> Vec<(String, bool)> {
    let plain: Vec<String> = members
        .iter()
        .map(|m| {
            let n = clean(m.sample);
            if n.is_empty() {
                format!("{} #{}", base(m.file), m.id)
            } else {
                n
            }
        })
        .collect();
    let mut out: Vec<(String, bool)> = members
        .iter()
        .zip(&plain)
        .map(|(m, p)| {
            let clash = plain.iter().filter(|q| q.as_str() == p).count() > 1;
            if clash {
                (
                    with_extension(&format!("{p} ({} #{})", base(m.file), m.id), m.file),
                    true,
                )
            } else {
                (with_extension(p, m.file), false)
            }
        })
        .collect();
    // the same file given twice (two folders): the later ones are numbered, so no two columns share a name
    for k in 1..out.len() {
        let seen = out[..k].iter().filter(|(n, _)| *n == out[k].0).count();
        if seen > 0 {
            let (n, tie) = &out[k];
            let name = n
                .rsplit_once('.')
                .map_or(format!("{n} [{}]", seen + 1), |(h, e)| {
                    format!("{h} [{}].{e}", seen + 1)
                });
            out[k] = (name, *tie);
        }
    }
    out
}

#[cfg(test)]
#[path = "sample_label_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "features_label_tests.rs"]
mod cli_tests;
