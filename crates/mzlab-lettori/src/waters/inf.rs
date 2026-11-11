//! The text files of the folder: `_HEADER.TXT` (`$$ key: value`, the sample and the calibrations), `_extern.inf` (the instrument parameters, global and by function) and
//! `_INLET.INF` (the LC method). Every line is kept as a (section, key, value) so that nothing is lost; the fields that the reader uses are also parsed.

use super::dat::Calibration;
use std::collections::BTreeMap;

/// The most lines kept from one text file.
const ENTRIES_MAX: usize = 5000;

#[derive(Debug, Clone, Default)]
pub(super) struct Entry {
    pub(super) section: String,
    pub(super) chiave: String,
    pub(super) valore: String,
}

fn number(v: &str) -> Option<f64> {
    v.split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite())
}

/// The polarity of an ion mode: `ES+`, `Positive`, or a mode word ending in the sign (`ESCi+`, `APCI-`).
fn polarity(v: &str) -> Option<i8> {
    let v = v.trim().to_ascii_uppercase();
    let word = v.split_whitespace().next().unwrap_or("");
    if v.starts_with("ES+") || v.starts_with("POS") || word.ends_with('+') {
        Some(1)
    } else if v.starts_with("ES-") || v.starts_with("NEG") || word.ends_with('-') {
        Some(-1)
    } else {
        None
    }
}

// ---- _HEADER.TXT ----

#[derive(Debug, Default)]
pub(super) struct WatersHeader {
    pub(super) entries: Vec<(String, String)>,
    pub(super) tarature: BTreeMap<u32, Calibration>,
}

impl WatersHeader {
    /// The value of `chiave` when the file gives one.
    pub(super) fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, v)| k == key && !v.trim().is_empty())
            .map(|(_, v)| v.trim())
    }
}

pub(super) fn file_header(t: &str) -> WatersHeader {
    let mut h = WatersHeader::default();
    for row in t.lines() {
        let Some(r) = row.trim().strip_prefix("$$") else {
            continue;
        };
        let Some((k, v)) = r
            .split_once(": ")
            .or_else(|| r.trim_end().strip_suffix(':').map(|k| (k, "")))
        else {
            continue;
        };
        if h.entries.len() >= ENTRIES_MAX {
            break;
        }
        let (k, v) = (k.trim(), v.trim());
        if let Some(n) = k
            .strip_prefix("Cal Function ")
            .and_then(|n| n.trim().parse::<u32>().ok())
            && let Some(t) = Calibration::from_text(v)
        {
            h.tarature.insert(n, t);
        }
        h.entries.push((k.to_string(), v.to_string()));
    }
    h
}

// ---- _extern.inf ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Mode {
    Ms,
    /// «TOF PARENT»: the survey function of MSe or SONAR.
    Parent,
    Msms,
    Figlie,
    Riferimento,
    #[default]
    Altro,
}

#[derive(Debug, Clone, Default)]
pub(super) struct FunctionExt {
    pub(super) label: String,
    pub(super) mode: Mode,
    /// The label names ion mobility (not decoded).
    pub(super) mobilita: bool,
    pub(super) polarita: Option<i8>,
    pub(super) set_mass: Option<f64>,
    /// Collision energy ramp (start, end) in eV.
    pub(super) rampa: Option<(f64, f64)>,
    pub(super) ce: Option<f64>,
}

#[derive(Debug, Default)]
pub(super) struct Extern {
    pub(super) voci: Vec<Entry>,
    pub(super) funzioni: BTreeMap<u32, FunctionExt>,
    /// The polarity of the global section (functions without their own).
    pub(super) polarita: Option<i8>,
}

fn mode_of(tail: &str) -> Mode {
    let t = tail.to_ascii_uppercase();
    if t.contains("REFERENCE") {
        Mode::Riferimento
    } else if t.contains("DAUGHTER") {
        Mode::Figlie
    } else if t.contains("MSMS") {
        Mode::Msms
    } else if t.contains("PARENT") {
        Mode::Parent
    } else if t.contains("TOF MS") {
        Mode::Ms
    } else {
        Mode::Altro
    }
}

/// «Function Parameters - Function 2 - TOF MS FUNCTION» → (2, «TOF MS FUNCTION»).
fn function_header(r: &str) -> Option<(u32, &str)> {
    let rest = r.strip_prefix("Function Parameters - Function ")?;
    let (n, tail) = rest.split_once('-').unwrap_or((rest, ""));
    Some((n.trim().parse().ok()?, tail.trim()))
}

/// The key and value of a line: split at the first tab, else at « - », else the whole line is a key.
fn pair(r: &str) -> (String, String) {
    let (k, v) = match r.split_once('\t') {
        Some((k, v)) => (k, v),
        None => r.split_once(" - ").unwrap_or((r, "")),
    };
    let v: Vec<&str> = v
        .split('\t')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    (
        k.trim().trim_end_matches(':').trim().to_string(),
        v.join("; "),
    )
}

type Ramps = BTreeMap<u32, (Option<f64>, Option<f64>)>;

/// One "Collision Energy" line of function `n`: an end of a ramp or a fixed value.
fn energy(e: &mut Extern, ramps: &mut Ramps, n: u32, b: &str, v: &str) {
    if b.ends_with("Collision Energy Ramp Start") || b.ends_with("Collision Energy Low") {
        ramps.entry(n).or_default().0 = number(v);
    } else if b.ends_with("Collision Energy Ramp End") || b.ends_with("Collision Energy High") {
        ramps.entry(n).or_default().1 = number(v);
    } else if let Some((a, z)) = v.split_once("(Ramping to") {
        ramps.insert(n, (number(a), number(z.trim_end_matches(')'))));
    } else if !b.contains("Use ") && !b.contains("Manual") {
        let f = e.funzioni.entry(n).or_default();
        f.ce = f.ce.or(number(v));
    }
}

pub(super) fn extern_inf(t: &str) -> Extern {
    let mut e = Extern::default();
    let (mut sezione, mut funz) = (String::new(), None::<u32>);
    let mut ramps = Ramps::new();
    for row in t.lines() {
        let r = row.trim();
        if r.is_empty() {
            continue;
        }
        if let Some((n, tail)) = function_header(r) {
            funz = Some(n);
            sezione = r.to_string();
            let f = e.funzioni.entry(n).or_default();
            f.mode = mode_of(tail);
            f.mobilita = tail.to_ascii_uppercase().contains("MOBILITY");
            f.label = tail.to_string();
            continue;
        }
        if r.ends_with(':') && !r.contains('\t') {
            let title = r.trim_end_matches(':').trim();
            funz = title
                .strip_prefix("Instrument Parameters - Function ")
                .and_then(|n| n.trim().parse().ok());
            sezione = title.to_string();
            if let Some(n) = funz {
                e.funzioni.entry(n).or_default();
            }
            continue;
        }
        let (k, v) = pair(r);
        let base = k.split('(').next().unwrap_or(&k).trim_end().to_string();
        match (base.as_str(), funz) {
            ("Polarity", f) => {
                let p = polarity(&v);
                match f {
                    Some(n) => e.funzioni.entry(n).or_default().polarita = p,
                    None => e.polarita = e.polarita.or(p),
                }
            }
            ("Set Mass", Some(n)) => {
                e.funzioni.entry(n).or_default().set_mass = number(&v).filter(|&m| m > 0.0);
            }
            (b, Some(n)) if b.contains("Collision Energy") => {
                energy(&mut e, &mut ramps, n, b, &v);
            }
            _ => {}
        }
        if e.voci.len() < ENTRIES_MAX {
            e.voci.push(Entry {
                section: sezione.clone(),
                chiave: k,
                valore: v,
            });
        }
    }
    for (n, (a, z)) in ramps {
        if let (Some(f), Some(a), Some(z)) = (e.funzioni.get_mut(&n), a, z)
            && z > a
        {
            f.rampa = Some((a, z));
        }
    }
    e
}

// ---- _INLET.INF ----

#[derive(Debug, Default)]
pub(super) struct Inlet {
    pub(super) entries: Vec<Entry>,
    /// (time min, flow mL/min, %B) of the first gradient table.
    pub(super) gradiente: Vec<(f64, Option<f64>, Option<f64>)>,
    pub(super) duration_min: Option<f64>,
    pub(super) oven_c: Option<f64>,
}

/// A row «1. Initial 0.250 65.0 35.0 Initial» or «2. 1.00 0.250 60.0 40.0 6».
fn gradient_row(r: &str) -> Option<(f64, Option<f64>, Option<f64>)> {
    let t: Vec<&str> = r.split_whitespace().collect();
    if t.len() < 5 || !t[0].ends_with('.') || t[0].trim_end_matches('.').parse::<u32>().is_err() {
        return None;
    }
    let moment = if t[1] == "Initial" {
        0.0
    } else {
        t[1].parse().ok()?
    };
    Some((moment, t[2].parse().ok(), t[4].parse().ok()))
}

pub(super) fn inlet(t: &str) -> Inlet {
    let mut l = Inlet::default();
    let (mut sezione, mut in_table, mut done) = (String::new(), false, false);
    for row in t.lines() {
        let r = row.trim();
        if r.starts_with("--") && r.ends_with("--") {
            sezione = r.trim_matches('-').trim().to_string();
            in_table = false;
            continue;
        }
        if r.starts_with('[') {
            in_table = !done && r == "[Gradient Table]";
            continue;
        }
        if in_table {
            match gradient_row(r) {
                Some(g) => l.gradiente.push(g),
                None if r.starts_with("Time") => {}
                None => {
                    in_table = false;
                    done = !l.gradiente.is_empty();
                }
            }
            continue;
        }
        let Some((k, v)) = r.split_once(':') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        if k.is_empty() {
            continue;
        }
        match k {
            "Run Time" if l.duration_min.is_none() => l.duration_min = number(v),
            "Target Column Temperature" if l.oven_c.is_none() => l.oven_c = number(v),
            _ => {}
        }
        if l.entries.len() < ENTRIES_MAX {
            l.entries.push(Entry {
                section: sezione.clone(),
                chiave: k.to_string(),
                valore: v.to_string(),
            });
        }
    }
    l
}
