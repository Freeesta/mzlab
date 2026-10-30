//! The rows of the scan table, one spectrum header at a time: from the headers of an mzML (`read`) or the headers `mzmlwrite` would write
//! for the scans of a `Lettore` (`mzidx_lettore`).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::inerte::extra_inerte;
use super::*;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, SpectrumExtra};

/// A level and the key of a filter path (`path_key`).
type PathKey = (u8, Vec<(i64, String, u64)>);

/// What the file header says that the rows need: the analyzers of the instrument configurations and the default one, and `run.nce`.
#[derive(Debug, PartialEq)]
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

    fn analyzer(&self, filt: &str, icref: Option<&str>) -> &'static str {
        let tok = filt.split(' ').next().unwrap_or("").to_uppercase();
        if let Some(a) = ANALYZERS.iter().find(|a| **a == tok) {
            return a;
        }
        let look = |k: &str| self.ic.iter().find(|(n, _)| n == k).map(|(_, a)| *a);
        look(icref.unwrap_or(""))
            .or_else(|| look(self.default_ic.as_deref().unwrap_or("")))
            .unwrap_or("?")
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
    refs: HashMap<usize, Arc<str>>,
    level_of: HashMap<Arc<str>, u8>,
    /// The texts that many rows repeat (filter strings, activation names): one allocation each.
    pool: HashSet<Arc<str>>,
    window: super::DeclaredWindow,
}

impl Rows {
    pub(super) fn with(head: Head) -> Rows {
        Rows {
            window: Default::default(),
            head,
            rows: Vec::new(),
            refs: HashMap::new(),
            level_of: HashMap::new(),
            pool: HashSet::new(),
        }
    }

    /// The rows kept so far.
    pub(crate) fn len(&self) -> usize {
        self.rows.len()
    }

    /// Room for `n` more rows in one allocation (a vector that doubles holds the old and the new buffer together: the peak of a million-scan load).
    pub(crate) fn reserve(&mut self, n: usize) {
        self.rows.reserve_exact(n);
    }

    /// The shared allocation of a text that rows repeat.
    fn intern(&mut self, t: &str) -> Arc<str> {
        let known = self.pool.get(t).cloned();
        known.unwrap_or_else(|| {
            let a: Arc<str> = Arc::from(t);
            self.pool.insert(a.clone());
            a
        })
    }

    /// The text of a reference to another scan: the allocation of that scan's native id when it is already known (a precursor comes after its parent).
    fn shared(&self, t: &str) -> Arc<str> {
        self.level_of
            .get_key_value(t)
            .map_or_else(|| Arc::from(t), |(k, _)| k.clone())
    }

    /// The native id of a row, and the key of `level_of` for it: one allocation for both.
    fn native_id(&mut self, id: &str, level: u8) -> Arc<str> {
        let a: Arc<str> = Arc::from(id);
        self.level_of.insert(a.clone(), level);
        a
    }

    /// Level and polarity of the row added last: what the `.mzidx` builder groups the scan by (the text of an extra can say more than the scan does).
    pub(crate) fn ultima(&self) -> Option<(u8, i8)> {
        self.rows.last().map(|r| (r.level, r.polarity))
    }

    /// Whether the last row alone makes the file high-resolution (`Table::hr1` takes the rows of every MS1 scan).
    pub(crate) fn ultima_hr(&self) -> bool {
        self.rows.last().is_some_and(|r| super::is_hr(&[r]))
    }
}

/// The retention time of a spectrum header, in minutes.
pub(super) fn rt_minutes(h: &str) -> f64 {
    let mut rt = cv(h, "MS:1000016").and_then(float).unwrap_or(0.0);
    if let Some(p) = h.find("accession=\"MS:1000016\"") {
        let tag_end = h[p..].find('>').map_or(h.len(), |g| p + g);
        let tag = &h[p..tag_end];
        if crate::mzml::is_seconds(attr(tag, "unitName"), attr(tag, "unitAccession")) {
            rt /= 60.0;
        }
    }
    rt
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
            level_of,
            pool,
            window,
        } = self;
        // only reading needed them; the tables below are what the peak of this step is made of
        drop((level_of, pool));
        let by_native: HashMap<&str, usize> = rows
            .iter()
            .enumerate()
            .map(|(i, r)| (&*r.native, i))
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
                let mut p = refs.get(&i).and_then(|r| by_native.get(&**r).copied());
                if p.is_none() {
                    p = last.get(&s.polarity).or_else(|| last.get(&0)).copied();
                }
                parents[i] = p;
            } else if let Some(pa) = &s.d().path {
                parents[i] = last_path
                    .get(&(s.level - 1, path_key(&pa[..pa.len() - 1])))
                    .copied();
            } else {
                let j = refs.get(&i).and_then(|r| by_native.get(&**r).copied());
                parents[i] = j.filter(|&j| rows[j].level == s.level - 1);
            }
            if let Some(pa) = &s.d().path {
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
            window: window.span,
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
        self.window.note(h, level);
        let hs = h.split("<precursorList").next().unwrap_or("");
        let filt = cv(hs, "MS:1000512").unwrap_or("");
        let mut m = if level > 1 {
            self.msn(h, level, filt)
        } else {
            Msn::default()
        };
        if m.ce.is_none() && h.contains("accession=\"MS:1000045\"") {
            m.ce = cv(h, "MS:1000045").and_then(float);
        }
        let icref = hs.find("<scan").and_then(|p| {
            let t = &hs[p..];
            let tag = &t[..t.find('>').unwrap_or(t.len())];
            attr(tag, "instrumentConfigurationRef")
        });
        let res = resolution(hs, h);
        let native = match &id {
            Some(i) => self.native_id(i, level),
            None => self.intern(""),
        };
        let act = m.act.map(|a| self.intern(&a));
        let mut detail = Detail {
            precursor: m.prec,
            ce: m.ce,
            iso: m.iso,
            act,
            pint: m.pint,
            path: m.path,
            precs: m.precs,
            charge: m.charge,
        };
        detail.shrink();
        let row = Row {
            start: a,
            end: b,
            dove: None,
            level,
            rt,
            polarity: pol,
            tic: cv(h, "MS:1000285")
                .and_then(float)
                .filter(|v| *v != 0.0)
                .unwrap_or(0.0),
            an: self.head.analyzer(filt, icref),
            kind: (level > 1).then(|| super::kind_of(h, filt)).flatten(),
            mob: super::mobility_of_header(h),
            filter: self.intern(filt),
            profile: h.contains("accession=\"MS:1000128\""),
            parent: None,
            res,
            native,
            detail: (!detail.is_empty()).then(|| Box::new(detail)),
        };
        self.rows.push(row);
    }
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
        let filt = filt.as_str();
        let level = sp.level;
        let polarity = match sp.polarity {
            1 => 1,
            -1 => -1,
            _ => 0,
        };
        self.window.note_span(extra.window, level);
        let m = plain_msn(sp, extra, filt);
        let res = extra
            .params
            .iter()
            .find(|(k, _)| k == "opentfraw.resolution")
            .and_then(|(_, v)| float(&esc(v)));
        let act = m.act.map(|a| self.intern(&a));
        let mut detail = Detail {
            precursor: m.prec,
            ce: m.ce,
            iso: m.iso,
            act,
            pint: m.pint,
            path: m.path,
            precs: m.precs,
            charge: m.charge,
        };
        detail.shrink();
        let row = Row {
            start: 0,
            end: 0,
            dove: None,
            level,
            rt: sp.rt,
            polarity,
            tic: Some(tic).filter(|v| *v != 0.0).unwrap_or(0.0),
            an: self.head.analyzer(filt, None),
            kind: (level > 1).then(|| super::kind_of("", filt)).flatten(),
            filter: self.intern(filt),
            profile: sp.profile,
            parent: None,
            res,
            native: self.native_id(&native, level),
            mob: None,
            detail: (!detail.is_empty()).then(|| Box::new(detail)),
        };
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
            let rf = attr(attrs, "spectrumRef").map(|r| self.shared(r));
            m.precs.push(precursor_element(body, rf));
            from = after + g + 1 + close + 12;
        }
        if m.precs.is_empty() && (h.contains("MS:1000744") || h.contains("MS:1000827")) {
            m.precs.push(precursor_element(h, None));
        }
        // the parent of an MS3+ is its immediate precursor, not the first one listed (which may be the MS1)
        let mut immediate_ref = None;
        if let Some(e) = immediate(&m.precs, &m.path, level, &self.level_of) {
            immediate_ref = e.refer.clone().filter(|r| !r.is_empty());
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
        if let Some(r) = parent_ref(&m.precs, immediate_ref, level) {
            self.refs.insert(self.rows.len(), r);
        }
        m
    }
}

/// The scan an MSn row points to as its parent: for MS3+ the ref of the immediate precursor, else (and for MS2) the first ref listed.
fn parent_ref(precs: &[Prec], immediate_ref: Option<Arc<str>>, level: u8) -> Option<Arc<str>> {
    immediate_ref.filter(|_| level > 2).or_else(|| {
        precs
            .iter()
            .find_map(|e| e.refer.clone().filter(|r| !r.is_empty()))
    })
}

/// The precursor part of a row of a scan whose reader gave a plain precursor: what the writer says of it (its window, with offsets when the reader knows
/// them, CID and the energy), else the activation of the filter.
fn plain_msn(sp: &Spectrum, extra: &SpectrumExtra, filt: &str) -> Msn {
    let mut m = Msn::default();
    if sp.level <= 1 {
        return m;
    }
    m.path = filter_path(filt);
    if let Some(p) = sp.precursor {
        let half = extra.iso_half;
        m.precs.push(Prec {
            target: Some(p),
            selected: Some(p),
            lo: half.map(|h| p - h),
            hi: half.map(|h| p + h),
            act: Some("CID"),
            ce: extra.ce,
            ..Prec::default()
        });
        m.prec = Some(p);
        m.iso = Some((p - half.unwrap_or(0.5), p + half.unwrap_or(0.5)));
        m.act = Some("CID".into());
        m.ce = extra.ce;
    }
    if m.act.is_none() {
        m.act = match &m.path {
            Some(p) => Some(p[p.len() - 1].1.clone()),
            None => activation_in_filter(filt),
        };
    }
    m
}

impl Rows {
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
