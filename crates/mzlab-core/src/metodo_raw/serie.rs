//! The instrument log of a `.raw` as series over time: every numeric channel of the status log (`status:<section>/<label>`) and every numeric field of the scan trailer that changes
//! (`trailer:<label>`, one point per scan). A channel with fewer than two different values stays in `campi` only. The list (`Voce`) carries no points; the points of one series are
//! computed when it is asked for (`Lettore::cromatogramma`), never at opening. The series are not chromatograms of the file: they are not in the mzML nor in the `.mzidx`.

use crate::metodo::{num, obj, text, V};
use opentfraw::generic_data::{GenericDataHeader, GenericRecord, GenericValue};
use opentfraw::status_log::{split_label, StatusLog};
use opentfraw::{GenericType, MsPower, RawFileReader};

pub const STATUS: &str = "status:";
pub const TRAILER: &str = "trailer:";

/// One series of the list: what the page shows to choose it.
#[derive(Debug, Clone, PartialEq)]
pub struct Voce {
    pub id: String,
    /// The section of the status log (`Ion Source`); empty for the trailer and for a channel before any title.
    pub gruppo: String,
    pub nome: String,
    pub unita: String,
    pub n: usize,
    pub min: f64,
    pub max: f64,
}

/// The points of one series: time in minutes and value.
#[derive(Debug, Clone, PartialEq)]
pub struct Serie {
    pub unita: String,
    pub t: Vec<f64>,
    pub y: Vec<f64>,
}

/// A scan as the trailer series see it.
#[derive(Debug, Clone, Copy)]
pub struct Scansione {
    pub t_min: f64,
    pub livello: u8,
}

/// Where the values of a series are: the field of the schema and, for the trailer, the MS level whose scans it takes.
struct Spec {
    voce: Voce,
    campo: usize,
    livello: Option<u8>,
}

/// `====  Ion Source:  ====` → `Ion Source`.
fn titolo(label: &str) -> String {
    label
        .trim()
        .trim_matches(|c: char| c == '=' || c == ':' || c.is_whitespace())
        .to_string()
}

#[derive(Default)]
struct Statistiche {
    n: usize,
    min: f64,
    max: f64,
    primo: f64,
    diverso: bool,
}

impl Statistiche {
    fn push(&mut self, y: f64) {
        if self.n == 0 {
            (self.min, self.max, self.primo) = (y, y, y);
        } else {
            self.min = self.min.min(y);
            self.max = self.max.max(y);
            self.diverso |= y != self.primo;
        }
        self.n += 1;
    }
}

fn numero(r: &GenericRecord, campo: usize) -> Option<f64> {
    let v = &r.values.get(campo)?.1;
    if matches!(v, GenericValue::Bool(_)) {
        return None;
    }
    v.as_f64().filter(|x| x.is_finite())
}

fn spec(
    id: String,
    gruppo: String,
    nome: String,
    unita: String,
    s: &Statistiche,
    campo: usize,
    livello: Option<u8>,
) -> Option<Spec> {
    (s.n >= 2 && s.diverso).then_some(Spec {
        voce: Voce {
            id,
            gruppo,
            nome,
            unita,
            n: s.n,
            min: s.min,
            max: s.max,
        },
        campo,
        livello,
    })
}

/// The id with a number after it when `usati` already has it.
fn unico(base: String, usati: &mut Vec<String>) -> String {
    let mut id = base.clone();
    let mut k = 1;
    while usati.contains(&id) {
        k += 1;
        id = format!("{base} #{k}");
    }
    usati.push(id.clone());
    id
}

fn specs_stato(log: &StatusLog) -> Vec<Spec> {
    if log.times_min.len() != log.records.len() {
        return Vec::new();
    }
    let (mut out, mut usati, mut sezione) = (Vec::new(), Vec::new(), String::new());
    for (i, d) in log.header.fields.iter().enumerate() {
        if d.field_type == GenericType::Gap {
            sezione = titolo(&d.label);
            continue;
        }
        let (nome, unita) = split_label(&d.label);
        if nome.is_empty() {
            continue;
        }
        let mut s = Statistiche::default();
        for (r, t) in log.records.iter().zip(&log.times_min) {
            if let (Some(y), true) = (numero(r, i), t.is_finite()) {
                s.push(y);
            }
        }
        let base = format!("{STATUS}{sezione}/{nome}");
        if s.n >= 2 && s.diverso {
            let id = unico(base, &mut usati);
            out.extend(spec(id, sezione.clone(), nome, unita, &s, i, None));
        }
    }
    out
}

fn specs_trailer(h: &GenericDataHeader, recs: &[GenericRecord], scans: &[Scansione]) -> Vec<Spec> {
    let livelli: std::collections::BTreeSet<u8> = scans.iter().map(|s| s.livello).collect();
    let (mut out, mut usati) = (Vec::new(), Vec::new());
    for (i, d) in h.fields.iter().enumerate() {
        if d.field_type == GenericType::Gap {
            continue;
        }
        let (nome, unita) = split_label(&d.label);
        if nome.is_empty() {
            continue;
        }
        let mut per_livello: Vec<(u8, Statistiche)> = Vec::new();
        for l in livelli.iter() {
            let mut s = Statistiche::default();
            for (r, sc) in recs.iter().zip(scans).filter(|(_, sc)| sc.livello == *l) {
                if let (Some(y), true) = (numero(r, i), sc.t_min.is_finite()) {
                    s.push(y);
                }
            }
            if s.n >= 2 && s.diverso {
                per_livello.push((*l, s));
            }
        }
        // a field that changes at one level only keeps its plain name; at several levels each one is a series of its own
        let separati = per_livello.len() > 1;
        for (l, s) in &per_livello {
            let (base, etichetta) = if separati {
                (format!("{TRAILER}{nome}@MS{l}"), format!("{nome} (MS{l})"))
            } else {
                (format!("{TRAILER}{nome}"), nome.clone())
            };
            let id = unico(base, &mut usati);
            out.extend(spec(
                id,
                String::new(),
                etichetta,
                unita.clone(),
                s,
                i,
                Some(*l),
            ));
        }
    }
    out
}

/// The status series that vary, in the order of the file.
pub fn indice_stato(log: &StatusLog) -> Vec<Voce> {
    specs_stato(log).into_iter().map(|s| s.voce).collect()
}

/// The trailer series that vary, in the order of the file.
pub fn indice_trailer(
    h: &GenericDataHeader,
    recs: &[GenericRecord],
    scans: &[Scansione],
) -> Vec<Voce> {
    specs_trailer(h, recs, scans)
        .into_iter()
        .map(|s| s.voce)
        .collect()
}

/// The points of the status series `id` (`None` when the id is not one of the list).
pub fn serie_stato(log: &StatusLog, id: &str) -> Option<Serie> {
    let s = specs_stato(log).into_iter().find(|s| s.voce.id == id)?;
    let (mut t, mut y) = (Vec::new(), Vec::new());
    for (r, tm) in log.records.iter().zip(&log.times_min) {
        if let (Some(v), true) = (numero(r, s.campo), tm.is_finite()) {
            t.push(f64::from(*tm));
            y.push(v);
        }
    }
    Some(Serie {
        unita: s.voce.unita,
        t,
        y,
    })
}

/// The points of the trailer series `id`, one per scan of its level.
pub fn serie_trailer(
    h: &GenericDataHeader,
    recs: &[GenericRecord],
    scans: &[Scansione],
    id: &str,
) -> Option<Serie> {
    let s = specs_trailer(h, recs, scans)
        .into_iter()
        .find(|s| s.voce.id == id)?;
    let (mut t, mut y) = (Vec::new(), Vec::new());
    for (r, sc) in recs.iter().zip(scans) {
        if s.livello.is_some_and(|l| l != sc.livello) || !sc.t_min.is_finite() {
            continue;
        }
        if let Some(v) = numero(r, s.campo) {
            t.push(sc.t_min);
            y.push(v);
        }
    }
    Some(Serie {
        unita: s.voce.unita,
        t,
        y,
    })
}

/// The time and the MS level of every scan, from the scan index and the scan events (no scan is read; an SRM run is all level 2).
fn scansioni(raw: &RawFileReader) -> Vec<Scansione> {
    raw.scan_index
        .iter()
        .enumerate()
        .map(|(k, e)| Scansione {
            t_min: e.start_time,
            livello: if raw.flat_peaks {
                2
            } else {
                raw.scan_events
                    .get(k)
                    .and_then(|ev| ev.preamble.ms_power())
                    .map_or(1, |p| if p == MsPower::Undefined { 1 } else { p as u8 })
            },
        })
        .collect()
}

/// Every series of the file: the status log first, then the trailer.
pub fn indice(raw: &RawFileReader) -> Vec<Voce> {
    let mut out = raw
        .status_log
        .as_ref()
        .map(indice_stato)
        .unwrap_or_default();
    out.extend(indice_trailer(
        &raw.scan_parameters_header,
        &raw.scan_parameters,
        &scansioni(raw),
    ));
    out
}

/// The list as the `serie_registro` section of `metodo/1`: no points, the page asks for them one series at a time.
pub fn sezione(voci: &[Voce]) -> Vec<V> {
    voci.iter()
        .filter_map(|v| {
            obj(vec![
                ("id", text(&v.id)),
                ("gruppo", text(&v.gruppo)),
                ("nome", text(&v.nome)),
                ("unita", text(&v.unita)),
                ("n", num(Some(v.n as f64))),
                ("min", num(Some(v.min))),
                ("max", num(Some(v.max))),
            ])
        })
        .collect()
}

/// The points of the series `id` of the list (`None` for any other id).
pub fn serie(raw: &RawFileReader, id: &str) -> Option<Serie> {
    if id.starts_with(STATUS) {
        serie_stato(raw.status_log.as_ref()?, id)
    } else if id.starts_with(TRAILER) {
        serie_trailer(
            &raw.scan_parameters_header,
            &raw.scan_parameters,
            &scansioni(raw),
            id,
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
