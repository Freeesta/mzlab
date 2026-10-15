//! One scan of an mzXML: the attributes of `<scan>`, of its `<precursorMz>` and `<peaks>`, the `<nameValue>` pairs, and the peaks themselves (base64, big-endian, 32 or 64 bit, zlib or not,
//! `m/z-int` pairs interleaved). Written from the ISB schema (mzXML 2.1 and 3.2); a number read from the file is checked against `Limiti` before it allocates.

use std::io::Read;

use base64::Engine;
use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::Limiti;

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
        let Some((v, resto)) = rest[1..].split_once(q) else {
            break;
        };
        f(k.trim(), v);
        r = resto;
    }
}

/// The tags of `s` in order, as `(text, position after it)`; declarations and comments are skipped. `f` returns `false` to stop.
pub(super) fn tags<'a>(s: &'a str, f: &mut dyn FnMut(Tag<'a>, usize) -> bool) {
    let b = s.as_bytes();
    let mut i = 0;
    while let Some(p) = s.get(i..).and_then(|r| r.find('<')) {
        let a = i + p + 1;
        let (mut q, mut citato) = (a, 0u8);
        while let Some(&c) = b.get(q) {
            match (citato, c) {
                (0, b'"' | b'\'') => citato = c,
                (x, y) if x != 0 && x == y => citato = 0,
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
pub(super) fn nome<'a>(t: Tag<'a>) -> &'a str {
    t.split(char::is_whitespace)
        .next()
        .unwrap_or("")
        .trim_end_matches('/')
}

/// A duration `PT1M30.5S` (or a plain number of seconds) in minutes.
pub(super) fn minuti(t: &str) -> Option<f64> {
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
pub(super) struct Testa<'a> {
    pub(super) scan: Tag<'a>,
    /// (tag, value) of each `<precursorMz>`.
    pub(super) precursori: Vec<(Tag<'a>, &'a str)>,
    /// The `<nameValue>` tags.
    pub(super) valori: Vec<Tag<'a>>,
    /// The `<peaks>` tag and its text.
    pub(super) picchi: Option<(Tag<'a>, &'a str)>,
}

/// The scan, its precursors, its `nameValue` pairs and its peaks, out of the text of a scan (at most `tetto` of each kind).
pub(super) fn testa(t: &str, tetto: usize) -> Result<Testa<'_>> {
    let mut r = Testa::default();
    let testo = |dopo: usize| {
        t.get(dopo..)
            .and_then(|x| x.split('<').next())
            .unwrap_or("")
            .trim()
    };
    tags(t, &mut |g, dopo| {
        match nome(g) {
            "scan" if r.scan.is_empty() => r.scan = g,
            "precursorMz" if r.precursori.len() < tetto => r.precursori.push((g, testo(dopo))),
            "nameValue" if r.valori.len() < tetto => r.valori.push(g),
            "peaks" => {
                r.picchi = Some((g, if g.ends_with('/') { "" } else { testo(dopo) }));
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
pub(super) struct Picchi {
    pub(super) mz: Vec<f64>,
    pub(super) intensita: Vec<f64>,
    pub(super) a_64_bit: bool,
}

fn xml(why: &str) -> Error {
    Error::new("err.file.xml").with("detail", why)
}

/// The bytes of the `<peaks>` text: base64, then zlib when the file says so (at most `limite` bytes).
fn grezzi(testo: &str, zlib: bool, limite: usize) -> Result<Vec<u8>> {
    let pulito: Vec<u8> = testo.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let raw = base64::engine::general_purpose::STANDARD
        .decode(&pulito)
        .map_err(|e| Error::new("err.file.base64").with("detail", e))?;
    if !zlib {
        return Ok(raw);
    }
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(&raw[..])
        .take(limite as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| Error::new("err.file.zlib").with("detail", e))?;
    if out.len() > limite {
        return Err(Error::memory_budget());
    }
    Ok(out)
}

/// Decodes the `<peaks>` of a scan with the attributes `attr`: `precision`, `byteOrder`, `contentType`/`pairOrder`, `compressionType`; `conteggio` is the `peaksCount` of the scan.
pub(super) fn picchi(
    tag: Tag,
    testo: &str,
    conteggio: Option<usize>,
    limiti: &Limiti,
) -> Result<Picchi> {
    let larghezza = match attr(tag, "precision") {
        "" | "32" => 4usize,
        "64" => 8,
        _ => return Err(xml("precision")),
    };
    if !matches!(attr(tag, "byteOrder"), "" | "network" | "big") {
        return Err(xml("byteOrder"));
    }
    let ordine = [attr(tag, "contentType"), attr(tag, "pairOrder")];
    if ordine.iter().any(|o| !matches!(*o, "" | "m/z-int")) {
        return Err(xml("contentType"));
    }
    let zlib = match attr(tag, "compressionType") {
        "" | "none" => false,
        "zlib" => true,
        _ => return Err(xml("compressionType")),
    };
    let tetto = match conteggio {
        Some(n) => limiti.voci_ok(n as u64)?,
        None => limiti.voci,
    };
    let limite = usize::try_from(limiti.byte)
        .unwrap_or(usize::MAX)
        .min(tetto.saturating_mul(2 * larghezza));
    let b = grezzi(testo, zlib, limite)?;
    let n = (b.len() / (2 * larghezza)).min(conteggio.unwrap_or(usize::MAX));
    let (mut mz, mut intensita) = (Vec::new(), Vec::new());
    mz.try_reserve_exact(n)
        .map_err(|_| Error::memory_budget())?;
    intensita
        .try_reserve_exact(n)
        .map_err(|_| Error::memory_budget())?;
    for c in b.chunks_exact(2 * larghezza).take(n) {
        let (x, y) = c.split_at(larghezza);
        mz.push(grande(x));
        intensita.push(grande(y));
    }
    Ok(Picchi {
        mz,
        intensita,
        a_64_bit: larghezza == 8,
    })
}

/// A big-endian float of 4 or 8 bytes.
fn grande(b: &[u8]) -> f64 {
    match <[u8; 4]>::try_from(b) {
        Ok(q) => f64::from(f32::from_be_bytes(q)),
        Err(_) => <[u8; 8]>::try_from(b).map_or(0.0, f64::from_be_bytes),
    }
}
