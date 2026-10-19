//! The CXSMILES tail `|...|` of a SMILES: only the enhanced-stereo groups (`a:`, `&n:`, `on:`) are read; every other extension is skipped.

use crate::model::GroupKind;

/// One group as written: its kind, label (`&1` and `&2` are different groups) and 0-based atom indices.
pub(crate) type CxGroup = (GroupKind, u32, Vec<u32>);

/// Splits `text` at the first white space into the SMILES and the content between the bars of the tail (empty when there is none).
pub(crate) fn split(text: &str) -> (&str, &str) {
    let text = text.trim();
    let Some(at) = text.find(char::is_whitespace) else {
        return (text, "");
    };
    let rest = text[at..].trim_start();
    let tail = rest
        .strip_prefix('|')
        .and_then(|r| r.find('|').map(|end| &r[..end]))
        .unwrap_or("");
    (&text[..at], tail)
}

fn start(token: &str) -> Option<(GroupKind, u32, &str)> {
    let (head, first) = token.split_once(':')?;
    let (kind, label) = match head.chars().next()? {
        'a' if head == "a" => (GroupKind::Abs, 0),
        '&' => (GroupKind::And, head[1..].parse().ok()?),
        'o' => (GroupKind::Or, head[1..].parse().ok()?),
        _ => return None,
    };
    Some((kind, label, first))
}

/// Groups of the tail in the order they start, atoms in the order written.
pub(crate) fn groups(tail: &str) -> Vec<CxGroup> {
    let mut out: Vec<CxGroup> = Vec::new();
    let mut current: Option<usize> = None;
    for token in tail.split(',') {
        if let Some((kind, label, first)) = start(token) {
            let at = out
                .iter()
                .position(|g| g.0 == kind && g.1 == label)
                .unwrap_or_else(|| {
                    out.push((kind, label, Vec::new()));
                    out.len() - 1
                });
            current = Some(at);
            if let Ok(n) = first.parse() {
                out[at].2.push(n);
            }
        } else if let (Some(at), Ok(n)) = (current, token.parse::<u32>()) {
            out[at].2.push(n);
        } else {
            current = None;
        }
    }
    out
}
