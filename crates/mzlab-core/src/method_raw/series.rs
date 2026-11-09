//! The instrument log of a `.raw` as series over time: every numeric channel of the status log (`status:<section>/<label>`) and every numeric field of the scan trailer that changes
//! (`trailer:<label>`, one point per scan). A channel with fewer than two different values stays in `campi` only. The list (`Voce`) carries no points; the points of one series are
//! computed when it is asked for (`Lettore::chromatogram`), never at opening. The series are not chromatograms of the file: they are not in the mzML nor in the `.mzidx`.

use crate::method::{num, obj, text, V};
use opentfraw::generic_data::{GenericDataHeader, GenericRecord, GenericValue};
use opentfraw::status_log::{split_label, StatusLog};
use opentfraw::{GenericType, MsPower, RawFileReader};
use std::io::{Read, Seek};

pub const STATUS: &str = "status:";
pub const TRAILER: &str = "trailer:";

/// One series of the list: what the page shows to choose it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: String,
    /// The section of the status log (`Ion Source`); empty for the trailer and for a channel before any title.
    pub group: String,
    pub name: String,
    pub unita: String,
    pub n: usize,
    pub min: f64,
    pub max: f64,
}

/// The points of one series: time in minutes and value.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub unita: String,
    pub t: Vec<f64>,
    pub y: Vec<f64>,
}

/// A scan as the trailer series see it.
#[derive(Debug, Clone, Copy)]
pub struct Scansione {
    pub t_min: f64,
    pub level: u8,
}

/// Where the values of a series are: the field of the schema and, for the trailer, the MS level whose scans it takes.
struct Spec {
    entry: Entry,
    field: usize,
    level: Option<u8>,
}

/// `====  Ion Source:  ====` → `Ion Source`.
fn titolo(label: &str) -> String {
    label
        .trim()
        .trim_matches(|c: char| c == '=' || c == ':' || c.is_whitespace())
        .to_string()
}

#[derive(Default)]
struct Statistics {
    n: usize,
    min: f64,
    max: f64,
    first: f64,
    different: bool,
}

impl Statistics {
    fn push(&mut self, y: f64) {
        if self.n == 0 {
            (self.min, self.max, self.first) = (y, y, y);
        } else {
            self.min = self.min.min(y);
            self.max = self.max.max(y);
            self.different |= y != self.first;
        }
        self.n += 1;
    }
}

pub(super) fn number_of(v: &GenericValue) -> Option<f64> {
    if matches!(v, GenericValue::Bool(_)) {
        return None;
    }
    v.as_f64().filter(|x| x.is_finite())
}

fn number(r: &GenericRecord, field: usize) -> Option<f64> {
    number_of(&r.values.get(field)?.1)
}

/// The records of the scan trailer, one after the other: the number of the scan and the numeric value of a field of it. The trailer is read from the file a chunk at a time (`GenericRecordStream::for_each`),
/// so a pass over it holds no more than a chunk; the series ask for two passes (the list, then the points of one series).
pub type Row<'a> = &'a mut dyn FnMut(usize, &dyn Fn(usize) -> Option<f64>);
pub type Rows<'a> = &'a mut dyn FnMut(Row<'_>);

fn spec(
    id: String,
    gruppo: String,
    nome: String,
    unita: String,
    s: &Statistics,
    campo: usize,
    livello: Option<u8>,
) -> Option<Spec> {
    (s.n >= 2 && s.different).then_some(Spec {
        entry: Entry {
            id,
            group: gruppo,
            name: nome,
            unita,
            n: s.n,
            min: s.min,
            max: s.max,
        },
        field: campo,
        level: livello,
    })
}

/// The id with a number after it when `usati` already has it.
fn unico(base: String, used: &mut Vec<String>) -> String {
    let mut id = base.clone();
    let mut k = 1;
    while used.contains(&id) {
        k += 1;
        id = format!("{base} #{k}");
    }
    used.push(id.clone());
    id
}

fn status_specs(log: &StatusLog) -> Vec<Spec> {
    if log.times_min().len() != log.records().len() {
        return Vec::new();
    }
    let (mut out, mut used, mut section) = (Vec::new(), Vec::new(), String::new());
    for (i, d) in log.header.fields.iter().enumerate() {
        if d.field_type == GenericType::Gap {
            section = titolo(&d.label);
            continue;
        }
        let (name, unita) = split_label(&d.label);
        if name.is_empty() {
            continue;
        }
        let mut s = Statistics::default();
        for (r, t) in log.records().iter().zip(log.times_min()) {
            if let (Some(y), true) = (number(r, i), t.is_finite()) {
                s.push(y);
            }
        }
        let base = format!("{STATUS}{section}/{name}");
        if s.n >= 2 && s.different {
            let id = unico(base, &mut used);
            out.extend(spec(id, section.clone(), name, unita, &s, i, None));
        }
    }
    out
}

fn specs_trailer(h: &GenericDataHeader, scans: &[Scansione], rows: Rows) -> Vec<Spec> {
    let levels: Vec<u8> = scans
        .iter()
        .map(|s| s.level)
        .collect::<std::collections::BTreeSet<u8>>()
        .into_iter()
        .collect();
    let mut stat: Vec<Vec<Statistics>> = h
        .fields
        .iter()
        .map(|_| levels.iter().map(|_| Statistics::default()).collect())
        .collect();
    rows(&mut |k, val| {
        let Some(sc) = scans.get(k).filter(|sc| sc.t_min.is_finite()) else {
            return;
        };
        let Some(li) = levels.iter().position(|l| *l == sc.level) else {
            return;
        };
        for (i, row) in stat.iter_mut().enumerate() {
            if let Some(y) = val(i) {
                row[li].push(y);
            }
        }
    });
    let (mut out, mut used) = (Vec::new(), Vec::new());
    for (i, d) in h.fields.iter().enumerate() {
        if d.field_type == GenericType::Gap {
            continue;
        }
        let (name, unita) = split_label(&d.label);
        if name.is_empty() {
            continue;
        }
        let mut per_level: Vec<(u8, Statistics)> = Vec::new();
        for (li, l) in levels.iter().enumerate() {
            let s = std::mem::take(&mut stat[i][li]);
            if s.n >= 2 && s.different {
                per_level.push((*l, s));
            }
        }
        // a field that changes at one level only keeps its plain name; at several levels each one is a series of its own
        let separate = per_level.len() > 1;
        for (l, s) in &per_level {
            let (base, label) = if separate {
                (format!("{TRAILER}{name}@MS{l}"), format!("{name} (MS{l})"))
            } else {
                (format!("{TRAILER}{name}"), name.clone())
            };
            let id = unico(base, &mut used);
            out.extend(spec(
                id,
                String::new(),
                label,
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
pub fn status_index(log: &StatusLog) -> Vec<Entry> {
    status_specs(log).into_iter().map(|s| s.entry).collect()
}

/// The trailer series that vary, in the order of the file.
pub fn trailer_index(h: &GenericDataHeader, scans: &[Scansione], rows: Rows) -> Vec<Entry> {
    specs_trailer(h, scans, rows)
        .into_iter()
        .map(|s| s.entry)
        .collect()
}

/// The points of the status series `id` (`None` when the id is not one of the list).
pub fn status_series(log: &StatusLog, id: &str) -> Option<Series> {
    let s = status_specs(log).into_iter().find(|s| s.entry.id == id)?;
    let (mut t, mut y) = (Vec::new(), Vec::new());
    for (r, tm) in log.records().iter().zip(log.times_min()) {
        if let (Some(v), true) = (number(r, s.field), tm.is_finite()) {
            t.push(f64::from(*tm));
            y.push(v);
        }
    }
    Some(Series {
        unita: s.entry.unita,
        t,
        y,
    })
}

/// The points of the trailer series `id`, one per scan of its level.
pub fn trailer_series(
    h: &GenericDataHeader,
    scans: &[Scansione],
    rows: Rows,
    id: &str,
) -> Option<Series> {
    let s = specs_trailer(h, scans, &mut *rows)
        .into_iter()
        .find(|s| s.entry.id == id)?;
    let (mut t, mut y) = (Vec::new(), Vec::new());
    rows(&mut |k, val| {
        let Some(sc) = scans.get(k) else { return };
        if s.level.is_some_and(|l| l != sc.level) || !sc.t_min.is_finite() {
            return;
        }
        if let Some(v) = val(s.field) {
            t.push(sc.t_min);
            y.push(v);
        }
    });
    Some(Series {
        unita: s.entry.unita,
        t,
        y,
    })
}

/// The trailer of `raw` as rows for `Righe`: read from `source` a chunk at a time.
fn rows_of<'a, R: Read + Seek>(
    raw: &'a RawFileReader,
    source: &'a mut R,
) -> impl FnMut(Row<'_>) + 'a {
    move |f| {
        raw.scan_parameters.for_each(source, &mut |k, rec| {
            f(k, &|i| rec.value(i).as_ref().and_then(number_of));
        });
    }
}

/// The time and the MS level of every scan, from the scan index and the scan events (no scan is read; an SRM run is all level 2).
fn scansioni(raw: &RawFileReader) -> Vec<Scansione> {
    raw.scan_index
        .iter()
        .enumerate()
        .map(|(k, e)| Scansione {
            t_min: e.start_time,
            level: if raw.flat_peaks {
                2
            } else {
                raw.scan_event(k)
                    .and_then(|ev| ev.preamble.ms_power())
                    .map_or(1, |p| if p == MsPower::Undefined { 1 } else { p as u8 })
            },
        })
        .collect()
}

/// Every series of the file: the status log first, then the trailer, whose records `righe` gives (the single pass of `metodo_raw` over it).
pub fn index(raw: &RawFileReader, rows: Rows) -> Vec<Entry> {
    let mut out = raw
        .status_log
        .as_ref()
        .map(status_index)
        .unwrap_or_default();
    out.extend(trailer_index(
        &raw.scan_parameters_header,
        &scansioni(raw),
        rows,
    ));
    out
}

/// The list as the `serie_registro` section of `metodo/1`: no points, the page asks for them one series at a time.
pub fn sezione(entries: &[Entry]) -> Vec<V> {
    entries
        .iter()
        .filter_map(|v| {
            obj(vec![
                ("id", text(&v.id)),
                ("gruppo", text(&v.group)),
                ("nome", text(&v.name)),
                ("unita", text(&v.unita)),
                ("n", num(Some(v.n as f64))),
                ("min", num(Some(v.min))),
                ("max", num(Some(v.max))),
            ])
        })
        .collect()
}

/// The points of the series `id` of the list (`None` for any other id).
pub fn series<R: Read + Seek>(raw: &RawFileReader, source: &mut R, id: &str) -> Option<Series> {
    if id.starts_with(STATUS) {
        status_series(raw.status_log.as_ref()?, id)
    } else if id.starts_with(TRAILER) {
        trailer_series(
            &raw.scan_parameters_header,
            &scansioni(raw),
            &mut rows_of(raw, source),
            id,
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
