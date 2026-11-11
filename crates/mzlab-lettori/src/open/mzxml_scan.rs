//! One scan of an mzXML: the attributes of `<scan>`, of its `<precursorMz>` and `<peaks>`, the `<nameValue>` pairs, and the peaks themselves (base64, big-endian, 32 or 64 bit, zlib or not,
//! `m/z-int` pairs interleaved). Written from the ISB schema (mzXML 2.1 and 3.2); a number read from the file is checked against `Limiti` before it allocates.

use std::io::Read;

use base64::Engine;
use mzlab_core::error::{Error, Result};
use mzlab_core::readers::Limits;

/// The text of a tag without its brackets: `name a="1" b="2"` (a tag ends at the first `>` outside quotes).
pub(super) type Tag<'a> = &'a str;

/// The value of attribute `n` in the tag `t` ("" when it has none; entities are not decoded).
pub(super) fn attr<'a>(t: Tag<'a>, n: &str) -> &'a str {
    let mut v = "";
    each(t, &mut |k, x| {
        if k == n && v.is_empty() {
            v = x;
        }
    });
    v
}

/// Calls `f(name, value)` for every attribute of the tag `t`.
pub(super) fn each<'a>(t: Tag<'a>, f: &mut dyn FnMut(&'a str, &'a str)) {
    let mut r = t.trim_end_matches('/');
    r = r.split_once(char::is_whitespace).map_or("", |(_, x)| x);
    while let Some((k, rest)) = r.split_once('=') {
        let rest = rest.trim_start();
        let Some(q) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            break;
        };
        let Some((v, remainder)) = rest[1..].split_once(q) else {
            break;
        };
        f(k.trim(), v);
        r = remainder;
    }
}

/// The tags of `s` in order, as `(text, position after it)`; declarations and comments are skipped. `f` returns `false` to stop.
pub(super) fn tags<'a>(s: &'a str, f: &mut dyn FnMut(Tag<'a>, usize) -> bool) {
    let b = s.as_bytes();
    let mut i = 0;
    while let Some(p) = s.get(i..).and_then(|r| r.find('<')) {
        let a = i + p + 1;
        let (mut q, mut quoted) = (a, 0u8);
        while let Some(&c) = b.get(q) {
            match (quoted, c) {
                (0, b'"' | b'\'') => quoted = c,
                (x, y) if x != 0 && x == y => quoted = 0,
                (0, b'>') => break,
                _ => {}
            }
            q += 1;
        }
        let Some(t) = s.get(a..q) else { break };
        i = q + 1;
        if !t.starts_with(['?', '!']) && !f(t, i) {
            break;
        }
    }
}

/// The name of a tag text (`/scan` for a closing tag).
pub(super) fn name<'a>(t: Tag<'a>) -> &'a str {
    t.split(char::is_whitespace)
        .next()
        .unwrap_or("")
        .trim_end_matches('/')
}

/// A duration `PT1M30.5S` (or a plain number of seconds) in minutes.
pub(super) fn minutes(t: &str) -> Option<f64> {
    let t = t.trim();
    let Some(r) = t.strip_prefix("PT").or_else(|| t.strip_prefix("pt")) else {
        return t.parse::<f64>().ok().map(|s| s / 60.0);
    };
    let (mut sec, mut num) = (0.0, String::new());
    for c in r.chars() {
        let f = match c.to_ascii_uppercase() {
            'H' => 3600.0,
            'M' => 60.0,
            'S' => 1.0,
            _ => {
                num.push(c);
                continue;
            }
        };
        sec += num.parse::<f64>().ok()? * f;
        num.clear();
    }
    num.is_empty().then_some(sec / 60.0)
}

/// The start of a scan (everything before `<peaks`), borrowed from its text.
#[derive(Default)]
pub(super) struct Head<'a> {
    pub(super) scan: Tag<'a>,
    /// (tag, value) of each `<precursorMz>`.
    pub(super) precursori: Vec<(Tag<'a>, &'a str)>,
    /// The `<nameValue>` tags.
    pub(super) values: Vec<Tag<'a>>,
    /// The `<peaks>` tag and its text.
    pub(super) peaks: Option<(Tag<'a>, &'a str)>,
}

/// The scan, its precursors, its `nameValue` pairs and its peaks, out of the text of a scan (at most `tetto` of each kind).
pub(super) fn head(t: &str, cap: usize) -> Result<Head<'_>> {
    let mut r = Head::default();
    let text = |after: usize| {
        t.get(after..)
            .and_then(|x| x.split('<').next())
            .unwrap_or("")
            .trim()
    };
    tags(t, &mut |g, after| {
        match name(g) {
            "scan" if r.scan.is_empty() => r.scan = g,
            "precursorMz" if r.precursori.len() < cap => r.precursori.push((g, text(after))),
            "nameValue" if r.values.len() < cap => r.values.push(g),
            "peaks" => {
                r.peaks = Some((g, if g.ends_with('/') { "" } else { text(after) }));
                // the schema puts the `nameValue` pairs after the peaks
            }
            _ => {}
        }
        true
    });
    if r.scan.is_empty() {
        return Err(xml("no <scan> in its bytes"));
    }
    Ok(r)
}

/// The peaks of a scan: m/z, intensities, and whether the file stores 64-bit values.
pub(super) struct Peaks {
    pub(super) mz: Vec<f64>,
    pub(super) intensities: Vec<f64>,
    pub(super) a_64_bit: bool,
}

fn xml(why: &str) -> Error {
    Error::new("err.file.xml").with("detail", why)
}

/// The bytes of the `<peaks>` text: base64, then zlib when the file says so (at most `limit` bytes; without a declared size, what the compressed bytes explain).
fn raw_values(text: &str, zlib: bool, limit: usize, declared: Option<usize>) -> Result<Vec<u8>> {
    let clean: Vec<u8> = text.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let raw = base64::engine::general_purpose::STANDARD
        .decode(&clean)
        .map_err(|e| Error::new("err.file.base64").with("detail", e))?;
    if !zlib {
        return Ok(raw);
    }
    let limit =
        limit.min(declared.unwrap_or_else(|| mzlab_core::budget::undeclared_inflate(raw.len())));
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(&raw[..])
        .take(limit as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| Error::new("err.file.zlib").with("detail", e))?;
    if out.len() > limit {
        return Err(Error::memory_budget());
    }
    Ok(out)
}

/// Decodes the `<peaks>` of a scan with the attributes `attr`: `precision`, `byteOrder`, `contentType`/`pairOrder`, `compressionType`; `conteggio` is the `peaksCount` of the scan.
pub(super) fn peaks(tag: Tag, text: &str, count: Option<usize>, limits: &Limits) -> Result<Peaks> {
    let width = match attr(tag, "precision") {
        "" | "32" => 4usize,
        "64" => 8,
        _ => return Err(xml("precision")),
    };
    if !matches!(attr(tag, "byteOrder"), "" | "network" | "big") {
        return Err(xml("byteOrder"));
    }
    let order = [attr(tag, "contentType"), attr(tag, "pairOrder")];
    if order.iter().any(|o| !matches!(*o, "" | "m/z-int")) {
        return Err(xml("contentType"));
    }
    let zlib = match attr(tag, "compressionType") {
        "" | "none" => false,
        "zlib" => true,
        _ => return Err(xml("compressionType")),
    };
    let cap = match count {
        Some(n) => limits.ok_entries(n as u64)?,
        None => limits.entries,
    };
    let limit = usize::try_from(limits.byte)
        .unwrap_or(usize::MAX)
        .min(cap.saturating_mul(2 * width));
    let b = raw_values(
        text,
        zlib,
        limit,
        count.map(|n| n.saturating_mul(2 * width)),
    )?;
    let n = (b.len() / (2 * width)).min(count.unwrap_or(usize::MAX));
    let (mut mz, mut intensities) = (Vec::new(), Vec::new());
    mz.try_reserve_exact(n)
        .map_err(|_| Error::memory_budget())?;
    intensities
        .try_reserve_exact(n)
        .map_err(|_| Error::memory_budget())?;
    for c in b.chunks_exact(2 * width).take(n) {
        let (x, y) = c.split_at(width);
        mz.push(large(x));
        intensities.push(large(y));
    }
    Ok(Peaks {
        mz,
        intensities,
        a_64_bit: width == 8,
    })
}

/// A big-endian float of 4 or 8 bytes.
fn large(b: &[u8]) -> f64 {
    match <[u8; 4]>::try_from(b) {
        Ok(q) => f64::from(f32::from_be_bytes(q)),
        Err(_) => <[u8; 8]>::try_from(b).map_or(0.0, f64::from_be_bytes),
    }
}
