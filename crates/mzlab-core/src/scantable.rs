//! The scan table of an mzML: what `mzlab/reader/mzml.py` (`Run._index`, `_read_header`, `_link_parents`) reads from the headers of the
//! spectra, written as the compact JSON rows of `SCAN_COLUMNS` that the `.mzidx` stores. The data arrays are never decoded: only the
//! header of each spectrum (up to `<binaryDataArrayList`) is read. Text is searched as the Python regular expressions do, without a
//! regex engine (it would cost size in the WASM).

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::json::Out;

const ANALYZERS: [&str; 5] = ["FTMS", "ITMS", "TOFMS", "TQMS", "SQMS"];

#[derive(Debug, Clone, Default)]
struct Prec {
    target: Option<f64>,
    lo: Option<f64>,
    hi: Option<f64>,
    selected: Option<f64>,
    charge: Option<i64>,
    refer: Option<String>,
    act: Option<&'static str>,
    ce: Option<f64>,
    pint: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct Row {
    pub start: u64,
    pub end: u64,
    pub level: u8,
    pub rt: f64,
    pub polarity: i8,
    pub tic: f64,
    precursor: Option<f64>,
    ce: Option<f64>,
    pub filter: String,
    pub profile: bool,
    parent: Option<usize>,
    iso: Option<(f64, f64)>,
    act: Option<String>,
    res: Option<f64>,
    pub an: String,
    pint: Option<f64>,
    path: Option<Vec<(f64, String, f64)>>,
    precs: Vec<Prec>,
    charge: Option<i64>,
    native: String,
}

/// The table, with what the `run` object of the metadata needs.
#[derive(Debug, Default)]
pub struct Table {
    pub rows: Vec<Row>,
    pub nce: bool,
    pub hr1: bool,
    pub n_chromatograms: usize,
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

fn precursor_element(e: &str, refer: Option<String>) -> Prec {
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
    level_of: &std::collections::HashMap<String, u8>,
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
        let a = if r.an.is_empty() { "?" } else { r.an.as_str() };
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

/// Reads the scan table of an mzML of `size` bytes.
pub fn read<R: ReadAt>(r: &mut R, size: u64) -> Result<Table> {
    let mut src = Source { r, size };
    // header: instrument configurations and the default one
    let head_end = {
        let first = src.read(0, size.min(4 << 20))?;
        find(&first, b"<run ").map(|p| p as u64)
    };
    let head_bytes = src.read(0, head_end.unwrap_or(200000.min(size)))?;
    let head = String::from_utf8_lossy(&head_bytes).into_owned();
    let nce = head.contains("accession=\"MS:1000768\"");
    let mut ic: Vec<(String, &'static str)> = Vec::new();
    let mut from = 0;
    while let Some(i) = head[from..].find("<instrumentConfiguration") {
        let s = from + i;
        let after = &head[s + 24..];
        if !after.starts_with(char::is_whitespace) {
            from = s + 24;
            continue;
        }
        let Some(close) = after.find("</instrumentConfiguration>") else {
            break;
        };
        let block = &after[..close];
        from = s + 24 + close;
        let Some(id) = attr(block.split('>').next().unwrap_or(""), "id") else {
            continue;
        };
        let mut names = Vec::new();
        let mut a = 0;
        while let Some(p) = block[a..].find("<analyzer") {
            let st = a + p;
            let Some(e) = block[st..].find("</analyzer>") else {
                break;
            };
            let blk = &block[st..st + e];
            let mut c = 0;
            while let Some(q) = blk[c..].find("<cvParam ") {
                let ts = c + q;
                let te = blk[ts..].find('>').map_or(blk.len(), |g| ts + g);
                if let Some(n) = blk[ts..te].rfind("name=\"") {
                    let ns = ts + n + 6;
                    if let Some(ne) = blk[ns..].find('"') {
                        names.push(blk[ns..ns + ne].to_string());
                    }
                }
                c = te;
            }
            a = st + e;
        }
        ic.push((id.to_string(), analyzer_from_components(&names)));
    }
    let run_tag_lo = head_end.unwrap_or(0);
    let run_text =
        String::from_utf8_lossy(&src.read(run_tag_lo, (run_tag_lo + 2000).min(size))?).into_owned();
    let default_ic = run_text
        .find("<run")
        .and_then(|p| {
            let t = &run_text[p..];
            let tag = &t[..t.find('>').unwrap_or(t.len())];
            attr(tag, "defaultInstrumentConfigurationRef").map(String::from)
        })
        .filter(|d| ic.iter().any(|(k, _)| k == d))
        .or_else(|| ic.first().map(|(k, _)| k.clone()));
    let analyzer = |filt: &str, icref: Option<&str>| -> String {
        let tok = filt.split(' ').next().unwrap_or("").to_uppercase();
        if ANALYZERS.contains(&tok.as_str()) {
            return tok;
        }
        let look = |k: &str| ic.iter().find(|(n, _)| n == k).map(|(_, a)| *a);
        look(icref.unwrap_or(""))
            .or_else(|| look(default_ic.as_deref().unwrap_or("")))
            .unwrap_or("?")
            .to_string()
    };

    // the spans of the spectra
    let offs = src.offsets()?;
    let mut spans: Vec<(u64, u64, String)> = Vec::new();
    let mut last_end = 0u64;
    fn push(spans: &mut Vec<(u64, u64, String)>, a: u64, b: u64, h: &[u8]) {
        spans.push((a, b, String::from_utf8_lossy(h).into_owned()));
    }
    let mut done = false;
    if let Some(offs) = offs {
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
                push(&mut spans, a, b, &chunk[..c.unwrap_or(chunk.len())]);
                last_end = b;
            }
            done = true;
        }
    }
    if !done {
        spans.clear();
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
            // find "</spectrum>" after a
            let mut n: u64 = 1 << 16;
            let b = loop {
                let chunk = src.read(a, a + n)?;
                if let Some(p) = find(&chunk, b"</spectrum>") {
                    let b = a + p as u64 + 11;
                    let hdr_end = find(&chunk[..p], b"<binaryDataArrayList").unwrap_or(p);
                    push(&mut spans, a, b, &chunk[..hdr_end]);
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
    }

    // the rows
    let mut rows: Vec<Row> = Vec::with_capacity(spans.len());
    let mut refs: std::collections::HashMap<usize, String> = Default::default();
    let mut level_of: std::collections::HashMap<String, u8> = Default::default();
    for (a, b, h) in &spans {
        let id = {
            // first ` id="..."` (\sid="...")
            let mut from = 0;
            let mut found = None;
            while let Some(p) = h[from..].find("id=\"") {
                let at = from + p;
                if at > 0 && h.as_bytes()[at - 1].is_ascii_whitespace() {
                    let s = at + 4;
                    if let Some(e) = h[s..].find('"') {
                        found = Some(h[s..s + e].to_string());
                    }
                    break;
                }
                from = at + 3;
            }
            found
        };
        let lvl = cv(h, "MS:1000511").filter(|v| !v.is_empty());
        let pol = if h.contains("accession=\"MS:1000130\"") {
            1
        } else if h.contains("accession=\"MS:1000129\"") {
            -1
        } else {
            0
        };
        let mut rt = cv(h, "MS:1000016").and_then(float).unwrap_or(0.0);
        if let Some(p) = h.find("accession=\"MS:1000016\"") {
            let tag_end = h[p..].find('>').map_or(h.len(), |g| p + g);
            if let Some(u) = attr(&h[p..tag_end], "unitName") {
                if u.starts_with("second") {
                    rt /= 60.0;
                }
            }
        }
        let level = lvl.and_then(float).map_or(1, |v| v as u8);
        let hs = h.split("<precursorList").next().unwrap_or("");
        let filt = cv(hs, "MS:1000512").unwrap_or("").to_string();
        let (mut prec, mut iso, mut act, mut pint, mut path, mut ce) =
            (None, None, None::<String>, None, None, None);
        let mut precs: Vec<Prec> = Vec::new();
        let mut imm_charge = None;
        if level > 1 {
            path = filter_path(&filt);
            let mut from = 0;
            while let Some(p) = h[from..].find("<precursor") {
                let s = from + p;
                let after = s + 10;
                let nxt = h.as_bytes().get(after).copied().unwrap_or(b'>');
                if nxt.is_ascii_alphanumeric() || nxt == b'_' {
                    from = after;
                    continue;
                }
                let Some(g) = h[after..].find('>') else { break };
                let attrs = &h[after..after + g];
                let Some(close) = h[after + g + 1..].find("</precursor>") else {
                    break;
                };
                let body = &h[after + g + 1..after + g + 1 + close];
                let rf = attr(attrs, "spectrumRef").map(String::from);
                precs.push(precursor_element(body, rf));
                from = after + g + 1 + close + 12;
            }
            if precs.is_empty() && (h.contains("MS:1000744") || h.contains("MS:1000827")) {
                precs.push(precursor_element(h, None));
            }
            if let Some(m) = immediate(&precs, &path, level, &level_of) {
                prec = m.selected.or(m.target);
                let tgt = m.target.or(prec);
                if let Some(t) = tgt {
                    let lo = match (m.lo, m.target) {
                        (Some(_), Some(tg)) => tg - m.lo.unwrap(),
                        _ => 0.5,
                    };
                    let hi = match (m.hi, m.target) {
                        (Some(_), Some(tg)) => m.hi.unwrap() - tg,
                        _ => 0.5,
                    };
                    iso = Some((t - lo, t + hi));
                }
                act = m.act.map(String::from);
                pint = m.pint;
                ce = m.ce;
                imm_charge = m.charge;
            }
            if act.is_none() {
                act = match &path {
                    Some(p) => Some(p[p.len() - 1].1.clone()),
                    None => activation_in_filter(&filt),
                };
            }
            if let Some(r) = precs
                .iter()
                .find_map(|e| e.refer.clone().filter(|r| !r.is_empty()))
            {
                refs.insert(rows.len(), r);
            }
        }
        if ce.is_none() && h.contains("MS:1000045") {
            ce = cv(h, "MS:1000045").and_then(float);
        }
        let icref = hs.find("<scan").and_then(|p| {
            let t = &hs[p..];
            let tag = &t[..t.find('>').unwrap_or(t.len())];
            attr(tag, "instrumentConfigurationRef")
        });
        let res = cv(hs, "MS:1000800").and_then(float).or_else(|| {
            let pat = "<userParam name=\"opentfraw.resolution\" value=\"";
            h.find(pat).and_then(|p| {
                let s = p + pat.len();
                h[s..].find('"').and_then(|e| float(&h[s..s + e]))
            })
        });
        let row = Row {
            start: *a,
            end: *b,
            level,
            rt,
            polarity: pol,
            tic: cv(h, "MS:1000285")
                .and_then(float)
                .filter(|v| *v != 0.0)
                .unwrap_or(0.0),
            precursor: prec,
            ce,
            an: analyzer(&filt, icref),
            filter: filt,
            profile: h.contains("accession=\"MS:1000128\""),
            parent: None,
            iso,
            act,
            res,
            pint,
            path,
            precs,
            charge: imm_charge,
            native: id.clone().unwrap_or_default(),
        };
        if let Some(i) = id {
            level_of.insert(i, level);
        }
        rows.push(row);
    }

    // _link_parents
    let by_native: std::collections::HashMap<&str, usize> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| (r.native.as_str(), i))
        .collect();
    let mut parents: Vec<Option<usize>> = vec![None; rows.len()];
    let mut last: std::collections::HashMap<i8, usize> = Default::default();
    let mut last_path: std::collections::HashMap<(u8, Vec<(i64, String, u64)>), usize> =
        Default::default();
    for (i, s) in rows.iter().enumerate() {
        if s.level == 1 {
            last.insert(s.polarity, i);
            continue;
        }
        if s.level == 2 {
            let mut p = refs
                .get(&i)
                .and_then(|r| by_native.get(r.as_str()).copied());
            if p.is_none() {
                p = last.get(&s.polarity).or_else(|| last.get(&0)).copied();
            }
            parents[i] = p;
        } else if let Some(pa) = &s.path {
            parents[i] = last_path
                .get(&(s.level - 1, path_key(&pa[..pa.len() - 1])))
                .copied();
        } else {
            let j = refs
                .get(&i)
                .and_then(|r| by_native.get(r.as_str()).copied());
            parents[i] = j.filter(|&j| rows[j].level == s.level - 1);
        }
        if let Some(pa) = &s.path {
            last_path.insert((s.level, path_key(pa)), i);
        }
    }
    for (r, p) in rows.iter_mut().zip(parents) {
        r.parent = p;
    }
    let hr1 = is_hr(&rows.iter().filter(|r| r.level == 1).collect::<Vec<_>>());

    // chromatograms after the last spectrum
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
    Ok(Table {
        rows,
        nce,
        hr1,
        n_chromatograms: n_chrom,
    })
}

fn py_float(o: &mut Out, v: f64) {
    if !v.is_finite() {
        o.raw("null");
        return;
    }
    let s = format!("{v:?}");
    if let Some((m, e)) = s.split_once('e') {
        let n: i32 = e.parse().unwrap_or(0);
        o.raw(&format!(
            "{m}e{}{:02}",
            if n < 0 { '-' } else { '+' },
            n.abs()
        ));
    } else {
        o.raw(&s);
    }
}

fn opt(o: &mut Out, v: Option<f64>) {
    match v {
        Some(x) => py_float(o, x),
        None => {
            o.raw("null");
        }
    }
}

impl Table {
    /// The rows of `SCAN_COLUMNS` as compact JSON (`json.dumps(rows, separators=(",", ":"))`).
    pub fn rows_json(&self) -> String {
        let mut o = Out::default();
        o.raw("[");
        for (i, r) in self.rows.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.raw("[")
                .int(r.start as i64)
                .raw(",")
                .int(r.end as i64)
                .raw(",")
                .int(r.level as i64)
                .raw(",");
            py_float(&mut o, r.rt);
            o.raw(",").int(r.polarity as i64).raw(",");
            py_float(&mut o, r.tic);
            o.raw(",");
            opt(&mut o, r.precursor);
            o.raw(",");
            opt(&mut o, r.ce);
            o.raw(",")
                .str(&r.filter)
                .raw(if r.profile { ",true," } else { ",false," });
            match r.parent {
                Some(p) => o.int(p as i64),
                None => o.raw("null"),
            };
            o.raw(",");
            match r.iso {
                Some((a, b)) => {
                    o.raw("[");
                    py_float(&mut o, a);
                    o.raw(",");
                    py_float(&mut o, b);
                    o.raw("]");
                }
                None => {
                    o.raw("null");
                }
            }
            o.raw(",");
            match &r.act {
                Some(a) => o.str(a),
                None => o.raw("null"),
            };
            o.raw(",");
            opt(&mut o, r.res);
            o.raw(",").str(&r.an).raw(",");
            opt(&mut o, r.pint);
            o.raw(",");
            match &r.path {
                Some(p) => {
                    o.raw("[");
                    for (j, (m, a, c)) in p.iter().enumerate() {
                        if j > 0 {
                            o.raw(",");
                        }
                        o.raw("[");
                        py_float(&mut o, *m);
                        o.raw(",").str(a).raw(",");
                        py_float(&mut o, *c);
                        o.raw("]");
                    }
                    o.raw("]");
                }
                None => {
                    o.raw("null");
                }
            }
            o.raw(",[");
            for (j, e) in r.precs.iter().enumerate() {
                if j > 0 {
                    o.raw(",");
                }
                o.raw("{\"target\":");
                opt(&mut o, e.target);
                o.raw(",\"lo\":");
                opt(&mut o, e.lo);
                o.raw(",\"hi\":");
                opt(&mut o, e.hi);
                o.raw(",\"selected\":");
                opt(&mut o, e.selected);
                o.raw(",\"charge\":");
                match e.charge {
                    Some(c) => o.int(c),
                    None => o.raw("null"),
                };
                o.raw(",\"ref\":");
                match &e.refer {
                    Some(s) => o.str(s),
                    None => o.raw("null"),
                };
                o.raw(",\"act\":");
                match e.act {
                    Some(s) => o.str(s),
                    None => o.raw("null"),
                };
                o.raw(",\"ce\":");
                opt(&mut o, e.ce);
                o.raw(",\"pint\":");
                opt(&mut o, e.pint);
                o.raw("}");
            }
            o.raw("],");
            match r.charge {
                Some(c) => o.int(c),
                None => o.raw("null"),
            };
            o.raw(",").str(&r.native).raw("]");
        }
        o.raw("]");
        o.0
    }
}
