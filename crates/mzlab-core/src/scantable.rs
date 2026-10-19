//! The scan table of an mzML: what `mzlab/reader/mzml.py` (`Run._index`, `_read_header`, `_link_parents`) reads from the headers of the
//! spectra, written as the compact JSON rows of `SCAN_COLUMNS` that the `.mzidx` stores. The data arrays are never decoded: only the
//! header of each spectrum (up to `<binaryDataArrayList`) is read. Text is searched as the Python regular expressions do, without a
//! regex engine (it would cost size in the WASM).

use std::sync::Arc;

use crate::budget::MemBudget;
use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::lettori::Posizione;
use crate::stima;

const ANALYZERS: [&str; 5] = ["FTMS", "ITMS", "TOFMS", "TQMS", "SQMS"];

#[derive(Debug, Clone, Default)]
struct Prec {
    target: Option<f64>,
    lo: Option<f64>,
    hi: Option<f64>,
    selected: Option<f64>,
    charge: Option<i64>,
    refer: Option<Arc<str>>,
    act: Option<&'static str>,
    ce: Option<f64>,
    pint: Option<f64>,
}

/// What only a scan with a precursor (or a vendor file) has: kept behind one pointer so that a survey scan does not carry the room for it.
#[derive(Debug, Clone, Default)]
pub struct Detail {
    pub precursor: Option<f64>,
    ce: Option<f64>,
    iso: Option<(f64, f64)>,
    act: Option<Arc<str>>,
    pint: Option<f64>,
    path: Option<Vec<(f64, String, f64)>>,
    precs: Vec<Prec>,
    charge: Option<i64>,
}

impl Detail {
    const EMPTY: Detail = Detail {
        precursor: None,
        ce: None,
        iso: None,
        act: None,
        pint: None,
        path: None,
        precs: Vec::new(),
        charge: None,
    };

    /// Gives back the room that `push` left in the lists (a `Vec` takes room for four precursors at the first one, 576 bytes).
    fn shrink(&mut self) {
        self.precs.shrink_to_fit();
        if let Some(p) = self.path.as_mut() {
            p.shrink_to_fit();
        }
    }

    /// Whether the scan has nothing of what a `Detail` holds (then it has none).
    fn is_empty(&self) -> bool {
        self.precursor.is_none()
            && self.ce.is_none()
            && self.iso.is_none()
            && self.act.is_none()
            && self.pint.is_none()
            && self.path.is_none()
            && self.precs.is_empty()
            && self.charge.is_none()
    }
}

/// Where a scan of a vendor file lies: the number of the scan in `Lettore::indice` and its place (written in place of `start`, `end`).
#[derive(Debug, Clone, Copy)]
pub struct Dove {
    pub k: usize,
    pub posizione: Option<Posizione>,
}

#[derive(Debug, Clone, Default)]
pub struct Row {
    pub start: u64,
    pub end: u64,
    /// A row of a vendor file (`mzidx_lettore::rows_json`).
    pub dove: Option<Box<Dove>>,
    pub level: u8,
    pub rt: f64,
    pub polarity: i8,
    pub tic: f64,
    /// The filter string, shared by all the rows that have the same text (`Rows::intern`).
    pub filter: Arc<str>,
    pub profile: bool,
    parent: Option<usize>,
    res: Option<f64>,
    /// The analyzer: one of `ANALYZERS`, or `?`.
    pub an: &'static str,
    /// The native id, the same allocation as the key of `Rows::level_of`.
    pub native: Arc<str>,
    /// Scan type that is not a plain full or product-ion scan (`PREC`, `NL`, `NG`, `EPI`): see `kind_of`.
    pub kind: Option<&'static str>,
    /// The mobility of the whole scan (a scan attribute) and its unit (`k0` or `ms`).
    pub mob: Option<(f64, &'static str)>,
    detail: Option<Box<Detail>>,
}

impl Row {
    fn d(&self) -> &Detail {
        static NONE: Detail = Detail::EMPTY;
        self.detail.as_deref().unwrap_or(&NONE)
    }

    /// The precursor m/z of the scan.
    pub fn precursor(&self) -> Option<f64> {
        self.d().precursor
    }
}

/// The table, with what the `run` object of the metadata needs.
#[derive(Debug, Default)]
pub struct Table {
    pub rows: Vec<Row>,
    pub nce: bool,
    pub hr1: bool,
    pub n_chromatograms: usize,
}

/// The scan type of a spectrum, from the CV term of its header (`MS:1000341` precursor ion, `MS:1000326` constant neutral loss, `MS:1000325`
/// constant neutral gain, `MS:1000789` enhanced multiply charged and `MS:1000790` time-delayed fragmentation, both shown as `EPI`) or, when the
/// header says nothing, from a Thermo filter token (`pr`, `cnl`, `cng`). Never from a name.
pub(crate) fn kind_of(h: &str, filter: &str) -> Option<&'static str> {
    const TERMS: [(&str, &str); 5] = [
        ("MS:1000341", "PREC"),
        ("MS:1000326", "NL"),
        ("MS:1000325", "NG"),
        ("MS:1000789", "EPI"),
        ("MS:1000790", "EPI"),
    ];
    for (acc, k) in TERMS {
        if h.contains(&format!("accession=\"{acc}\"")) {
            return Some(k);
        }
    }
    filter
        .split_whitespace()
        .find_map(|w| match w.to_ascii_lowercase().as_str() {
            "pr" => Some("PREC"),
            "cnl" => Some("NL"),
            "cng" => Some("NG"),
            _ => None,
        })
}

/// The mobility of a scan from its header: the scan attribute `MS:1002815` (inverse reduced, `k0`) or `MS:1002476` (drift time, `ms`).
pub(crate) fn mobility_of_header(h: &str) -> Option<(f64, &'static str)> {
    [("MS:1002815", "k0"), ("MS:1002476", "ms")]
        .into_iter()
        .find_map(|(acc, u)| cv(h, acc).and_then(float).map(|v| (v, u)))
}

fn float(s: &str) -> Option<f64> {
    s.trim().parse().ok()
}

/// Value of the first cvParam with this accession that has a `value` in the same tag (`accession="X"[^>]*?value="([^"]*)"`).
fn cv<'a>(h: &'a str, acc: &str) -> Option<&'a str> {
    let pat = format!("accession=\"{acc}\"");
    let mut from = 0;
    while let Some(i) = h[from..].find(&pat) {
        let at = from + i + pat.len();
        let tag_end = h[at..].find('>').map_or(h.len(), |g| at + g);
        if let Some(v) = h[at..tag_end].find("value=\"") {
            let s = at + v + 7;
            if let Some(e) = h[s..].find('"') {
                return Some(&h[s..s + e]);
            }
        }
        from = at;
    }
    None
}

/// `value="..."` of the first attribute of that name in a tag-like text.
fn attr<'a>(t: &'a str, name: &str) -> Option<&'a str> {
    let pat = format!("{name}=\"");
    let s = t.find(&pat)? + pat.len();
    let e = t[s..].find('"')?;
    Some(&t[s..s + e])
}

fn precursor_element(e: &str, refer: Option<Arc<str>>) -> Prec {
    // _RX_PCV.findall: every accession of the set that has a value in its tag; the first occurrence of each wins
    const SET: [&str; 7] = ["827", "828", "829", "744", "041", "045", "042"];
    let mut found: Vec<(&str, Option<f64>)> = Vec::new();
    let mut from = 0;
    while let Some(i) = e[from..].find("accession=\"MS:1000") {
        let at = from + i + 18;
        let code = e.get(at..at + 3).unwrap_or("");
        from = at;
        if !SET.contains(&code) || e.get(at + 3..at + 4) != Some("\"") {
            continue;
        }
        let rest = at + 4;
        let tag_end = e[rest..].find('>').map_or(e.len(), |g| rest + g);
        if let Some(v) = e[rest..tag_end].find("value=\"") {
            let s = rest + v + 7;
            if let Some(q) = e[s..].find('"') {
                found.push((code, float(&e[s..s + q])));
                from = s + q;
            }
        }
    }
    let get = |c: &str| found.iter().find(|(k, _)| *k == c).and_then(|(_, v)| *v);
    let (tgt, off_lo, off_hi, ch) = (get("827"), get("828"), get("829"), get("041"));
    Prec {
        target: tgt,
        lo: tgt.zip(off_lo).map(|(t, o)| t - o),
        hi: tgt.zip(off_hi).map(|(t, o)| t + o),
        selected: get("744"),
        charge: ch.filter(|c| *c != 0.0).map(|c| c as i64),
        refer,
        act: if e.contains("accession=\"MS:1000422\"") {
            Some("HCD")
        } else if e.contains("accession=\"MS:1000133\"") {
            Some("CID")
        } else if e.contains("accession=\"MS:1000598\"") {
            Some("ETD")
        } else {
            None
        },
        ce: get("045"),
        pint: get("042"),
    }
}

/// `_filter_path`: `(\d+\.\d+)@([a-z]+)(\d+\.\d+)` (case-insensitive) in a Thermo filter string.
fn filter_path(f: &str) -> Option<Vec<(f64, String, f64)>> {
    if !f.contains('@') {
        return None;
    }
    let b = f.as_bytes();
    let num = |mut i: usize| -> Option<(usize, usize)> {
        // \d+\.\d+ starting at i
        let s = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == s || b.get(i) != Some(&b'.') {
            return None;
        }
        i += 1;
        let d = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == d {
            return None;
        }
        Some((s, i))
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            if let Some((s, e)) = num(i) {
                if b.get(e) == Some(&b'@') {
                    let mut j = e + 1;
                    while j < b.len() && b[j].is_ascii_alphabetic() {
                        j += 1;
                    }
                    if j > e + 1 {
                        if let Some((s2, e2)) = num(j) {
                            out.push((
                                f[s..e].parse().unwrap_or(0.0),
                                f[e + 1..j].to_uppercase(),
                                f[s2..e2].parse().unwrap_or(0.0),
                            ));
                            i = e2;
                            continue;
                        }
                    }
                }
                i = e;
                continue;
            }
        }
        i += 1;
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn path_key(p: &[(f64, String, f64)]) -> Vec<(i64, String, u64)> {
    p.iter()
        .map(|(m, a, ce)| (m.round_ties_even() as i64, a.clone(), ce.to_bits()))
        .collect()
}

fn activation_in_filter(f: &str) -> Option<String> {
    let lo = f.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lo[from..].find('@') {
        let at = from + i + 1;
        for a in ["hcd", "cid", "etd", "ecd", "pqd", "uvpd"] {
            if lo[at..].starts_with(a) {
                return Some(a.to_uppercase());
            }
        }
        from = at;
    }
    None
}

fn immediate<'a>(
    precs: &'a [Prec],
    path: &Option<Vec<(f64, String, f64)>>,
    level: u8,
    level_of: &std::collections::HashMap<Arc<str>, u8>,
) -> Option<&'a Prec> {
    if precs.is_empty() {
        return None;
    }
    if let Some(p) = path {
        let last = p[p.len() - 1].0;
        if let Some(e) = precs.iter().find(|e| {
            [e.target, e.selected]
                .iter()
                .any(|v| v.is_some_and(|v| (v - last).abs() <= 0.01))
        }) {
            return Some(e);
        }
    }
    for e in precs {
        if let Some(r) = &e.refer {
            if !r.is_empty() && level_of.get(r).copied() == Some(level.wrapping_sub(1)) {
                return Some(e);
            }
        }
    }
    precs.first()
}

fn analyzer_from_components(names: &[String]) -> &'static str {
    let low = names
        .iter()
        .map(|n| n.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let n_q = low.matches("quadrupole").count();
    if low.contains("orbitrap") {
        "FTMS"
    } else if low.contains("time-of-flight") || low.contains("time of flight") {
        "TOFMS"
    } else if n_q >= 2 {
        "TQMS"
    } else if low.contains("ion trap") {
        "ITMS"
    } else if n_q > 0 {
        "SQMS"
    } else {
        "?"
    }
}

/// `mass_profile(scans)["hr"]` of the survey scans.
fn is_hr(rows: &[&Row]) -> bool {
    if rows.is_empty() {
        return false;
    }
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for r in rows {
        let a = if r.an.is_empty() { "?" } else { r.an };
        match counts.iter_mut().find(|(k, _)| *k == a) {
            Some(c) => c.1 += 1,
            None => counts.push((a, 1)),
        }
    }
    let mut best = counts[0];
    for c in &counts[1..] {
        if c.1 > best.1 {
            best = *c;
        }
    }
    match best.0 {
        "FTMS" | "TOFMS" => true,
        "?" => {
            let mut res: Vec<f64> = rows
                .iter()
                .filter_map(|r| r.res)
                .filter(|v| *v != 0.0)
                .collect();
            crate::sortc::by(&mut res, |a, b| a.total_cmp(b));
            !res.is_empty() && res[res.len() / 2] >= 10000.0
        }
        _ => false,
    }
}

/// Each spectrum's `[start, end)` and header text. Offsets come from the `indexList` when the file has a valid one, else the file is walked.
struct Source<'a, R: ReadAt> {
    r: &'a mut R,
    size: u64,
}

impl<R: ReadAt> Source<'_, R> {
    fn read(&mut self, a: u64, b: u64) -> Result<Vec<u8>> {
        let b = b.min(self.size);
        let mut v = Vec::new();
        v.try_reserve_exact((b.saturating_sub(a)) as usize)
            .map_err(|_| Error::memory_budget())?;
        v.resize((b.saturating_sub(a)) as usize, 0);
        self.r.read_at(a, &mut v)?;
        Ok(v)
    }

    /// `_spectrum_offsets`
    fn offsets(&mut self) -> Result<Option<Vec<u64>>> {
        let n = self.size;
        let tail = self.read(n.saturating_sub(2048), n)?;
        let Some(p) = find(&tail, b"<indexListOffset>") else {
            return Ok(None);
        };
        let after = &tail[p + 17..];
        let digits: Vec<u8> = after
            .iter()
            .copied()
            .skip_while(|c| c.is_ascii_whitespace())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let Ok(off) = std::str::from_utf8(&digits).unwrap_or("").parse::<u64>() else {
            return Ok(None);
        };
        if off >= n {
            return Ok(None);
        }
        let blob = self.read(off, n)?;
        let Some(a) = find(&blob, b"<index name=\"spectrum\"") else {
            return Ok(None);
        };
        let Some(gt) = blob[a..].iter().position(|c| *c == b'>') else {
            return Ok(None);
        };
        let body = &blob[a + gt + 1..];
        let end = find(body, b"</index>").unwrap_or(body.len());
        let body = &body[..end];
        let mut offs = Vec::new();
        let mut i = 0;
        while let Some(p) = find(&body[i..], b"<offset") {
            let s = i + p;
            let Some(g) = body[s..].iter().position(|c| *c == b'>') else {
                break;
            };
            let t: Vec<u8> = body[s + g + 1..]
                .iter()
                .copied()
                .skip_while(|c| c.is_ascii_whitespace())
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(v) = std::str::from_utf8(&t).unwrap_or("").parse::<u64>() {
                offs.push(v);
            }
            i = s + g + 1;
        }
        if offs.is_empty() || offs.windows(2).any(|w| w[1] <= w[0]) {
            return Ok(None);
        }
        for o in [offs[0], *offs.last().unwrap()] {
            if o + 10 > n || self.read(o, o + 10)? != b"<spectrum " {
                return Ok(None);
            }
        }
        Ok(Some(offs))
    }
}

fn find(h: &[u8], pat: &[u8]) -> Option<usize> {
    if pat.is_empty() || h.len() < pat.len() {
        return None;
    }
    let first = pat[0];
    let mut i = 0;
    while i + pat.len() <= h.len() {
        match h[i..=h.len() - pat.len()].iter().position(|c| *c == first) {
            None => return None,
            Some(p) => {
                i += p;
                if &h[i..i + pat.len()] == pat {
                    return Some(i);
                }
                i += 1;
            }
        }
    }
    None
}

mod json;
mod rows;

pub use json::Columns;
use rows::Head;
pub(crate) use rows::Rows;

fn push(rows: &mut Rows, a: u64, b: u64, h: &[u8]) {
    rows.spectrum(a, b, &String::from_utf8_lossy(h));
}

/// Reads the headers of the spectra whose offsets the file lists: the end of the last spectrum, or `None` when it cannot be found.
fn read_listed<R: ReadAt>(
    src: &mut Source<'_, R>,
    offs: &[u64],
    rows: &mut Rows,
) -> Result<Option<u64>> {
    let size = src.size;
    let mut last_end = 0u64;
    // end of the last spectrum
    let tail_from = *offs.last().unwrap();
    let mut n = 1 << 16;
    let mut last = None;
    loop {
        let chunk = src.read(tail_from, tail_from + n)?;
        if let Some(p) = find(&chunk, b"</spectrum>") {
            last = Some(tail_from + p as u64);
            break;
        }
        if tail_from + n >= size {
            break;
        }
        n *= 4;
    }
    if let Some(last) = last {
        let limits: Vec<u64> = offs[1..]
            .iter()
            .copied()
            .chain(std::iter::once(last + 11 + 1))
            .collect();
        for (&a, &limit) in offs.iter().zip(&limits) {
            let mut n: u64 = 4096;
            let (chunk, c) = loop {
                let chunk = src.read(a, (a + n).min(limit))?;
                let c = find(&chunk, b"<binaryDataArrayList");
                if c.is_some() || a + n >= limit {
                    break (chunk, c);
                }
                n *= 4;
            };
            let b = match c {
                None => {
                    let e = chunk.windows(11).rposition(|w| w == b"</spectrum>");
                    a + e.map_or(chunk.len(), |e| e + 11) as u64
                }
                Some(_) => {
                    let from = limit.saturating_sub(64).max(a);
                    let tail = src.read(from, limit)?;
                    match tail.windows(11).rposition(|w| w == b"</spectrum>") {
                        Some(e) => from + e as u64 + 11,
                        None => limit,
                    }
                }
            };
            push(rows, a, b, &chunk[..c.unwrap_or(chunk.len())]);
            last_end = b;
        }
        return Ok(Some(last_end));
    }
    Ok(None)
}

/// Finds the spectra by walking the file (no list of offsets); the estimate of the table is made every `EVERY` scans. Returns the end of the last one.
fn read_walked<R: ReadAt>(
    src: &mut Source<'_, R>,
    rows: &mut Rows,
    budget: Option<&MemBudget>,
) -> Result<u64> {
    let size = src.size;
    let mut last_end = 0u64;
    let mut pos = 0u64;
    let mut buf: Vec<u8> = Vec::new();
    let mut buf_off = 0u64;
    loop {
        // find "<spectrum " from pos
        let a = loop {
            let rel = pos.saturating_sub(buf_off) as usize;
            if buf_off <= pos && rel <= buf.len() {
                if let Some(p) = find(&buf[rel..], b"<spectrum ") {
                    break Some(pos + p as u64);
                }
            }
            if buf_off <= pos && buf_off + buf.len() as u64 >= size {
                break None;
            }
            let next = if buf_off <= pos && pos < buf_off + buf.len() as u64 {
                (buf_off + buf.len() as u64).saturating_sub(10).max(pos)
            } else {
                pos
            };
            if next >= size {
                break None;
            }
            buf = src.read(next, next + (1 << 20))?;
            buf_off = next;
            pos = next;
        };
        let Some(a) = a else { break };
        if let Some(b) = budget {
            stima::check_walk(b, rows.len(), a, size)?;
        }
        // find "</spectrum>" after a
        let mut n: u64 = 1 << 16;
        let b = loop {
            let chunk = src.read(a, a + n)?;
            if let Some(p) = find(&chunk, b"</spectrum>") {
                let b = a + p as u64 + 11;
                let hdr_end = find(&chunk[..p], b"<binaryDataArrayList").unwrap_or(p);
                push(rows, a, b, &chunk[..hdr_end]);
                break Some(b);
            }
            if a + n >= size {
                break None;
            }
            n *= 4;
        };
        let Some(b) = b else { break };
        last_end = b;
        pos = b;
    }
    Ok(last_end)
}

/// The chromatograms after the last spectrum.
fn count_chromatograms<R: ReadAt>(src: &mut Source<'_, R>, last_end: u64) -> Result<usize> {
    let size = src.size;
    let mut n_chrom = 0usize;
    let mut pos = last_end;
    while pos < size {
        let chunk = src.read(pos, (pos + (1 << 20)).min(size))?;
        let mut i = 0;
        while let Some(p) = find(&chunk[i..], b"<chromatogram ") {
            n_chrom += 1;
            i += p + 1;
        }
        if pos + chunk.len() as u64 >= size {
            break;
        }
        pos += chunk.len() as u64 - 13;
    }
    Ok(n_chrom)
}

/// Reads the scan table of an mzML of `size` bytes.
pub fn read<R: ReadAt>(r: &mut R, size: u64) -> Result<Table> {
    read_in(r, size, None)
}

/// `read` that refuses in order (`error.memoryBudget`, phase `table`) when the rows would not fit in `budget`: the estimate (`stima`) is made from the
/// count of the spectra in the index of offsets before the first row is kept, or from the progress of the walk when the file has no index.
pub fn read_in<R: ReadAt>(r: &mut R, size: u64, budget: Option<&MemBudget>) -> Result<Table> {
    let mut src = Source { r, size };
    // header: instrument configurations and the default one
    let head_end = {
        let first = src.read(0, size.min(4 << 20))?;
        find(&first, b"<run ").map(|p| p as u64)
    };
    let head_bytes = src.read(0, head_end.unwrap_or(200000.min(size)))?;
    let run_tag_lo = head_end.unwrap_or(0);
    let run_text = src.read(run_tag_lo, (run_tag_lo + 2000).min(size))?;
    let mut rows = Rows::with(Head::parse(
        &String::from_utf8_lossy(&head_bytes),
        &String::from_utf8_lossy(&run_text),
    ));
    drop(head_bytes);

    // the spans of the spectra
    let offs = src.offsets()?;
    if let Some(o) = &offs {
        if let Some(b) = budget {
            stima::check_table(b, o.len())?;
        }
        rows.reserve(o.len());
    }
    // each header becomes its row as soon as it is read: keeping the headers of every spectrum (about 1.5 KB each) would hold hundreds of MB
    let listed = match &offs {
        Some(o) => read_listed(&mut src, o, &mut rows)?,
        None => None,
    };
    let last_end = match listed {
        Some(e) => e,
        None => read_walked(&mut src, &mut rows, budget)?,
    };
    let n_chrom = count_chromatograms(&mut src, last_end)?;
    Ok(rows.finish(n_chrom))
}
