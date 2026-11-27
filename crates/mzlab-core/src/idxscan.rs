//! The two text rules of `idxread::Scan::key` (the name of a scan type): the scan range at the end of a filter and the analyzer and polarity sign at its start.

/// `re.sub(r"\s*\[[^\]]*\]\s*$", "", f)`: the scan range in square brackets at the end of a filter.
pub(crate) fn strip_range(f: &str) -> &str {
    let t = f.trim_end();
    if t.ends_with(']')
        && let Some(p) = t.rfind('[')
        && !t[p + 1..t.len() - 1].contains(']')
    {
        return t[..p].trim_end();
    }
    f
}

/// `^(\w+)\s+([+-])\s` of the filter: the analyzer and the polarity sign.
pub(crate) fn key_prefix(f: &str) -> Option<(&str, char)> {
    let w = f.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    if w == 0 {
        return None;
    }
    let rest = &f[w..];
    let t = rest.trim_start();
    if t.len() == rest.len() {
        return None;
    }
    let sign = t.chars().next().filter(|c| *c == '+' || *c == '-')?;
    let after = &t[1..];
    if after.chars().next().is_some_and(char::is_whitespace) {
        Some((&f[..w], sign))
    } else {
        None
    }
}
