//! `file convert --name sample` and the files already in `--out`: a sample name such as `Blank` comes back in every run, so a name that holds the mzML of another
//! input would be replaced silently. A small table in the work folder (`names.tsv`: file, input) records what each run wrote; a later run that would put a different
//! input on a recorded name, or on a file the table does not know, is refused (exit 2) unless `--fresh`.

use std::collections::HashMap;
use std::path::Path;

use crate::convert_entries::Entry;
use crate::convert_report::Item;

const TABLE: &str = "names.tsv";

/// What identifies the input of an entry: its path and the name of the plan input (`NAME@ID`).
fn key(e: &Entry) -> String {
    let path = e.input.canonicalize().unwrap_or_else(|_| e.input.clone());
    format!("{}\t{}", path.display(), e.label)
}

fn read(work: &Path) -> HashMap<String, String> {
    let text = std::fs::read_to_string(work.join(TABLE)).unwrap_or_default();
    text.lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(f, k)| (f.to_string(), k.to_string()))
        .collect()
}

/// The first file of `entries` that exists in `out` and does not come from the same input as an earlier run recorded.
pub(crate) fn taken_by_other(entries: &[Entry], out: &Path, work: &Path) -> Option<String> {
    let known = read(work);
    entries.iter().find_map(|e| {
        let to = out.join(&e.out);
        let mine = known.get(&e.out).is_some_and(|k| *k == key(e));
        (to.exists() && !mine).then(|| to.display().to_string())
    })
}

/// Records the files `entries` wrote (those whose `items` are done), keeping what earlier runs recorded.
pub(crate) fn record(entries: &[Entry], items: &[Item], work: &Path) {
    let mut known = read(work);
    for (e, _) in entries
        .iter()
        .zip(items)
        .filter(|(_, i)| matches!(i, Item::Done { .. }))
    {
        known.insert(e.out.clone(), key(e));
    }
    let mut rows: Vec<_> = known.into_iter().collect();
    rows.sort();
    let text: String = rows.iter().map(|(f, k)| format!("{f}\t{k}\n")).collect();
    // a table that cannot be written only means the next run asks for `--fresh`
    let _ = std::fs::write(work.join(TABLE), text);
}
