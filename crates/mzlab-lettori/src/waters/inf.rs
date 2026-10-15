//! The text files of the folder: `_HEADER.TXT` (`$$ key: value`, the sample and the calibrations), `_extern.inf` (the instrument parameters, global and by function) and
//! `_INLET.INF` (the LC method). Every line is kept as a (section, key, value) so that nothing is lost; the fields that the reader uses are also parsed.

use super::dat::Taratura;
use std::collections::BTreeMap;

/// The most lines kept from one text file.
const VOCI_MAX: usize = 5000;

#[derive(Debug, Clone, Default)]
pub(super) struct Voce {
    pub(super) sezione: String,
    pub(super) chiave: String,
    pub(super) valore: String,
}

fn numero(v: &str) -> Option<f64> {
    v.split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite())
}

fn polarita(v: &str) -> Option<i8> {
    let v = v.trim().to_ascii_uppercase();
    if v.starts_with("ES+") || v.starts_with("POS") {
        Some(1)
    } else if v.starts_with("ES-") || v.starts_with("NEG") {
        Some(-1)
    } else {
        None
    }
}

// ---- _HEADER.TXT ----

#[derive(Debug, Default)]
pub(super) struct Testata {
    pub(super) voci: Vec<(String, String)>,
    pub(super) tarature: BTreeMap<u32, Taratura>,
}

impl Testata {
    /// The value of `chiave` when the file gives one.
    pub(super) fn get(&self, chiave: &str) -> Option<&str> {
        self.voci
            .iter()
            .find(|(k, v)| k == chiave && !v.trim().is_empty())
            .map(|(_, v)| v.trim())
    }
}

pub(super) fn testata(t: &str) -> Testata {
    let mut h = Testata::default();
    for riga in t.lines() {
        let Some(r) = riga.trim().strip_prefix("$$") else {
            continue;
        };
        let Some((k, v)) = r
            .split_once(": ")
            .or_else(|| r.trim_end().strip_suffix(':').map(|k| (k, "")))
        else {
            continue;
        };
        if h.voci.len() >= VOCI_MAX {
            break;
        }
        let (k, v) = (k.trim(), v.trim());
        if let Some(n) = k
            .strip_prefix("Cal Function ")
            .and_then(|n| n.trim().parse::<u32>().ok())
        {
            if let Some(t) = Taratura::da_testo(v) {
                h.tarature.insert(n, t);
            }
        }
        h.voci.push((k.to_string(), v.to_string()));
    }
    h
}

// ---- _extern.inf ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Modo {
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
pub(super) struct FunzioneExt {
    pub(super) etichetta: String,
    pub(super) modo: Modo,
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
    pub(super) voci: Vec<Voce>,
    pub(super) funzioni: BTreeMap<u32, FunzioneExt>,
    /// The polarity of the global section (functions without their own).
    pub(super) polarita: Option<i8>,
}

fn modo_di(coda: &str) -> Modo {
    let t = coda.to_ascii_uppercase();
    if t.contains("REFERENCE") {
        Modo::Riferimento
    } else if t.contains("DAUGHTER") {
        Modo::Figlie
    } else if t.contains("MSMS") {
        Modo::Msms
    } else if t.contains("PARENT") {
        Modo::Parent
    } else if t.contains("TOF MS") {
        Modo::Ms
    } else {
        Modo::Altro
    }
}

/// «Function Parameters - Function 2 - TOF MS FUNCTION» → (2, «TOF MS FUNCTION»).
fn intestazione_funzione(r: &str) -> Option<(u32, &str)> {
    let rest = r.strip_prefix("Function Parameters - Function ")?;
    let (n, coda) = rest.split_once('-').unwrap_or((rest, ""));
    Some((n.trim().parse().ok()?, coda.trim()))
}

/// The key and value of a line: split at the first tab, else at « - », else the whole line is a key.
fn coppia(r: &str) -> (String, String) {
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

type Rampe = BTreeMap<u32, (Option<f64>, Option<f64>)>;

/// One "Collision Energy" line of function `n`: an end of a ramp or a fixed value.
fn energia(e: &mut Extern, rampe: &mut Rampe, n: u32, b: &str, v: &str) {
    if b.ends_with("Collision Energy Ramp Start") || b.ends_with("Collision Energy Low") {
        rampe.entry(n).or_default().0 = numero(v);
    } else if b.ends_with("Collision Energy Ramp End") || b.ends_with("Collision Energy High") {
        rampe.entry(n).or_default().1 = numero(v);
    } else if let Some((a, z)) = v.split_once("(Ramping to") {
        rampe.insert(n, (numero(a), numero(z.trim_end_matches(')'))));
    } else if !b.contains("Use ") && !b.contains("Manual") {
        let f = e.funzioni.entry(n).or_default();
        f.ce = f.ce.or(numero(v));
    }
}

pub(super) fn extern_inf(t: &str) -> Extern {
    let mut e = Extern::default();
    let (mut sezione, mut funz) = (String::new(), None::<u32>);
    let mut rampe = Rampe::new();
    for riga in t.lines() {
        let r = riga.trim();
        if r.is_empty() {
            continue;
        }
        if let Some((n, coda)) = intestazione_funzione(r) {
            funz = Some(n);
            sezione = r.to_string();
            let f = e.funzioni.entry(n).or_default();
            f.modo = modo_di(coda);
            f.mobilita = coda.to_ascii_uppercase().contains("MOBILITY");
            f.etichetta = coda.to_string();
            continue;
        }
        if r.ends_with(':') && !r.contains('\t') {
            let titolo = r.trim_end_matches(':').trim();
            funz = titolo
                .strip_prefix("Instrument Parameters - Function ")
                .and_then(|n| n.trim().parse().ok());
            sezione = titolo.to_string();
            if let Some(n) = funz {
                e.funzioni.entry(n).or_default();
            }
            continue;
        }
        let (k, v) = coppia(r);
        let base = k.split('(').next().unwrap_or(&k).trim_end().to_string();
        match (base.as_str(), funz) {
            ("Polarity", f) => {
                let p = polarita(&v);
                match f {
                    Some(n) => e.funzioni.entry(n).or_default().polarita = p,
                    None => e.polarita = e.polarita.or(p),
                }
            }
            ("Set Mass", Some(n)) => {
                e.funzioni.entry(n).or_default().set_mass = numero(&v).filter(|&m| m > 0.0);
            }
            (b, Some(n)) if b.contains("Collision Energy") => {
                energia(&mut e, &mut rampe, n, b, &v);
            }
            _ => {}
        }
        if e.voci.len() < VOCI_MAX {
            e.voci.push(Voce {
                sezione: sezione.clone(),
                chiave: k,
                valore: v,
            });
        }
    }
    for (n, (a, z)) in rampe {
        if let (Some(f), Some(a), Some(z)) = (e.funzioni.get_mut(&n), a, z) {
            if z > a {
                f.rampa = Some((a, z));
            }
        }
    }
    e
}

// ---- _INLET.INF ----

#[derive(Debug, Default)]
pub(super) struct Inlet {
    pub(super) voci: Vec<Voce>,
    /// (time min, flow mL/min, %B) of the first gradient table.
    pub(super) gradiente: Vec<(f64, Option<f64>, Option<f64>)>,
    pub(super) durata_min: Option<f64>,
    pub(super) forno_c: Option<f64>,
}

/// A row «1. Initial 0.250 65.0 35.0 Initial» or «2. 1.00 0.250 60.0 40.0 6».
fn riga_gradiente(r: &str) -> Option<(f64, Option<f64>, Option<f64>)> {
    let t: Vec<&str> = r.split_whitespace().collect();
    if t.len() < 5 || !t[0].ends_with('.') || t[0].trim_end_matches('.').parse::<u32>().is_err() {
        return None;
    }
    let tempo = if t[1] == "Initial" {
        0.0
    } else {
        t[1].parse().ok()?
    };
    Some((tempo, t[2].parse().ok(), t[4].parse().ok()))
}

pub(super) fn inlet(t: &str) -> Inlet {
    let mut l = Inlet::default();
    let (mut sezione, mut in_tabella, mut fatto) = (String::new(), false, false);
    for riga in t.lines() {
        let r = riga.trim();
        if r.starts_with("--") && r.ends_with("--") {
            sezione = r.trim_matches('-').trim().to_string();
            in_tabella = false;
            continue;
        }
        if r.starts_with('[') {
            in_tabella = !fatto && r == "[Gradient Table]";
            continue;
        }
        if in_tabella {
            match riga_gradiente(r) {
                Some(g) => l.gradiente.push(g),
                None if r.starts_with("Time") => {}
                None => {
                    in_tabella = false;
                    fatto = !l.gradiente.is_empty();
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
            "Run Time" if l.durata_min.is_none() => l.durata_min = numero(v),
            "Target Column Temperature" if l.forno_c.is_none() => l.forno_c = numero(v),
            _ => {}
        }
        if l.voci.len() < VOCI_MAX {
            l.voci.push(Voce {
                sezione: sezione.clone(),
                chiave: k.to_string(),
                valore: v.to_string(),
            });
        }
    }
    l
}
