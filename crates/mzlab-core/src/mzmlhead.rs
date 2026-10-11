//! What an mzML says outside its spectra: the chromatogram list (TIC, BPC, SRM, PDA, pressure and other traces), the arrays of one
//! chromatogram, the instrument metadata and the polarity written in the header (`Run.chromatograms`, `chromatogram`, `chromatogram_unit`,
//! `metadata`, `_read_header` and `Item._header_polarity` of `mzlab/reader/mzml.py`). Text is searched as the Python regular expressions do,
//! without a regex engine (it would cost size in the WASM). The bytes come through a `ReadAt`.

use std::io::Read;

use base64::Engine;

use crate::error::{Error, Result};
use crate::idxread::ReadAt;

const MAX_REGION: u64 = 256 << 20;

fn find(h: &[u8], n: &[u8], from: usize) -> Option<usize> {
    if from > h.len() || n.is_empty() {
        return None;
    }
    h[from..]
        .windows(n.len())
        .position(|w| w == n)
        .map(|p| p + from)
}
fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `name="..."` as the LAST `name="` of a tag head (the regex `<cvParam [^>]*name="([^"]*)"` is greedy), for every `<cvParam ` of `block`.
fn names(block: &str) -> Vec<String> {
    let mut out = vec![];
    let mut rest = block;
    while let Some(p) = rest.find("<cvParam ") {
        let tag = &rest[p + 9..];
        let end = tag.find('>').unwrap_or(tag.len());
        let head = &tag[..end];
        match head.rfind("name=\"") {
            Some(q) => {
                let v = &head[q + 6..];
                match v.find('"') {
                    Some(e) => {
                        out.push(v[..e].to_string());
                        rest = &tag[q + 6 + e + 1..];
                    }
                    None => rest = &tag[end..],
                }
            }
            None => rest = &tag[end..],
        }
    }
    out
}

/// The text of every `<kind ...>...</kind>` block in `s` (the regex `<kind .*?</kind>` with DOTALL).
fn blocks<'a>(s: &'a str, kind: &str) -> Vec<&'a str> {
    let (open, close) = (format!("<{kind} "), format!("</{kind}>"));
    let mut out = vec![];
    let mut rest = s;
    while let Some(a) = rest.find(&open) {
        match rest[a..].find(&close) {
            Some(b) => {
                out.push(&rest[a..a + b + close.len()]);
                rest = &rest[a + b + close.len()..];
            }
            None => break,
        }
    }
    out
}

/// `attr="value"` of the first occurrence in `s` (value up to the next quote).
fn attr<'a>(s: &'a str, a: &str) -> Option<&'a str> {
    let k = format!("{a}=\"");
    let mut from = 0;
    while let Some(p) = s[from..].find(&k) {
        let at = from + p;
        let ok = at == 0 || !s[..at].chars().next_back().is_some_and(is_word);
        if ok {
            let v = &s[at + k.len()..];
            return v.find('"').map(|e| &v[..e]);
        }
        from = at + 1;
    }
    None
}

fn pyfloat(s: &str) -> Option<f64> {
    s.trim().parse().ok()
}

/// Header of one `<chromatogram>`: id, kind and, for SRM, Q1, Q3, collision energy, name, dwell time.
#[derive(Debug, Clone, Default)]
pub struct Chrom {
    pub index: usize,
    pub id: String,
    pub start: u64,
    pub end: u64,
    pub kind: String,
    pub q1: Option<f64>,
    pub q3: Option<f64>,
    pub ce: Option<f64>,
    pub name: String,
    pub dwell: Option<f64>,
    pub n: Option<i64>,
}

/// `Q1=([0-9.]+)` and its kind: the number after the first occurrence of `key` that is followed by digits or dots.
fn after(id: &str, key: &str) -> Option<f64> {
    let mut from = 0;
    while let Some(p) = id[from..].find(key) {
        let v = &id[from + p + key.len()..];
        let n = v
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .count();
        if n > 0 {
            return pyfloat(&v[..n]);
        }
        from += p + 1;
    }
    None
}

pub struct Mzml<R: ReadAt> {
    pub r: R,
    pub size: u64,
}

impl<R: ReadAt> Mzml<R> {
    pub fn new(r: R, size: u64) -> Self {
        Mzml { r, size }
    }

    fn read(&mut self, a: u64, b: u64) -> Result<Vec<u8>> {
        let b = b.min(self.size);
        if b <= a {
            return Ok(vec![]);
        }
        if b - a > MAX_REGION {
            return Err(Error::memory_budget());
        }
        let mut v = Vec::new();
        v.try_reserve_exact((b - a) as usize)
            .map_err(|_| Error::memory_budget())?;
        v.resize((b - a) as usize, 0);
        self.r.read_at(a, &mut v)?;
        Ok(v)
    }

    /// The header of every `<chromatogram>`; they follow the spectra, so the search starts where the last scan ends (0 without scans).
    pub fn chromatograms(&mut self, scans_end: u64) -> Result<Vec<Chrom>> {
        let region = self.read(scans_end, self.size)?;
        let mut out: Vec<Chrom> = vec![];
        let mut pos = 0usize;
        while let Some(a) = find(&region, b"<chromatogram ", pos) {
            let Some(b) = find(&region, b"</chromatogram>", a) else {
                break;
            };
            let c = find(&region[..b], b"<binaryDataArrayList", a);
            let h = text(&region[a..c.unwrap_or(b)]);
            let cid = attr_id(&h).unwrap_or_default();
            let mut d = Chrom {
                index: out.len(),
                id: cid.clone(),
                start: scans_end + a as u64,
                end: scans_end + (b + 15) as u64,
                kind: "other".into(),
                ..Default::default()
            };
            let acc = |x: &str| h.contains(&format!("accession=\"{x}\""));
            if cid == "TIC" {
                d.kind = "tic".into();
            } else if cid == "BPC" {
                d.kind = "bpc".into();
            } else if cid == "TWC" || acc("MS:1000812") || acc("MS:1000813") {
                d.kind = "pda".into();
            } else if acc("MS:1003019") || cid.starts_with("Pump Pressure") {
                d.kind = "pressure".into();
            } else if cid.starts_with("SRM") {
                d.kind = "srm".into();
                d.q1 = after(&cid, "Q1=");
                d.q3 = after(&cid, "Q3=");
                d.ce = after(&cid, "ce=");
                d.name = cid
                    .find("name=")
                    .map(|p| cid[p + 5..].lines().next().unwrap_or("").trim().to_string())
                    .unwrap_or_default();
                d.dwell = dwell(&h);
            } else if ![
                "MS:1000235",
                "MS:1000627",
                "MS:1000628",
                "MS:1001473",
                "MS:1001472",
            ]
            .iter()
            .any(|x| acc(x))
            {
                d.kind = "analog".into();
            }
            d.n = attr(&h, "defaultArrayLength").and_then(|v| v.parse().ok());
            out.push(d);
            pos = b + 1;
        }
        Ok(out)
    }

    /// (time in minutes, intensity) of a chromatogram.
    pub fn chromatogram(&mut self, c: &Chrom) -> Result<(Vec<f64>, Vec<f64>)> {
        let chunk = self.read(c.start, c.end)?;
        let (mut t, mut y): (Option<Vec<f64>>, Option<Vec<f64>>) = (None, None);
        for (head, data) in arrays(&chunk) {
            let a = decode(&head, &data)?;
            if head.contains("MS:1000595") {
                t.get_or_insert(a);
            } else {
                y.get_or_insert(a);
            }
        }
        let (mut t, y) = (t.unwrap_or_default(), y.unwrap_or_default());
        let n = t.len().min(y.len());
        let list = find(&chunk, b"<binaryDataArrayList", 0).unwrap_or(chunk.len());
        let seconds = text(&chunk[..list]).contains("unitName=\"second\"")
            || find(
                &chunk,
                b"name=\"time array\" value=\"\" unitCvRef=\"UO\" unitAccession=\"UO:0000010\"",
                0,
            )
            .is_some();
        if seconds {
            t.iter_mut().for_each(|v| *v /= 60.0);
        }
        t.truncate(n);
        Ok((t, y[..n].to_vec()))
    }

    /// The unit written on the intensity array of a chromatogram, or "".
    pub fn chromatogram_unit(&mut self, c: &Chrom) -> Result<String> {
        let chunk = self.read(c.start, c.end)?;
        for (head, _) in arrays(&chunk) {
            if !head.contains("MS:1000595") {
                if let Some(u) = attr(&head, "unitName") {
                    return Ok(u.to_string());
                }
            }
        }
        Ok(String::new())
    }

    /// The part of the file before `<run `, as text (the header the instrument facts come from).
    fn head(&mut self) -> Result<String> {
        let probe = self.read(0, self.size.min(MAX_REGION))?;
        let end = find(&probe, b"<run ", 0).unwrap_or_else(|| probe.len().saturating_sub(1));
        Ok(text(&probe[..end]))
    }

    /// Polarity written in the header (used when a file has no scans, e.g. MRM): positive, negative, mixed or unknown.
    pub fn header_polarity(&mut self) -> Result<String> {
        let all = self.read(0, self.size.min(MAX_REGION))?;
        let end = find(&all, b"</run>", 0).unwrap_or_else(|| all.len().saturating_sub(1));
        let h = &all[..end];
        let (pos, neg) = (
            find(h, b"accession=\"MS:1000130\"", 0).is_some(),
            find(h, b"accession=\"MS:1000129\"", 0).is_some(),
        );
        Ok(match (pos, neg) {
            (true, false) => "positive",
            (false, true) => "negative",
            (true, true) => "mixed",
            _ => "unknown",
        }
        .into())
    }

    /// Thermo nativeID format in the header: the "collision energy" is relative (NCE), not eV.
    pub fn nce(&mut self) -> Result<bool> {
        let probe = self.read(0, self.size.min(200_000))?;
        let end = match find(&probe, b"<run ", 0) {
            Some(p) if p > 0 => p,
            _ => probe.len(),
        };
        Ok(find(&probe[..end], b"accession=\"MS:1000768\"", 0).is_some())
    }

    /// Instrument, serial, components, analyzers, source files and software from the header (`Run.metadata`).
    pub fn metadata(&mut self) -> Result<Metadata> {
        let head = self.head()?;
        let ic = head.find("<instrumentConfiguration").and_then(|a| {
            head[a..]
                .find("</instrumentConfiguration>")
                .map(|b| &head[a..a + b + 25])
        });
        const SERIAL: &str = "name=\"instrument serial number\" value=\"";
        let serial = head.find(SERIAL).map(|p| {
            let v = &head[p + SERIAL.len()..];
            v[..v.find('"').unwrap_or(0)].to_string()
        });
        let own = ic
            .map(|s| s.split("<componentList").next().unwrap_or(""))
            .unwrap_or("");
        let mut pool = names(own);
        let mut rest = own;
        while let Some(p) = rest.find("<referenceableParamGroupRef") {
            let t = &rest[p + 27..];
            let t2 = t.trim_start();
            if t2.len() < t.len() && t2.starts_with("ref=\"") {
                let r = &t2[5..];
                if let Some(e) = r.find('"') {
                    let id = &r[..e];
                    let open = format!("id=\"{id}\"");
                    if let Some(g) =
                        head.match_indices("<referenceableParamGroup")
                            .find(|(i, _)| {
                                let after = &head[i + 24..];
                                let a2 = after.trim_start();
                                a2.len() < after.len()
                                    && a2.starts_with(&open)
                                    && head[i + 24..]
                                        .chars()
                                        .next()
                                        .is_some_and(char::is_whitespace)
                            })
                    {
                        let body = &head[g.0..];
                        let from = body.find(&open).unwrap() + open.len();
                        let to = body
                            .find("</referenceableParamGroup>")
                            .unwrap_or(body.len());
                        if from <= to {
                            pool.extend(names(&body[from..to]));
                        }
                    }
                }
            }
            rest = t;
        }
        pool.retain(|n| n != "instrument serial number");
        let mut src = vec![];
        let mut rest = head.as_str();
        while let Some(p) = rest.find("<sourceFile id=\"") {
            let t = &rest[p + 16..];
            if let Some(e) = t.find('"') {
                let after = &t[e + 1..];
                if let Some(nm) = after.strip_prefix(" name=\"") {
                    if let Some(e2) = nm.find('"') {
                        src.push(nm[..e2].to_string());
                        rest = &nm[e2..];
                        continue;
                    }
                }
            }
            rest = t;
        }
        let mut soft = vec![];
        let mut rest = head.as_str();
        while let Some(p) = rest.find("<software id=\"") {
            let t = &rest[p + 14..];
            rest = t;
            let Some(e) = t.find('"') else { continue };
            let Some(v) = t[e + 1..].strip_prefix(" version=\"") else {
                continue;
            };
            let Some(e2) = v.find('"') else { continue };
            let Some(tail) = v[e2 + 1..].strip_prefix('>') else {
                continue;
            };
            let Some(cv) = tail.trim_start().strip_prefix("<cvParam ") else {
                continue;
            };
            let head_end = cv.find('>').unwrap_or(cv.len());
            if let Some(q) = cv[..head_end].rfind("name=\"") {
                let nv = &cv[q + 6..];
                if let Some(e3) = nv.find('"') {
                    soft.push((v[..e2].to_string(), nv[..e3].to_string()));
                }
            }
        }
        let mut comp = vec![];
        if let Some(ic) = ic {
            for kind in ["source", "analyzer", "detector"] {
                for b in blocks(ic, kind) {
                    comp.extend(names(b));
                }
            }
        }
        let mut analyzers: Vec<String> = self.analyzers()?.into_iter().map(|(_, a)| a).collect();
        analyzers.sort();
        analyzers.dedup();
        Ok(Metadata {
            instrument: pool.first().cloned().unwrap_or_default(),
            serial: serial.unwrap_or_default(),
            components: comp,
            analyzers,
            source_files: src,
            software: soft
                .into_iter()
                .map(|(v, n)| format!("{n} {v}").trim().to_string())
                .collect(),
        })
    }

    /// (instrumentConfiguration id, analyzer code) in file order, and nothing else (`Run._read_header`).
    pub fn analyzers(&mut self) -> Result<Vec<(String, String)>> {
        let probe = self.read(0, self.size.min(MAX_REGION))?;
        let end = match find(&probe, b"<run ", 0) {
            Some(p) if p > 0 => p,
            _ => probe.len().min(200_000),
        };
        let head = text(&probe[..end]);
        let mut out = vec![];
        let mut rest = head.as_str();
        while let Some(p) = rest.find("<instrumentConfiguration") {
            let t = &rest[p + 24..];
            rest = t;
            let t2 = t.trim_start();
            if t2.len() == t.len() || !t2.starts_with("id=\"") {
                continue;
            }
            let r = &t2[4..];
            let Some(e) = r.find('"') else { continue };
            let Some(close) = r[e..].find("</instrumentConfiguration>") else {
                continue;
            };
            let body = &r[e + 1..e + close];
            let mut an = vec![];
            for b in analyzer_blocks(body) {
                an.extend(names(b));
            }
            out.push((r[..e].to_string(), analyzer_from_components(&an)));
        }
        Ok(out)
    }
}

/// `<analyzer\b.*?</analyzer>`
fn analyzer_blocks(s: &str) -> Vec<&str> {
    let mut out = vec![];
    let mut rest = s;
    while let Some(a) = rest.find("<analyzer") {
        let boundary = !rest[a + 9..].chars().next().is_some_and(is_word);
        match rest[a..].find("</analyzer>") {
            Some(b) if boundary => {
                out.push(&rest[a..a + b + 11]);
                rest = &rest[a + b + 11..];
            }
            _ => rest = &rest[a + 9..],
        }
    }
    out
}

/// Thermo-style analyzer code from the names of the analyzer components of an instrumentConfiguration.
pub fn analyzer_from_components(names: &[String]) -> String {
    let low = names
        .iter()
        .map(|n| n.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let n_q = low.matches("quadrupole").count();
    if low.contains("orbitrap") {
        "FTMS".into()
    } else if low.contains("time-of-flight") || low.contains("time of flight") {
        "TOFMS".into()
    } else if n_q >= 2 {
        "TQMS".into()
    } else if low.contains("ion trap") {
        "ITMS".into()
    } else if n_q > 0 {
        "SQMS".into()
    } else {
        "?".into()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub instrument: String,
    pub serial: String,
    pub components: Vec<String>,
    pub analyzers: Vec<String>,
    pub source_files: Vec<String>,
    pub software: Vec<String>,
}

/// `\sid="([^"]*)"`
fn attr_id(h: &str) -> Option<String> {
    let mut from = 0;
    while let Some(p) = h[from..].find("id=\"") {
        let at = from + p;
        if at > 0 && h[..at].chars().next_back().is_some_and(char::is_whitespace) {
            let v = &h[at + 4..];
            return v.find('"').map(|e| v[..e].to_string());
        }
        from = at + 1;
    }
    None
}

/// `name="?MS_dwell_time"?\s+value="([^"]*)"`
fn dwell(h: &str) -> Option<f64> {
    let mut from = 0;
    while let Some(p) = h[from..].find("MS_dwell_time") {
        let at = from + p;
        let before_ok = h[..at].ends_with("name=\"") || h[..at].ends_with("name=");
        if before_ok {
            let mut t = &h[at + 13..];
            t = t.strip_prefix('"').unwrap_or(t);
            let t2 = t.trim_start();
            if t2.len() < t.len() {
                if let Some(v) = t2.strip_prefix("value=\"") {
                    return v.find('"').and_then(|e| pyfloat(&v[..e]));
                }
            }
        }
        from = at + 1;
    }
    None
}

/// Every `<binaryDataArray ...>...</binaryDataArray>`: (the head up to `<binary>`, the base64 text).
fn arrays(chunk: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut out = vec![];
    let mut pos = 0;
    while let Some(a) = find(chunk, b"<binaryDataArray ", pos) {
        let Some(b) = find(chunk, b"</binaryDataArray>", a) else {
            break;
        };
        let t = &chunk[a..b + 18];
        let k = find(t, b"<binary>", 0);
        let head = text(&t[..k.unwrap_or(t.len())]);
        let data = match (k, find(t, b"</binary>", 0)) {
            (Some(k), Some(e)) if e >= k + 8 => t[k + 8..e].to_vec(),
            _ => vec![],
        };
        out.push((head, data));
        pos = b + 18;
    }
    out
}

fn decode(head: &str, data: &[u8]) -> Result<Vec<f64>> {
    let clean: Vec<u8> = data
        .iter()
        .copied()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    let mut raw = base64::engine::general_purpose::STANDARD
        .decode(&clean)
        .map_err(|e| Error::new("err.file.base64").with("detail", e))?;
    if !raw.is_empty() && head.contains("accession=\"MS:1000574\"") {
        let mut out = vec![];
        flate2::read::ZlibDecoder::new(&raw[..])
            .read_to_end(&mut out)
            .map_err(|e| Error::new("err.file.zlib").with("detail", e))?;
        raw = out;
    }
    Ok(if head.contains("accession=\"MS:1000521\"") {
        raw.chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()) as f64)
            .collect()
    } else {
        raw.chunks_exact(8)
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect()
    })
}
