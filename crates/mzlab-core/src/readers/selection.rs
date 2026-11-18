//! Which samples of a multi-sample file to open (`--sample 1,3,5`, `2-4`): one parser and one check against the ids the reader lists, used by the CLI,
//! the app and the site's engine route (`sample_selection`), so the three give the same answer for the same text.

use crate::error::{Error, Result};
use crate::json::Out;

/// The ids of `text`: numbers and ranges `A-B` separated by commas, each id once, ascending. `err.sample.list` for anything else (empty item, `B` below `A`).
pub fn parse(text: &str) -> Result<Vec<u32>> {
    let bad = || Error::new("err.sample.list").with("text", text);
    let mut ids = Vec::new();
    for item in text.split(',') {
        let num = |x: &str| x.trim().parse::<u32>().map_err(|_| bad());
        match item.split_once('-') {
            Some((a, b)) => {
                let (a, b) = (num(a)?, num(b)?);
                if b < a || b - a > 65_535 {
                    return Err(bad());
                }
                ids.extend(a..=b);
            }
            None => ids.push(num(item)?),
        }
    }
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// `wanted` checked against the `present` ids of the file: they all have to be there, else `err.sample.unknown` names the missing ones and the ids present
/// (no silent skip). The answer is in the order of the file.
pub fn resolve(present: &[u32], wanted: &[u32]) -> Result<Vec<u32>> {
    let list = |v: &[u32]| {
        v.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let missing: Vec<u32> = wanted
        .iter()
        .copied()
        .filter(|i| !present.contains(i))
        .collect();
    if !missing.is_empty() {
        return Err(Error::new("err.sample.unknown")
            .with("missing", list(&missing))
            .with("ids", list(present)));
    }
    Ok(present
        .iter()
        .copied()
        .filter(|i| wanted.contains(i))
        .collect())
}

/// `{"samples":[1,3,5]}`: the ids that `text` selects among `present`; the route the site calls before it opens only those samples.
pub fn selection_json(present: &[u32], text: &str) -> Result<String> {
    let ids = resolve(present, &parse(text)?)?;
    let mut o = Out::default();
    o.raw("{\"samples\":[");
    for (k, i) in ids.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).raw(&i.to_string());
    }
    o.raw("]}");
    Ok(o.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lists_and_ranges() {
        assert_eq!(parse("1,3,5").unwrap(), [1, 3, 5]);
        assert_eq!(parse("5, 2-4,3").unwrap(), [2, 3, 4, 5]);
        for bad in ["", "1,,2", "4-2", "a", "1-", "-1", "0-70000"] {
            assert_eq!(parse(bad).unwrap_err().key, "err.sample.list", "{bad}");
        }
    }

    #[test]
    fn unknown_ids_are_named() {
        assert_eq!(resolve(&[1, 2, 3], &[3, 1]).unwrap(), [1, 3]);
        let e = resolve(&[1, 2], &[2, 9, 7]).unwrap_err();
        assert_eq!(e.key, "err.sample.unknown");
        assert_eq!(
            selection_json(&[1, 2, 3], "3,1").unwrap(),
            "{\"samples\":[1,3]}"
        );
    }
}
