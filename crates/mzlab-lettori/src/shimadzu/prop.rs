//! The property streams of the Shimadzu containers. A `.lcd` keeps them as flat XML (`<key>value</key>`; a text is `@StoX@<hex of its bytes>`, a number `@FtoX@<hex of a float>`), a `.qgd` as a
//! binary record with fixed-width fields (`qgd`). Written from the files themselves (docs/agenti/shimadzu.md); the bytes of a text are Windows-31J, of which only ASCII is decoded here.

/// The most key/value pairs read from one XML stream.
const PAIRS_MAX: usize = 4096;
/// Seconds between 1601-01-01 and 1970-01-01 (Windows `FILETIME`).
const FILETIME_EPOCH: u64 = 11_644_473_600;

/// The text of a hex string of bytes: ASCII as it is, a byte over 0x7F as `?` (UTF-8 text is kept whole).
fn text_of_hex(h: &str) -> String {
    let b: Vec<u8> = (0..h.len() / 2)
        .filter_map(|i| u8::from_str_radix(h.get(2 * i..2 * i + 2)?, 16).ok())
        .collect();
    let b = b.split(|&c| c == 0).next().unwrap_or(&[]);
    match std::str::from_utf8(b) {
        Ok(s) => s.trim().to_string(),
        Err(_) => b
            .iter()
            .map(|&c| if c < 0x80 { c as char } else { '?' })
            .collect::<String>()
            .trim()
            .to_string(),
    }
}

/// A value of the XML: `@StoX@…` is a text, `@FtoX@…` a float (8 hex digits are its bits, fewer a decimal number), anything else as it is.
pub(super) fn value_of(v: &str) -> String {
    if let Some(h) = v.strip_prefix("@StoX@") {
        return text_of_hex(h);
    }
    if let Some(h) = v.strip_prefix("@FtoX@") {
        if h.len() == 8 {
            if let Ok(bits) = u32::from_str_radix(h, 16) {
                let f = f32::from_bits(bits);
                if f.is_finite() {
                    return f.to_string();
                }
            }
        }
        return h.to_string();
    }
    v.trim().to_string()
}

/// The leaf elements of a property XML as `(key, value)`, in file order, values decoded (`value_of`).
pub(super) fn xml(bytes: &[u8]) -> Vec<(String, String)> {
    let from = bytes.windows(5).position(|w| w == b"<?xml").unwrap_or(0);
    let t = String::from_utf8_lossy(&bytes[from..]);
    let mut out = Vec::new();
    let mut rest: &str = &t;
    while let Some(a) = rest.find('<') {
        let Some(b) = rest[a..].find('>') else { break };
        let name = &rest[a + 1..a + b];
        rest = &rest[a + b + 1..];
        if name.starts_with(['?', '/', '!']) || name.ends_with('/') || name.contains(' ') {
            continue;
        }
        let Some(end) = rest.find('<') else { break };
        if rest[end..].starts_with(&format!("</{name}>")) {
            out.push((name.to_string(), value_of(&rest[..end])));
            if out.len() >= PAIRS_MAX {
                break;
            }
        }
    }
    out
}

/// The `FILETIME` made of the decimal `low` (signed 32 bits) and `high` of the XML, as `YYYY-MM-DDThh:mm:ss`.
pub(super) fn filetime_xml(low: &str, high: &str) -> Option<String> {
    let lo = low.trim().parse::<i32>().ok()? as u32 as u64;
    let hi = high.trim().parse::<i64>().ok()?;
    filetime_iso((u64::try_from(hi).ok()? << 32) | lo)
}

/// `FILETIME` (100 ns since 1601) as `YYYY-MM-DDThh:mm:ss`, exactly as the file says (no zone is converted); `None` outside 1990–2100.
pub(super) fn filetime_iso(ft: u64) -> Option<String> {
    let s = (ft / 10_000_000).checked_sub(FILETIME_EPOCH)?;
    if !(631_152_000..4_102_444_800).contains(&s) {
        return None;
    }
    let (days, rem) = (s / 86_400, s % 86_400);
    // civil-from-days (Howard Hinnant)
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    Some(format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    ))
}

/// The fixed-width text at `at` (`len` bytes, up to the first NUL, trimmed); empty when the stream is shorter.
fn str_at(b: &[u8], at: usize, len: usize) -> String {
    let s = b.get(at..(at + len).min(b.len())).unwrap_or(&[]);
    let s = s.split(|&c| c == 0).next().unwrap_or(&[]);
    s.iter()
        .map(|&c| {
            if (0x20..0x7f).contains(&c) {
                c as char
            } else {
                '?'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn time_at(b: &[u8], at: usize) -> Option<String> {
    filetime_iso(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// The fields of the `File Property` of a `.qgd`, `(key, value)` in file order. The offsets are those of the seven files at hand (docs/agenti/shimadzu.md); an empty field is left out.
pub(super) fn qgd(b: &[u8]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (k, v) in [
        ("fileFormatVersion", str_at(b, 4, 16)),
        ("generatedBy", str_at(b, 20, 32)),
        ("generatedTime", time_at(b, 52).unwrap_or_default()),
        ("modifiedBy", str_at(b, 60, 32)),
        ("modifiedTime", time_at(b, 92).unwrap_or_default()),
        ("softwareVersion", str_at(b, 148, 16)),
        ("sampleName", str_at(b, 204, 32)),
        ("sampleId", str_at(b, 236, 32)),
        ("operator", str_at(b, 300, 32)),
        ("acquisitionTime", time_at(b, 508).unwrap_or_default()),
        ("dataFile", str_at(b, 580, 256)),
        ("methodFile", str_at(b, 836, 256)),
        ("tuneFile", str_at(b, 1092, 256)),
        ("batchFile", str_at(b, 1604, 256)),
    ] {
        if !v.is_empty() {
            out.push((k.to_string(), v));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_and_dates() {
        assert_eq!(value_of("@StoX@352E3031"), "5.01");
        assert_eq!(value_of("@FtoX@40800000"), "4");
        assert_eq!(value_of("@FtoX@1"), "1");
        assert_eq!(value_of(" 7 "), "7");
        // 2023-03-09T00:00:00 = 133_227_936_000_000_000 ticks
        assert_eq!(
            filetime_iso(133_227_936_000_000_000).as_deref(),
            Some("2023-03-09T00:00:00")
        );
        assert_eq!(filetime_iso(0), None);
        let xml = b"\x20\0\0\x01<?xml version=\"1.0\"?>\r\n<R><a>@StoX@4142</a><b/><c>3</c></R>";
        assert_eq!(
            super::xml(xml),
            vec![("a".into(), "AB".into()), ("c".into(), "3".into())]
        );
    }
}
