//! `/api/scaninfo`: every name / value pair of the header of one scan as the mzML has it, plus the fields of the scan table
//! (`Item.scan_header` in `mzlab/explore.py`). Only the header up to `<binaryDataArrayList` is read from the mzML, never the arrays.

use crate::error::{Error, Result};
use crate::idxread::{ReadAt, Scan};
use crate::json::{round, Out};

const MARK: &[u8] = b"<binaryDataArrayList";

/// Header of a spectrum, read in small steps until the arrays begin.
fn header<M: ReadAt>(mzml: &mut M, start: u64, end: u64) -> Result<String> {
    let mut buf: Vec<u8> = Vec::new();
    let mut at = start;
    while at < end {
        let n = (end - at).min(8192) as usize;
        let from = buf.len();
        buf.resize(from + n, 0);
        mzml.read_at(at, &mut buf[from..])?;
        at += n as u64;
        // the marker may straddle two reads
        let look = from.saturating_sub(MARK.len());
        if let Some(p) = buf[look..].windows(MARK.len()).position(|w| w == MARK) {
            buf.truncate(look + p);
            break;
        }
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `(\w+)="([^"]*)"` found left to right, as `re.findall` does.
fn attrs(body: &str) -> Vec<(String, String)> {
    let s: Vec<char> = body.chars().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < s.len() {
        if !is_word(s[i]) {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < s.len() && is_word(s[j]) {
            j += 1;
        }
        if j + 1 < s.len() && s[j] == '=' && s[j + 1] == '"' {
            if let Some(k) = s[j + 2..].iter().position(|&c| c == '"') {
                out.push((s[i..j].iter().collect(), s[j + 2..j + 2 + k].iter().collect()));
                i = j + 2 + k + 1;
                continue;
            }
        }
        i = j;
    }
    out
}

/// `<(cvParam|userParam)\b([^>]*?)/?>`: the (name, value, unit) of every parameter that has a name.
fn pairs(head: &str) -> Vec<(String, String, String)> {
    let mut out = vec![];
    let mut rest = head;
    while let Some(p) = rest.find('<') {
        let tail = &rest[p + 1..];
        let tag = ["cvParam", "userParam"].into_iter().find(|t| tail.starts_with(t));
        let boundary = tag.is_some_and(|t| !tail[t.len()..].chars().next().is_some_and(is_word));
        let Some(end) = tail.find('>').filter(|_| boundary) else {
            rest = tail;
            continue;
        };
        let body = &tail[tag.unwrap().len()..end];
        let body = body.strip_suffix('/').unwrap_or(body);
        let a = attrs(body);
        let get = |k: &str| a.iter().rev().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        if let Some(name) = get("name").filter(|n| !n.is_empty()) {
            out.push((name, get("value").unwrap_or_default(), get("unitName").unwrap_or_default()));
        }
        rest = &tail[end + 1..];
    }
    out
}

/// The JSON of `/api/scaninfo` for scan `sid` of `scans` (the rows of the index); the header comes from the mzML.
pub fn scaninfo<M: ReadAt>(scans: &[Scan], sid: usize, mzml: &mut M) -> Result<String> {
    let sc = scans.get(sid).ok_or_else(|| Error::new("err.scan.outside"))?;
    let head = header(mzml, sc.start, sc.end)?;
    let mut o = Out::default();
    o.raw("{\"sid\":").int(sid as i64).raw(",\"no\":").int(sc.number(sid)).raw(",\"filter\":").str(&sc.filter);
    o.raw(",\"rt\":").num(round(sc.rt, 4)).raw(",\"level\":").int(sc.level as i64);
    o.raw(",\"polarity\":").str(match sc.polarity {
        1 => "positive",
        -1 => "negative",
        _ => "?",
    });
    o.raw(",\"profile\":").raw(if sc.profile { "true" } else { "false" });
    let prec = match (sc.iso, sc.precursor) {
        (Some((a, b)), _) => Some((a + b) / 2.0),
        (None, Some(p)) if p != 0.0 => Some(p),
        _ => None,
    };
    o.raw(",\"prec\":").opt(prec.map(|p| round(p, 5)));
    match sc.iso {
        Some((a, b)) => o.raw(",\"iso\":[").num(round(a, 4)).raw(",").num(round(b, 4)).raw("]"),
        None => o.raw(",\"iso\":null"),
    };
    o.raw(",\"act\":");
    match &sc.act {
        Some(a) => o.str(a),
        None => o.raw("null"),
    };
    o.raw(",\"ce\":").opt(sc.collision_energy).raw(",\"res\":").opt(sc.res).raw(",\"analyzer\":");
    match &sc.an {
        Some(a) => o.str(a),
        None => o.raw("null"),
    };
    o.raw(",\"tic\":").num(sc.tic).raw(",\"key\":").str(&sc.key()).raw(",\"pairs\":[");
    for (i, (n, v, u)) in pairs(&head).iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"name\":").str(n).raw(",\"value\":").str(v).raw(",\"unit\":").str(u).raw("}");
    }
    o.raw("]}");
    Ok(o.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_pairs() {
        let h = r#"<spectrum><cvParam cvRef="MS" accession="MS:1" name="ms level" value="1"/><userParam name="x" value="a" unitName="s" /><cvParamX name="no"/><cvParam name=""/></spectrum>"#;
        let p = pairs(h);
        assert_eq!(p, vec![("ms level".into(), "1".into(), "".into()), ("x".into(), "a".into(), "s".into())]);
    }

    #[test]
    fn outside() {
        assert_eq!(scaninfo(&[], 0, &mut Vec::<u8>::new()).unwrap_err().key, "err.scan.outside");
    }
}
