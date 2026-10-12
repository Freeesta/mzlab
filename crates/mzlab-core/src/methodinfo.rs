//! Acquisition parameters stored in Analyst `.dam` (method) and `.wiff` (data) files: `read_methods` of `mzlab/reader/methodinfo.py`.
//! Values are 32-bit floats after the UTF-16 parameter name; the LC method is XML. Anything not recognised is left out.

use crate::error::Result;
use crate::json::{round, Out};
use crate::ole::Ole;
use crate::xmltree::{self, El};

const NAMES: [(&str, &str, &str); 13] = [
    ("CUR", "Curtain gas (CUR)", "psi"),
    ("GS1", "Ion source gas 1 (GS1)", "psi"),
    ("GS2", "Ion source gas 2 (GS2)", "psi"),
    ("CAD", "Collision gas (CAD)", ""),
    ("IS", "IonSpray voltage (IS)", "V"),
    ("TEM", "Source temperature (TEM)", "°C"),
    ("ihe", "Interface heater (ihe)", ""),
    ("DP", "Declustering potential (DP)", "V"),
    ("EP", "Entrance potential (EP)", "V"),
    ("CEP", "Collision cell entrance potential (CEP)", "V"),
    ("CE", "Collision energy (CE)", "eV"),
    ("CXP", "Collision cell exit potential (CXP)", "V"),
    ("CES", "Collision energy spread (CES)", "eV"),
];

#[derive(Debug, Clone)]
pub struct Param {
    pub id: String,
    pub label: &'static str,
    pub value: f64,
    pub unit: &'static str,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub q1: f64,
    pub q3: f64,
    pub dwell: f64,
    pub name: String,
    pub ce: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum Kind {
    Mrm(Vec<Transition>),
    Scan { range: Option<[f64; 2]>, ce: Vec<f64> },
}

#[derive(Debug, Clone)]
pub struct Experiment {
    pub index: usize,
    pub kind: Kind,
}

#[derive(Debug, Clone, Default)]
pub struct Pda {
    pub start: Option<f64>,
    pub stop: Option<f64>,
    pub step: Option<f64>,
    pub freq: Option<f64>,
    pub lamp: String,
    pub cell: Option<f64>,
    pub slit: Option<f64>,
}

/// A point of the pump program (`None` = not set).
#[derive(Debug, Clone)]
pub struct GPoint {
    pub t: f64,
    pub b: Option<f64>,
    pub flow: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Lc {
    pub gradient: Vec<GPoint>,
    pub flow: Option<f64>,
    pub b0: Option<f64>,
    pub run_time: Option<f64>,
    pub pda: Pda,
    pub channels: Vec<(f64, Option<f64>)>,
    pub oven: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct Method {
    pub name: String,
    pub sample: Option<i64>,
    pub source: Vec<Param>,
    pub compound: Vec<Param>,
    pub file: Vec<(&'static str, String)>,
    pub experiments: Vec<Experiment>,
    pub lc: Option<Lc>,
}

fn num_param(v: f64) -> f64 {
    if v.abs() < 1e5 {
        round(v, 3)
    } else {
        v
    }
}

fn f32_at(d: &[u8], at: usize) -> Option<f64> {
    d.get(at..at + 4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64)
}

fn is_alnum(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

/// A run of `lo..=hi` pairs `[A-Za-z0-9]\x00` starting at `at`: the length in pairs.
fn pairs(d: &[u8], at: usize, hi: usize) -> usize {
    let mut n = 0;
    while n < hi && at + 2 * n + 1 < d.len() && is_alnum(d[at + 2 * n]) && d[at + 2 * n + 1] == 0 {
        n += 1;
    }
    n
}

fn ascii(b: &[u8]) -> String {
    b.chunks(2).map(|c| c[0] as char).collect()
}

fn find(d: &[u8], from: usize, to: usize, pat: &[u8]) -> Option<usize> {
    let to = to.min(d.len());
    (from..to.saturating_sub(pat.len() - 1)).find(|&i| d[i..].starts_with(pat))
}

/// `\x04\x00C\x00E\x00(.{4})` searched in `from..to`: the float after the first `CE`.
fn ce_after(d: &[u8], from: usize, to: usize) -> Option<f64> {
    let tag = [4u8, 0, b'C', 0, b'E', 0];
    let mut i = from;
    while i + 10 <= to.min(d.len()) {
        if d[i..].starts_with(&tag) {
            return f32_at(d, i + 6);
        }
        i += 1;
    }
    None
}

struct MrmHit {
    start: usize,
    end: usize,
    q1: f64,
    q3: f64,
    dwell: f64,
    len: u8,
    name: Vec<u8>,
}

/// One MRM transition: Q1, 0, Q3, dwell (ms), 0, length byte, 0, UTF-16 name, then the compound parameters (DP first).
fn mrm_at(d: &[u8], i: usize) -> Option<MrmHit> {
    if i + 22 > d.len() || d[i + 4..i + 8] != [0; 4] || d[i + 16..i + 20] != [0; 4] || d[i + 21] != 0 {
        return None;
    }
    let mut p = i + 22;
    while p + 1 < d.len() && (0x20..=0x7e).contains(&d[p]) && d[p + 1] == 0 {
        p += 2;
    }
    if !d[p..].starts_with(&[4, 0, b'D', 0, b'P', 0]) {
        return None;
    }
    Some(MrmHit { start: i, end: p + 6, q1: f32_at(d, i)?, q3: f32_at(d, i + 8)?, dwell: f32_at(d, i + 12)?, len: d[i + 20], name: d[i + 22..p].to_vec() })
}

fn experiments(ole: &Ole, base: &str) -> Vec<Experiment> {
    let mut out = Vec::new();
    for i in 0..16 {
        let key = format!("{base}DeviceMethod0/Period0/Experiment{i}/MassRangeEx/MassRangeEx");
        let Some(d) = ole.read(&key) else { break };
        let mut hits = Vec::new();
        let mut at = 0;
        while at < d.len() {
            match mrm_at(&d, at) {
                Some(h) => {
                    at = h.end;
                    hits.push(h);
                }
                None => at += 1,
            }
        }
        hits.retain(|h| !h.name.is_empty() && h.name.len() == h.len as usize);
        if !hits.is_empty() {
            let tr = hits
                .iter()
                .enumerate()
                .map(|(j, h)| {
                    let end = hits.get(j + 1).map_or(d.len(), |n| n.start);
                    Transition {
                        q1: round(h.q1, 2),
                        q3: round(h.q3, 2),
                        dwell: round(h.dwell, 1),
                        name: ascii(&h.name),
                        ce: ce_after(&d, h.end, end).map(|v| round(v, 2)),
                    }
                })
                .collect();
            out.push(Experiment { index: i, kind: Kind::Mrm(tr) });
            continue;
        }
        let mut ce: Vec<f64> = Vec::new();
        let mut at = 0;
        while at + 10 <= d.len() {
            if d[at..].starts_with(&[4, 0, b'C', 0, b'E', 0]) {
                let v = round(f32_at(&d, at + 6).unwrap_or(0.0), 2);
                if !ce.contains(&v) {
                    ce.push(v);
                }
                at += 10;
            } else {
                at += 1;
            }
        }
        ce.sort_by(|a, b| a.total_cmp(b));
        let mut range = None;
        if d.len() >= 244 {
            let a = f32_at(&d, 40).unwrap_or(f64::NAN);
            let b = if d.len() == 432 { f32_at(&d, 240) } else { None };
            if let Some(b) = b {
                if 10.0 <= a && a < b && b <= 3000.0 {
                    range = Some([round(a, 2), round(b, 2)]);
                }
            }
        }
        out.push(Experiment { index: i, kind: Kind::Scan { range, ce } });
    }
    out
}

fn num(x: &str) -> Option<f64> {
    x.trim().parse().ok()
}

fn cats(root: &El) -> Vec<(String, Vec<(String, (String, String))>, Vec<(String, Vec<Vec<String>>)>)> {
    let mut all = Vec::new();
    root.iter("Category", &mut all);
    all.into_iter()
        .map(|c| {
            let title = c.findtext("Title").unwrap_or("").trim().to_string();
            let par = c
                .findall("Parameter")
                .map(|p| {
                    let g = |n: &str| p.findtext(n).unwrap_or("").trim().to_string();
                    (g("Name"), (g("Value"), g("Unit")))
                })
                .collect();
            let tabs = c
                .findall("Table")
                .map(|t| {
                    let rows: Vec<Vec<String>> = t
                        .findall("row")
                        .map(|r| (1..=8).filter_map(|i| r.find(&format!("Field{i}")).map(|f| f.text.trim().to_string())).collect())
                        .collect();
                    (t.findtext("Name").unwrap_or("").trim().to_string(), rows.into_iter().skip(2).filter(|r| !r.is_empty()).collect())
                })
                .collect();
            (title, par, tabs)
        })
        .collect()
}

/// LC method (pump gradient, flow, PDA, oven) from the XML in `DeviceMethod1/VendorAppMethod`.
fn lc(ole: &Ole, base: &str) -> Option<Lc> {
    let raw = ole.read(&format!("{base}DeviceMethod1/VendorAppMethod"))?;
    let a = find(&raw, 0, raw.len(), b"<SCIEX_ANALYST_ACQUISITION_METHOD")?;
    let end = b"</SCIEX_ANALYST_ACQUISITION_METHOD>";
    let b = find(&raw, 0, raw.len(), end)?;
    let root = xmltree::parse(&String::from_utf8_lossy(raw.get(a..b + end.len())?))?;
    let cats = cats(&root);
    // a later category with the same title replaces the earlier one (a dict in Python)
    let cat = |title: &str| cats.iter().rev().find(|c| c.0 == title);
    let par = |title: &str, name: &str| cat(title).and_then(|c| c.1.iter().rev().find(|p| p.0 == name)).map(|p| p.1 .0.as_str()).unwrap_or("");
    let (flow0, b0) = (num(par("Pumps", "Total Flow")), num(par("Pumps", "B Concentration")));
    let tp = cat("System Controller").and_then(|c| c.2.iter().rev().find(|t| t.0 == "Time Program")).map(|t| t.1.clone()).unwrap_or_default();
    let mut events: Vec<(f64, String, Option<f64>)> = tp.iter().filter(|r| r.len() >= 5).filter_map(|r| num(&r[0]).map(|t| (t, r[3].clone(), num(&r[4])))).collect();
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (mut grad, mut flow, mut bconc, mut stop) = (Vec::new(), flow0, b0, None);
    if b0.is_some() {
        grad.push(GPoint { t: 0.0, b: b0, flow });
    }
    for (t, name, val) in events {
        match (name.as_str(), val) {
            ("Pump B Conc.", Some(v)) => bconc = Some(v),
            ("Total Flow", Some(v)) => flow = Some(v),
            ("Stop", _) => {
                stop = Some(t);
                continue;
            }
            _ => continue,
        }
        match grad.last_mut() {
            Some(l) if l.t == t => {
                l.b = bconc;
                l.flow = flow;
            }
            _ => grad.push(GPoint { t, b: bconc, flow }),
        }
    }
    let pv = |k: &str| par("PDA Detector", k);
    let channels = cat("PDA Detector")
        .and_then(|c| c.2.iter().rev().find(|t| t.0 == "DA Channel Array"))
        .map(|t| t.1.iter().filter(|r| r.len() >= 2).filter_map(|r| num(&r[0]).map(|w| (w, num(&r[1])))).collect())
        .unwrap_or_default();
    Some(Lc {
        gradient: grad,
        flow: flow0,
        b0,
        run_time: stop.or_else(|| num(pv("Run Time"))),
        pda: Pda {
            start: num(pv("Start WL")),
            stop: num(pv("Stop WL")),
            step: num(pv("Wave Step")),
            freq: num(pv("Sampling Frequency")),
            lamp: pv("Lamp").to_string(),
            cell: num(pv("Cell Temperature")),
            slit: num(pv("Slit Width")),
        },
        channels,
        oven: num(par("Oven", "Temperature")),
    })
}

fn named(id: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    NAMES.iter().find(|n| n.0 == id)
}

fn source(ole: &Ole, base: &str) -> Vec<Param> {
    let mut out = Vec::new();
    for i in 0..24 {
        let Some(d) = ole.read(&format!("{base}DeviceMethod0/Period0/Experiment0/IonSourceParamsTable/Parameter{i}/ParameterData")) else { break };
        let body = d.get(16..).unwrap_or(&[]);
        let Some(s) = (0..body.len()).find(|&s| pairs(body, s, 6) >= 2) else { continue };
        let n = pairs(body, s, 6);
        let id = ascii(&body[s..s + 2 * n]);
        let end = 16 + s + 2 * n;
        if let (Some(v), Some(nm)) = (f32_at(&d, end), named(&id)) {
            out.push(Param { id, label: nm.1, value: num_param(v), unit: nm.2 });
        }
    }
    out
}

fn compound(ole: &Ole, base: &str) -> Vec<Param> {
    let mut out: Vec<Param> = Vec::new();
    let Some(d) = ole.read(&format!("{base}DeviceMethod0/Period0/Experiment0/MassRangeEx/MassRangeEx")) else { return out };
    let mut at = 0;
    while at + 2 < d.len() {
        let n = if (2..=12).contains(&d[at]) && d[at + 1] == 0 { pairs(&d, at + 2, 4) } else { 0 };
        if n < 2 {
            at += 1;
            continue;
        }
        let end = at + 2 + 2 * n;
        let id = ascii(&d[at + 2..end]);
        if let (Some(nm), true) = (named(&id), !out.iter().any(|p| p.id == id)) {
            if let Some(v) = f32_at(&d, end) {
                out.push(Param { id, label: nm.1, value: num_param(v), unit: nm.2 });
            }
        }
        at = end;
    }
    out
}

/// Runs of at least four printable ASCII characters in UTF-16 (`_strings`).
fn strings(d: &[u8]) -> Vec<String> {
    let (mut out, mut i) = (Vec::new(), 0);
    while i + 1 < d.len() {
        let mut j = i;
        while j + 1 < d.len() && (0x20..=0x7e).contains(&d[j]) && d[j + 1] == 0 {
            j += 2;
        }
        if (j - i) / 2 >= 4 {
            out.push(ascii(&d[i..j]));
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

const DAYS: [&str; 7] = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];

/// `[A-Z]:\\.*?\.dam`
fn dam_path(t: &str) -> Option<&str> {
    let b = t.as_bytes();
    (0..b.len().saturating_sub(2)).find(|&i| b[i].is_ascii_uppercase() && b[i + 1] == b':' && b[i + 2] == b'\\').and_then(|i| t[i + 3..].find(".dam").map(|j| &t[i..i + 3 + j + 4]))
}

/// `[A-Z][a-z]+day .*?[AP]M`
fn stamp(t: &str) -> Option<&str> {
    let b = t.as_bytes();
    for i in 0..b.len() {
        if !b[i].is_ascii_uppercase() {
            continue;
        }
        let run = b[i + 1..].iter().take_while(|c| c.is_ascii_lowercase()).count();
        let Some(k) = (1..=run).rev().find(|&k| b[i + 1 + k..].starts_with(b"day ")) else { continue };
        let from = i + 1 + k + 4;
        if let Some(j) = (from..b.len().saturating_sub(1)).find(|&j| (b[j] == b'A' || b[j] == b'P') && b[j + 1] == b'M') {
            return Some(&t[i..j + 2]);
        }
    }
    None
}

fn file_info(ole: &Ole, base: &str) -> Vec<(&'static str, String)> {
    let mut f: Vec<(&'static str, String)> = Vec::new();
    for fname in [format!("{base}AcqMethodFileInfoStm"), "FileRec_Str".to_string()] {
        let Some(d) = ole.read(&fname) else { continue };
        for t in strings(&d) {
            let tl = t.to_lowercase();
            let has = |k: &str| f.iter().any(|x| x.0 == k);
            if tl.contains(".dam") && !has("metodo") {
                f.push(("metodo", dam_path(&t).unwrap_or(&t).to_string()));
            } else if DAYS.iter().any(|d| tl.contains(d)) {
                if !has("data") {
                    f.push(("data", stamp(&t).unwrap_or(&t).replace(": ", ":")));
                }
            } else if tl.starts_with("analyst") {
                if !has("software") {
                    f.push(("software", t.trim().to_string()));
                }
            } else if (t.starts_with("LAB_") || t.ends_with("-PC")) && !has("computer") {
                f.push(("computer", t.clone()));
            }
        }
        if !f.is_empty() {
            break;
        }
    }
    f
}

fn method(ole: &Ole, base: &str) -> Method {
    Method { source: source(ole, base), compound: compound(ole, base), file: file_info(ole, base), experiments: experiments(ole, base), lc: lc(ole, base), ..Default::default() }
}

fn digits(s: &str) -> i64 {
    s.chars().filter(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0)
}

/// `[{name, source, compound, file, ...}]` for a `.dam` (one method) or a `.wiff` (one per sample).
pub fn read_methods(data: Vec<u8>, file_name: &str) -> Result<Vec<Method>> {
    let ole = Ole::open(data, file_name)?;
    let mut subs: Vec<String> = ole.paths_list().filter(|k| k.starts_with("MethodSubtree/Method")).filter_map(|k| k.split('/').nth(1)).map(String::from).collect();
    subs.sort_by_key(|s| digits(s));
    subs.dedup();
    if !subs.is_empty() {
        return Ok(subs
            .iter()
            .map(|s| Method { name: s.replace("Method", "Method "), sample: Some(digits(s)), ..method(&ole, &format!("MethodSubtree/{s}/")) })
            .collect());
    }
    Ok(vec![Method { name: file_name.to_string(), sample: None, ..method(&ole, "") }])
}

fn opt(o: &mut Out, v: Option<f64>) {
    o.opt(v);
}

fn params(o: &mut Out, v: &[Param]) {
    o.raw("[");
    for (i, p) in v.iter().enumerate() {
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" }).str(&p.id).raw(",\"label\":").str(p.label).raw(",\"value\":").num(p.value).raw(",\"unit\":").str(p.unit).raw("}");
    }
    o.raw("]");
}

impl Method {
    /// The method as Python's `read_methods` gives it, as JSON (the `lab` rows of `/api/method` add `check`).
    pub fn write_json(&self, o: &mut Out) {
        o.raw("{\"source\":");
        params(o, &self.source);
        o.raw(",\"compound\":");
        params(o, &self.compound);
        o.raw(",\"file\":{");
        for (i, (k, v)) in self.file.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
        }
        o.raw("},\"experiments\":[");
        for (i, e) in self.experiments.iter().enumerate() {
            o.raw(if i > 0 { ",{\"index\":" } else { "{\"index\":" }).int(e.index as i64);
            match &e.kind {
                Kind::Mrm(tr) => {
                    o.raw(",\"kind\":\"mrm\",\"transitions\":[");
                    for (j, t) in tr.iter().enumerate() {
                        o.raw(if j > 0 { ",{\"q1\":" } else { "{\"q1\":" }).num(t.q1).raw(",\"q3\":").num(t.q3).raw(",\"dwell\":").num(t.dwell).raw(",\"name\":").str(&t.name).raw(",\"ce\":");
                        opt(o, t.ce);
                        o.raw("}");
                    }
                    o.raw("]");
                }
                Kind::Scan { range, ce } => {
                    o.raw(",\"kind\":\"scan\",\"range\":");
                    match range {
                        Some(r) => o.raw("[").num(r[0]).raw(",").num(r[1]).raw("]"),
                        None => o.raw("null"),
                    };
                    o.raw(",\"ce\":").floats(ce);
                }
            }
            o.raw("}");
        }
        o.raw("]");
        if let Some(l) = &self.lc {
            o.raw(",\"lc\":");
            l.write_json(o);
        }
        o.raw(",\"name\":").str(&self.name).raw(",\"sample\":");
        match self.sample {
            Some(n) => o.int(n),
            None => o.raw("null"),
        };
        o.raw("}");
    }
}

impl Lc {
    fn write_json(&self, o: &mut Out) {
        o.raw("{\"gradient\":[");
        for (i, g) in self.gradient.iter().enumerate() {
            o.raw(if i > 0 { ",{\"t\":" } else { "{\"t\":" }).num(g.t).raw(",\"b\":").opt(g.b).raw(",\"flow\":").opt(g.flow).raw("}");
        }
        o.raw("],\"flow\":").opt(self.flow).raw(",\"b0\":").opt(self.b0).raw(",\"run_time\":").opt(self.run_time).raw(",\"pda\":{\"start\":").opt(self.pda.start);
        o.raw(",\"stop\":").opt(self.pda.stop).raw(",\"step\":").opt(self.pda.step).raw(",\"freq\":").opt(self.pda.freq).raw(",\"lamp\":").str(&self.pda.lamp);
        o.raw(",\"cell\":").opt(self.pda.cell).raw(",\"slit\":").opt(self.pda.slit).raw("},\"pda_channels\":[");
        for (i, (w, bw)) in self.channels.iter().enumerate() {
            o.raw(if i > 0 { ",{\"wl\":" } else { "{\"wl\":" }).num(*w).raw(",\"bw\":").opt(*bw).raw("}");
        }
        o.raw("],\"oven\":").opt(self.oven).raw("}");
    }
}
