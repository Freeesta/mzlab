//! The rows of the scan table, one spectrum header at a time: from the headers of an mzML (`read`) or the headers `mzmlwrite` would write
//! for the scans of a `Lettore` (`mzidx_lettore`).

use std::collections::HashMap;

use super::*;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, SpectrumExtra};

/// A level and the key of a filter path (`path_key`).
type PathKey = (u8, Vec<(i64, String, u64)>);

/// What the file header says that the rows need: the analyzers of the instrument configurations and the default one, and `run.nce`.
pub(super) struct Head {
    nce: bool,
    ic: Vec<(String, &'static str)>,
    default_ic: Option<String>,
}

impl Head {
    /// `head`: the text before `<run `; `run_text`: the text from `<run ` on (2000 bytes are enough).
    pub(super) fn parse(head: &str, run_text: &str) -> Head {
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
            ic.push((
                id.to_string(),
                analyzer_from_components(&analyzer_names(block)),
            ));
        }
        let default_ic = run_text
            .find("<run")
            .and_then(|p| {
                let t = &run_text[p..];
                let tag = &t[..t.find('>').unwrap_or(t.len())];
                attr(tag, "defaultInstrumentConfigurationRef").map(String::from)
            })
            .filter(|d| ic.iter().any(|(k, _)| k == d))
            .or_else(|| ic.first().map(|(k, _)| k.clone()));
        Head {
            nce,
            ic,
            default_ic,
        }
    }

    /// What `parse` reads from the header `mzmlwrite` writes for `h`, without writing it: the one instrument configuration `IC1` (the analyzers are the
    /// `analyzer` components) and `run.nce`. `None` when a name has a character that the text would break on (the caller then goes through the text).
    pub(super) fn da_testata(h: &Header) -> Option<Head> {
        let bad = |t: &str| t.contains(['"', '<', '>']);
        let mut names: Vec<&str> = Vec::new();
        let mut accs: Vec<&str> = Vec::new();
        let mut free: Vec<&str> = Vec::new();
        for (kind, acc, name) in &h.components {
            if kind == "analyzer" {
                names.push(name);
            }
            free.extend([kind.as_str(), name.as_str()]);
            accs.push(acc);
        }
        let sources = if h.sources.is_empty() {
            &[][..]
        } else {
            &h.source_cv[..]
        };
        for (a, n) in h.content.iter().chain(sources).chain(h.instrument.iter()) {
            accs.push(a);
            free.push(n);
        }
        for (a, _, _) in &h.extra_software {
            accs.push(a);
        }
        if accs.iter().chain(&free).any(|t| bad(t)) {
            return None;
        }
        let has_nce = accs.contains(&"MS:1000768");
        let names: Vec<String> = names.into_iter().map(String::from).collect();
        Some(Head {
            nce: has_nce,
            ic: vec![("IC1".to_string(), analyzer_from_components(&names))],
            default_ic: Some("IC1".to_string()),
        })
    }

    fn analyzer(&self, filt: &str, icref: Option<&str>) -> String {
        let tok = filt.split(' ').next().unwrap_or("").to_uppercase();
        if ANALYZERS.contains(&tok.as_str()) {
            return tok;
        }
        let look = |k: &str| self.ic.iter().find(|(n, _)| n == k).map(|(_, a)| *a);
        look(icref.unwrap_or(""))
            .or_else(|| look(self.default_ic.as_deref().unwrap_or("")))
            .unwrap_or("?")
            .to_string()
    }
}

/// The `name` of every `cvParam` of the `<analyzer>` elements of an instrument configuration.
fn analyzer_names(block: &str) -> Vec<String> {
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
    names
}

/// The rows of a table, one spectrum header at a time (the text from `<spectrum ` up to `<binaryDataArrayList`), linked to their parents by `finish`.
/// `read` feeds it the headers of an mzML; `mzidx_lettore` the headers `mzmlwrite` would write for the scans of a `Lettore`.
pub(crate) struct Rows {
    head: Head,
    rows: Vec<Row>,
    refs: HashMap<usize, String>,
    level_of: HashMap<String, u8>,
}

/// The retention time of a spectrum header, in minutes.
fn rt_minutes(h: &str) -> f64 {
    let mut rt = cv(h, "MS:1000016").and_then(float).unwrap_or(0.0);
    if let Some(p) = h.find("accession=\"MS:1000016\"") {
        let tag_end = h[p..].find('>').map_or(h.len(), |g| p + g);
        if let Some(u) = attr(&h[p..tag_end], "unitName") {
            if u.starts_with("second") {
                rt /= 60.0;
            }
        }
    }
    rt
}

impl Rows {
    pub(super) fn with(head: Head) -> Rows {
        Rows {
            head,
            rows: Vec::new(),
            refs: HashMap::new(),
            level_of: HashMap::new(),
        }
    }

    /// Level and polarity of the row added last: what the `.mzidx` builder groups the scan by (the text of an extra can say more than the scan does).
    pub(crate) fn ultima(&self) -> Option<(u8, i8)> {
        self.rows.last().map(|r| (r.level, r.polarity))
    }

    /// From the header of a run, without writing it (`Head::da_testata`); `None` when the text would be needed.
    pub(crate) fn da_testata(h: &Header) -> Option<Rows> {
        Head::da_testata(h).map(Rows::with)
    }

    /// From the text of a document up to its first spectrum (the header `mzmlwrite` writes, with its `<run ` tag).
    pub(crate) fn from_head(doc: &[u8]) -> Rows {
        let at = find(doc, b"<run ");
        let head = String::from_utf8_lossy(&doc[..at.unwrap_or(200000.min(doc.len()))]);
        let lo = at.unwrap_or(0);
        let run_text = String::from_utf8_lossy(&doc[lo..(lo + 2000).min(doc.len())]);
        Rows::with(Head::parse(&head, &run_text))
    }
}

/// The resolution of a spectrum header (`hs` is its part before the precursors).
fn resolution(hs: &str, h: &str) -> Option<f64> {
    cv(hs, "MS:1000800").and_then(float).or_else(|| {
        let pat = "<userParam name=\"opentfraw.resolution\" value=\"";
        h.find(pat).and_then(|p| {
            let s = p + pat.len();
            h[s..].find('"').and_then(|e| float(&h[s..s + e]))
        })
    })
}

impl Rows {
    /// Links every MSn row to its parent (`_link_parents`) and closes the table.
    pub(crate) fn finish(self, n_chromatograms: usize) -> Table {
        let Rows {
            head,
            mut rows,
            refs,
            ..
        } = self;
        let by_native: HashMap<&str, usize> = rows
            .iter()
            .enumerate()
            .map(|(i, r)| (r.native.as_str(), i))
            .collect();
        let mut parents: Vec<Option<usize>> = vec![None; rows.len()];
        let mut last: HashMap<i8, usize> = HashMap::new();
        let mut last_path: HashMap<PathKey, usize> = HashMap::new();
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
        drop(by_native);
        for (r, p) in rows.iter_mut().zip(parents) {
            r.parent = p;
        }
        let hr1 = is_hr(&rows.iter().filter(|r| r.level == 1).collect::<Vec<_>>());
        Table {
            rows,
            nce: head.nce,
            hr1,
            n_chromatograms,
        }
    }
}

/// The first ` id="..."` of a spectrum header (`\sid="..."`).
fn first_id(h: &str) -> Option<String> {
    let mut from = 0;
    while let Some(p) = h[from..].find("id=\"") {
        let at = from + p;
        if at > 0 && h.as_bytes()[at - 1].is_ascii_whitespace() {
            let s = at + 4;
            return h[s..].find('"').map(|e| h[s..s + e].to_string());
        }
        from = at + 3;
    }
    None
}

impl Rows {
    /// One spectrum: `a`, `b` are its bytes in the mzML, `h` its header text.
    pub(crate) fn spectrum(&mut self, a: u64, b: u64, h: &str) {
        let id = first_id(h);
        let lvl = cv(h, "MS:1000511").filter(|v| !v.is_empty());
        let pol = if h.contains("accession=\"MS:1000130\"") {
            1
        } else if h.contains("accession=\"MS:1000129\"") {
            -1
        } else {
            0
        };
        let rt = rt_minutes(h);
        let level = lvl.and_then(float).map_or(1, |v| v as u8);
        let hs = h.split("<precursorList").next().unwrap_or("");
        let filt = cv(hs, "MS:1000512").unwrap_or("").to_string();
        let mut m = if level > 1 {
            self.msn(h, level, &filt)
        } else {
            Msn::default()
        };
        if m.ce.is_none() && h.contains("MS:1000045") {
            m.ce = cv(h, "MS:1000045").and_then(float);
        }
        let icref = hs.find("<scan").and_then(|p| {
            let t = &hs[p..];
            let tag = &t[..t.find('>').unwrap_or(t.len())];
            attr(tag, "instrumentConfigurationRef")
        });
        let res = resolution(hs, h);
        let row = Row {
            start: a,
            end: b,
            k: None,
            posizione: None,
            level,
            rt,
            polarity: pol,
            tic: cv(h, "MS:1000285")
                .and_then(float)
                .filter(|v| *v != 0.0)
                .unwrap_or(0.0),
            precursor: m.prec,
            ce: m.ce,
            an: self.head.analyzer(&filt, icref),
            kind: super::kind_of(h, &filt).filter(|_| level > 1),
            mob: super::mobility_of_header(h),
            filter: filt,
            profile: h.contains("accession=\"MS:1000128\""),
            parent: None,
            iso: m.iso,
            act: m.act,
            res,
            pint: m.pint,
            path: m.path,
            precs: m.precs,
            charge: m.charge,
            native: id.clone().unwrap_or_default(),
        };
        if let Some(i) = id {
            self.level_of.insert(i, level);
        }
        self.rows.push(row);
    }
}

/// The accessions `spectrum` looks for in the text of a header (as substrings: `h.contains` too): an extra that carries one of them would change the row.
const LETTI: [&str; 25] = [
    "MS:1000341",
    "MS:1000326",
    "MS:1000325",
    "MS:1000789",
    "MS:1000790",
    "MS:1002815",
    "MS:1002476",
    "MS:1000511",
    "MS:1000130",
    "MS:1000129",
    "MS:1000016",
    "MS:1000512",
    "MS:1000800",
    "MS:1000285",
    "MS:1000128",
    "MS:1000045",
    "MS:1000744",
    "MS:1000827",
    "MS:1000828",
    "MS:1000829",
    "MS:1000041",
    "MS:1000042",
    "MS:1000422",
    "MS:1000133",
    "MS:1000598",
];

/// True when nothing in the scan or its extras can make the header text say more than the row `lettore` builds: no `MS:1000...` accession in the id, the
/// filter or the free text of the extras, and no `cv:` extra with an accession `spectrum` reads.
fn extra_inerte(sp: &Spectrum, extra: &SpectrumExtra) -> bool {
    let bad = |t: &str| t.contains(['"', '<', '>']);
    let acc = |t: &str| t.contains("MS:1000");
    if acc(&sp.id) || acc(&sp.filter) {
        return false;
    }
    extra
        .params
        .iter()
        .all(|(k, v)| match k.strip_prefix("cv:") {
            Some(a) => !bad(a) && !bad(v) && !acc(v) && !LETTI.iter().any(|l| a.contains(l)),
            None => !acc(k) && !acc(v),
        })
}

/// `&`, `<`, `>` and `"` as the writer escapes them.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl Rows {
    /// One scan of a reader, as `spectrum` would read the header `mzmlwrite::spectrum_head(k, sp, extra)` writes for it, without writing it: the same
    /// fields from the same values (`sp` with its arrays cut to the shorter one, `tic` the sum of its intensities). `false` and nothing added when an
    /// extra could change what the text says (`extra_inerte`): the caller then gives `spectrum` the text.
    pub(crate) fn lettore(&mut self, sp: &Spectrum, extra: &SpectrumExtra, tic: f64) -> bool {
        if !extra_inerte(sp, extra) {
            return false;
        }
        let native = esc(&sp.id);
        let filt = esc(&sp.filter);
        let level = sp.level;
        let polarity = match sp.polarity {
            1 => 1,
            -1 => -1,
            _ => 0,
        };
        let mut m = Msn::default();
        if level > 1 {
            m.path = filter_path(&filt);
            if let Some(p) = sp.precursor {
                m.precs.push(Prec {
                    target: Some(p),
                    selected: Some(p),
                    ..Prec::default()
                });
                m.prec = Some(p);
                m.iso = Some((p - 0.5, p + 0.5));
            }
            if m.act.is_none() {
                m.act = match &m.path {
                    Some(p) => Some(p[p.len() - 1].1.clone()),
                    None => activation_in_filter(&filt),
                };
            }
        }
        let res = extra
            .params
            .iter()
            .find(|(k, _)| k == "opentfraw.resolution")
            .and_then(|(_, v)| float(&esc(v)));
        let row = Row {
            start: 0,
            end: 0,
            k: None,
            posizione: None,
            level,
            rt: sp.rt,
            polarity,
            tic: Some(tic).filter(|v| *v != 0.0).unwrap_or(0.0),
            precursor: m.prec,
            ce: m.ce,
            an: self.head.analyzer(&filt, None),
            kind: super::kind_of("", &filt).filter(|_| level > 1),
            filter: filt,
            profile: sp.profile,
            parent: None,
            iso: m.iso,
            act: m.act,
            res,
            pint: m.pint,
            path: m.path,
            precs: m.precs,
            charge: m.charge,
            native: native.clone(),
            mob: None,
        };
        self.level_of.insert(native, level);
        self.rows.push(row);
        true
    }
}

/// What the precursor elements of an MSn header say of the immediate precursor.
#[derive(Default)]
struct Msn {
    prec: Option<f64>,
    iso: Option<(f64, f64)>,
    act: Option<String>,
    pint: Option<f64>,
    path: Option<Vec<(f64, String, f64)>>,
    ce: Option<f64>,
    charge: Option<i64>,
    precs: Vec<Prec>,
}

impl Rows {
    /// The precursor part of the header of an MSn spectrum.
    fn msn(&mut self, h: &str, level: u8, filt: &str) -> Msn {
        let mut m = Msn {
            path: filter_path(filt),
            ..Msn::default()
        };
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
            m.precs.push(precursor_element(body, rf));
            from = after + g + 1 + close + 12;
        }
        if m.precs.is_empty() && (h.contains("MS:1000744") || h.contains("MS:1000827")) {
            m.precs.push(precursor_element(h, None));
        }
        if let Some(e) = immediate(&m.precs, &m.path, level, &self.level_of) {
            m.prec = e.selected.or(e.target);
            if let Some(t) = e.target.or(m.prec) {
                let lo = match (e.lo, e.target) {
                    (Some(lo), Some(tg)) => tg - lo,
                    _ => 0.5,
                };
                let hi = match (e.hi, e.target) {
                    (Some(hi), Some(tg)) => hi - tg,
                    _ => 0.5,
                };
                m.iso = Some((t - lo, t + hi));
            }
            m.act = e.act.map(String::from);
            m.pint = e.pint;
            m.ce = e.ce;
            m.charge = e.charge;
        }
        if m.act.is_none() {
            m.act = match &m.path {
                Some(p) => Some(p[p.len() - 1].1.clone()),
                None => activation_in_filter(filt),
            };
        }
        if let Some(r) = m
            .precs
            .iter()
            .find_map(|e| e.refer.clone().filter(|r| !r.is_empty()))
        {
            self.refs.insert(self.rows.len(), r);
        }
        m
    }
}
