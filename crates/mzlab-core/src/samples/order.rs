//! Puts the files of a samples document (`campioni/1`, `/2` or `/3`) in the order of the files of a table, matched by file name: the readers of the
//! roles pair the samples with the files by position, so every step that holds a table calls `order_like` first (`docs/agenti/campioni.md`).

use super::json::bad;
use crate::error::Result;
use crate::json::{parse, write_value, Out, Value};

/// The name of a file without its folders (`/` or `\`).
pub(super) fn base(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// The extensions of the files a series is read from (raw, converted, indexed): `T5`, `T5.mzML` and `T5.mzML.mzidx` name one file.
const EXT: [&str; 13] = [
    "mzidx", "mzml", "mzxml", "mzdata", "mgf", "raw", "wiff", "wiff2", "d", "cdf", "dam", "gz",
    "mzfeatf",
];

pub(super) fn stem(name: &str) -> &str {
    let mut s = base(name);
    while let Some((head, ext)) = s.rsplit_once('.') {
        if head.is_empty() || !EXT.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
            break;
        }
        s = head;
    }
    s
}

/// `text` with its files in the order of `names` (the files of the table, with or without folders and the extensions of `EXT`); the other fields are kept.
/// The same text when the order is already that one, or when `names` is empty or has an empty name (a table that does not record its files).
/// A name with no sample, or a sample naming no file of the table, is `error.badCampioni` with the name in `file`.
pub fn order_like(text: &str, names: &[String]) -> Result<String> {
    if names.is_empty() || names.iter().any(|n| base(n).is_empty()) {
        return Ok(text.to_string());
    }
    let mut doc = parse(text.as_bytes())?;
    let Value::Obj(fields) = &mut doc else {
        return Err(bad());
    };
    let Some((_, Value::Arr(rows))) = fields.iter_mut().find(|(k, _)| k == "campioni") else {
        return Err(bad());
    };
    let file = |v: &Value| base(v.get("file").and_then(|x| x.str()).unwrap_or("")).to_string();
    let files: Vec<String> = rows.iter().map(file).collect();
    let mut used = vec![false; rows.len()];
    let mut pick = vec![usize::MAX; names.len()];
    // the exact names first, so that `S1` never takes the place of `S1.5`
    for exact in [true, false] {
        for (i, n) in names.iter().enumerate() {
            if pick[i] != usize::MAX {
                continue;
            }
            let n = base(n);
            let hit = (0..rows.len()).find(|&k| {
                !used[k]
                    && if exact {
                        files[k] == n
                    } else {
                        stem(&files[k]) == stem(n)
                    }
            });
            if let Some(k) = hit {
                used[k] = true;
                pick[i] = k;
            }
        }
    }
    if let Some(i) = pick.iter().position(|&k| k == usize::MAX) {
        return Err(bad().with("file", base(&names[i])));
    }
    if let Some(k) = used.iter().position(|u| !u) {
        return Err(bad().with("file", &files[k]));
    }
    if pick.iter().enumerate().all(|(i, &k)| i == k) {
        return Ok(text.to_string());
    }
    let mut old = std::mem::take(rows);
    *rows = pick
        .iter()
        .map(|&k| std::mem::replace(&mut old[k], Value::Null))
        .collect();
    let mut o = Out::default();
    write_value(&mut o, &doc);
    Ok(o.0)
}
