//! The rows of the scan table, one spectrum header at a time: from the headers of an mzML (`read`) or the headers `mzmlwrite` would write
//! for the scans of a `Lettore` (`mzidx_lettore`).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::analyzer::analyzer_names;
use super::inert::{extra_inerte, tic_read};
use super::*;
use crate::mzml::Spectrum;
use crate::mzmlwrite::{Header, SpectrumExtra};
use crate::spechead::{self, Groups, Stage};

/// A level and the key of a filter path (`path_key`).
type PathKey = (u8, Vec<(i64, String, u64)>);

/// What the file header says that the rows need: the analyzers of the instrument configurations and the default one, and `run.nce`.
#[derive(Debug, PartialEq)]
pub(super) struct Head {
    nce: bool,
    ic: Vec<(String, &'static str)>,
    default_ic: Option<String>,
    /// `count` of the `<spectrumList` that follows the `<run ` tag: what the file says it holds (0 when absent).
    declared: usize,
    /// The `referenceableParamGroup`s of the file header.
    groups: Groups,
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
        let declared = super::head_read::declared_count(run_text);
        let groups = if head.contains("referenceableParamGroup") {
            crate::spechead::groups(head)
        } else {
            Groups::new()
        };
        Head {
            nce,
            ic,
            default_ic,
            declared,
            groups,
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
            declared: 0,
            groups: Groups::new(),
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
    pub(crate) fn last(&self) -> Option<(u8, i8)> {
        self.rows.last().map(|r| (r.level, r.polarity))
    }

    /// Whether the last row alone makes the file high-resolution (`Table::hr1` takes the rows of every MS1 scan).
    pub(crate) fn last_hr(&self) -> bool {
        self.rows.last().is_some_and(|r| super::is_hr(&[r]))
    }
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
            declared: head.declared,
        }
    }
}

impl Rows {
    /// One spectrum: `a`, `b` are its bytes in the mzML, `h` its header text (read by `spechead`, the reader every mzML path shares).
    pub(crate) fn spectrum(&mut self, a: u64, b: u64, h: &str) {
        let sh = spechead::parse(h, &self.head.groups).unwrap_or_default();
        let t = &sh.terms;
        let level = sh.level();
        self.window
            .note_span(t.float("MS:1000501").zip(t.float("MS:1000500")), level);
        let filt = sh.filter();
        let mut m = if level > 1 {
            self.msn(&sh, level, filt)
        } else {
            Msn::default()
        };
        if m.ce.is_none() {
            m.ce = t
                .float("MS:1000045")
                .or_else(|| sh.precursors.iter().find_map(Stage::energy));
        }
        let res = t
            .float("MS:1000800")
            .or_else(|| t.user("opentfraw.resolution").and_then(float));
        // the table keeps a native id as the writer escapes it (the direct path's `esc` too)
        let native = if sh.id.is_empty() {
            self.intern("")
        } else {
            self.native_id(&esc(&sh.id), level)
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
            at: None,
            level,
            rt: sh.rt_minutes(),
            polarity: sh.polarity(),
            tic: t.float("MS:1000285").filter(|v| *v != 0.0).unwrap_or(0.0),
            an: self.head.analyzer(filt, sh.ic_ref.as_deref()),
            kind: (level > 1)
                .then(|| super::kind_of(|acc| t.has(acc), filt))
                .flatten()
                .or_else(|| {
                    t.user(crate::wiff::modes::SCAN_TYPE_PARAM)
                        .and_then(crate::wiff::modes::scan_type_named)
                }),
            mob: super::mobility_of_terms(t),
            filter: self.intern(filt),
            profile: sh.profile(),
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
    /// fields from the same values (`sp` with its arrays cut to the shorter one, `tic` the total the writer writes: `extra.tic`, else the sum of the
    /// intensities). `false` and nothing added when an extra could change what the text says (`extra_inerte`): the caller then gives `spectrum` the text.
    pub(crate) fn reader(&mut self, sp: &Spectrum, extra: &SpectrumExtra, tic: f64) -> bool {
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
            at: None,
            level,
            rt: sp.rt,
            polarity,
            tic: Some(tic_read(tic)).filter(|v| *v != 0.0).unwrap_or(0.0),
            an: self.head.analyzer(filt, None),
            kind: (level > 1)
                .then(|| super::kind_of(|_| false, filt))
                .flatten()
                .or_else(|| crate::wiff::modes::scan_type_param(&extra.params)),
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
    fn msn(&mut self, sh: &spechead::Head, level: u8, filt: &str) -> Msn {
        let mut m = Msn {
            path: filter_path(filt),
            ..Msn::default()
        };
        for st in &sh.precursors {
            let rf = st.refer.as_deref().map(|r| self.shared(&esc(r)));
            m.precs.push(precursor_of(st, rf));
        }
        if m.precs.is_empty() && (sh.terms.has("MS:1000744") || sh.terms.has("MS:1000827")) {
            // a selected ion or a target written outside a <precursor>
            m.precs.push(precursor_of(&Stage::loose(&sh.terms), None));
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
