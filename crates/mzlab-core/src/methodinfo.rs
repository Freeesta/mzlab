//! Acquisition parameters stored in Analyst `.dam` (method) and `.wiff` (data) files: `read_methods` of `mzlab/reader/methodinfo.py`.
//! Values are 32-bit floats after the UTF-16 parameter name; the LC method is XML. Anything not recognised is left out.

use crate::error::Result;
use crate::json::round;
use crate::ole::Ole;

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

#[derive(Debug, Clone, Default)]
pub struct Transition {
    pub q1: f64,
    pub q3: f64,
    pub dwell: f64,
    pub name: String,
    pub ce: Option<f64>,
    pub dp: Option<f64>,
    pub ep: Option<f64>,
    pub cep: Option<f64>,
    pub cxp: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum Kind {
    Mrm(Vec<Transition>),
    Scan {
        range: Option<[f64; 2]>,
        ce: Vec<f64>,
    },
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
    d.get(at..at + 4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64)
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

/// Searches `from..to` for the UTF-16LE parameter tag `tag` (e.g. `b"\x04\x00C\x00E\x00"`) and returns the `f32` value immediately after it.
fn param_after(d: &[u8], from: usize, to: usize, tag: &[u8]) -> Option<f64> {
    find(d, from, to, tag)
        .and_then(|i| f32_at(d, i + tag.len()))
        .filter(|v| v.is_finite())
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
    if i + 22 > d.len()
        || d[i + 4..i + 8] != [0; 4]
        || d[i + 16..i + 20] != [0; 4]
        || d[i + 21] != 0
    {
        return None;
    }
    let mut p = i + 22;
    while p + 1 < d.len() && (0x20..=0x7e).contains(&d[p]) && d[p + 1] == 0 {
        p += 2;
    }
    if !d[p..].starts_with(&[4, 0, b'D', 0, b'P', 0]) {
        return None;
    }
    Some(MrmHit {
        start: i,
        end: p + 6,
        q1: f32_at(d, i)?,
        q3: f32_at(d, i + 8)?,
        dwell: f32_at(d, i + 12)?,
        len: d[i + 20],
        name: d[i + 22..p].to_vec(),
    })
}

fn transition_from_hit(d: &[u8], h: &MrmHit, end: usize) -> Transition {
    let p = |tag: &[u8]| param_after(d, h.start + 22, end, tag).map(|v| round(v, 2));
    Transition {
        q1: round(h.q1, 2),
        q3: round(h.q3, 2),
        dwell: round(h.dwell, 1),
        name: ascii(&h.name),
        ce: p(&[4, 0, b'C', 0, b'E', 0]),
        dp: p(&[4, 0, b'D', 0, b'P', 0]),
        ep: p(&[4, 0, b'E', 0, b'P', 0]),
        cep: p(&[6, 0, b'C', 0, b'E', 0, b'P', 0]),
        cxp: p(&[6, 0, b'C', 0, b'X', 0, b'P', 0]),
    }
}

fn experiments(ole: &Ole, base: &str) -> Vec<Experiment> {
    let mut out = Vec::new();
    for i in 0..16 {
        let key = format!("{base}DeviceMethod0/Period0/Experiment{i}/MassRangeEx/MassRangeEx");
        let Some(d) = ole.read(&key) else { break };
        let is_mrm =
            ole.read(&format!(
                "{base}DeviceMethod0/Period0/Experiment{i}/ExperimentHeader"
            ))
            .and_then(|h| {
                h.get(122..126)
                    .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            }) == Some(4);
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
        hits.retain(|h| h.name.len() == h.len as usize && (is_mrm || !h.name.is_empty()));
        if !hits.is_empty() {
            let tr = hits
                .iter()
                .enumerate()
                .map(|(j, h)| {
                    transition_from_hit(&d, h, hits.get(j + 1).map_or(d.len(), |n| n.start))
                })
                .collect();
            out.push(Experiment {
                index: i,
                kind: Kind::Mrm(tr),
            });
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
        crate::sortc::by(&mut ce, |a, b| a.total_cmp(b));
        let mut range = None;
        if d.len() >= 244 {
            let a = f32_at(&d, 40).unwrap_or(f64::NAN);
            let b = if d.len() == 432 {
                f32_at(&d, 240)
            } else {
                f32_at(&d, 44)
            };
            if let Some(b) = b {
                if 10.0 <= a && a < b && b <= 3000.0 {
                    range = Some([round(a, 2), round(b, 2)]);
                }
            }
        }
        out.push(Experiment {
            index: i,
            kind: Kind::Scan { range, ce },
        });
    }
    out
}

#[path = "methodinfo/lc.rs"]
mod lc;
use lc::lc;

fn named(id: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    NAMES.iter().find(|n| n.0 == id)
}

fn source(ole: &Ole, base: &str) -> Vec<Param> {
    let mut out = Vec::new();
    for i in 0..24 {
        let Some(d) = ole.read(&format!("{base}DeviceMethod0/Period0/Experiment0/IonSourceParamsTable/Parameter{i}/ParameterData")) else { break };
        let body = d.get(16..).unwrap_or(&[]);
        let Some(s) = (0..body.len()).find(|&s| pairs(body, s, 6) >= 2) else {
            continue;
        };
        let n = pairs(body, s, 6);
        let id = ascii(&body[s..s + 2 * n]);
        let end = 16 + s + 2 * n;
        if let (Some(v), Some(nm)) = (f32_at(&d, end), named(&id)) {
            out.push(Param {
                id,
                label: nm.1,
                value: num_param(v),
                unit: nm.2,
            });
        }
    }
    out
}

fn compound(ole: &Ole, base: &str) -> Vec<Param> {
    let mut out: Vec<Param> = Vec::new();
    let Some(d) = ole.read(&format!(
        "{base}DeviceMethod0/Period0/Experiment0/MassRangeEx/MassRangeEx"
    )) else {
        return out;
    };
    let mut at = 0;
    while at + 2 < d.len() {
        let n = if (2..=12).contains(&d[at]) && d[at + 1] == 0 {
            pairs(&d, at + 2, 4)
        } else {
            0
        };
        if n < 2 {
            at += 1;
            continue;
        }
        let end = at + 2 + 2 * n;
        let id = ascii(&d[at + 2..end]);
        if let (Some(nm), true) = (named(&id), !out.iter().any(|p| p.id == id)) {
            if let Some(v) = f32_at(&d, end) {
                out.push(Param {
                    id,
                    label: nm.1,
                    value: num_param(v),
                    unit: nm.2,
                });
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

const DAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

/// `[A-Z]:\\.*?\.dam`
fn dam_path(t: &str) -> Option<&str> {
    let b = t.as_bytes();
    (0..b.len().saturating_sub(2))
        .find(|&i| b[i].is_ascii_uppercase() && b[i + 1] == b':' && b[i + 2] == b'\\')
        .and_then(|i| t[i + 3..].find(".dam").map(|j| &t[i..i + 3 + j + 4]))
}

/// `[A-Z][a-z]+day .*?[AP]M`
fn stamp(t: &str) -> Option<&str> {
    let b = t.as_bytes();
    for i in 0..b.len() {
        if !b[i].is_ascii_uppercase() {
            continue;
        }
        let run = b[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_lowercase())
            .count();
        let Some(k) = (1..=run)
            .rev()
            .find(|&k| b[i + 1 + k..].starts_with(b"day "))
        else {
            continue;
        };
        let from = i + 1 + k + 4;
        if let Some(j) = (from..b.len().saturating_sub(1))
            .find(|&j| (b[j] == b'A' || b[j] == b'P') && b[j + 1] == b'M')
        {
            return Some(&t[i..j + 2]);
        }
    }
    None
}

fn file_info(ole: &Ole, base: &str) -> Vec<(&'static str, String)> {
    let mut f: Vec<(&'static str, String)> = Vec::new();
    for fname in [
        format!("{base}AcqMethodFileInfoStm"),
        "FileRec_Str".to_string(),
    ] {
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
    Method {
        source: source(ole, base),
        compound: compound(ole, base),
        file: file_info(ole, base),
        experiments: experiments(ole, base),
        lc: lc(ole, base),
        ..Default::default()
    }
}

fn digits(s: &str) -> i64 {
    s.chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// The method of sample `n` of a `.wiff` (`None` when the file has no `MethodN`: the samples of a batch may outnumber the methods).
pub fn method_of_sample(ole: &Ole, n: u32) -> Option<Method> {
    let base = format!("MethodSubtree/Method{n}/");
    ole.has(&format!(
        "{base}DeviceMethod0/Period0/Experiment0/ExperimentHeader"
    ))
    .then(|| Method {
        name: format!("Method {n}"),
        sample: Some(n as i64),
        ..method(ole, &base)
    })
}

/// `[{name, source, compound, file, ...}]` for a `.dam` (one method) or a `.wiff` (one per sample).
pub fn read_methods(data: Vec<u8>, file_name: &str) -> Result<Vec<Method>> {
    let ole = Ole::open(data, file_name)?;
    let mut subs: Vec<String> = ole
        .paths_list()
        .filter(|k| k.starts_with("MethodSubtree/Method"))
        .filter_map(|k| k.split('/').nth(1))
        .map(String::from)
        .collect();
    crate::sortc::any_by_key(&mut subs, |s| digits(s));
    subs.dedup();
    if !subs.is_empty() {
        return Ok(subs
            .iter()
            .map(|s| Method {
                name: s.replace("Method", "Method "),
                sample: Some(digits(s)),
                ..method(&ole, &format!("MethodSubtree/{s}/"))
            })
            .collect());
    }
    Ok(vec![Method {
        name: file_name.to_string(),
        sample: None,
        ..method(&ole, "")
    }])
}
