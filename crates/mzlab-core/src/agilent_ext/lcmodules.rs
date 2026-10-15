//! The LC modules of the `metodo/1` of an LC/MS run (pump gradient, column compartment, sampler, detectors), from `lcmethod::LcMethod`.

use super::doc::param;
use super::lcdoc::*;
use super::lcmethod::{LcMethod, Rc};
use crate::metodo::{num, obj, text, V};

fn gradient(m: &LcMethod, pump: &str) -> Vec<V> {
    let rows: Vec<&Rc> =
        m.rc.iter()
            .filter(|r| r.kind == pump && r.table.as_deref() == Some("Timetable"))
            .collect();
    let last = rows.iter().filter_map(|r| r.row).max();
    let cell = |row: usize, name: &str| {
        rows.iter()
            .find(|r| r.row == Some(row) && r.name == name)
            .and_then(|r| r.value.as_deref())
    };
    // (time, B %, flow)
    let mut points: Vec<(f64, Option<f64>, Option<f64>)> = Vec::new();
    for row in 0..=last.unwrap_or(0).min(100_000) {
        let Some(t) = cell(row, "Time").and_then(whole) else {
            continue;
        };
        let (mut b, mut flow) = (
            cell(row, "B").and_then(whole),
            cell(row, "Flow").and_then(whole),
        );
        if let (Some(f), Some(p)) = (cell(row, "Function"), cell(row, "Parameter")) {
            if f.contains("Solvent Composition") {
                b = p.split_once("B:").and_then(|(_, v)| lead(v));
            } else if f == "Change Flow" {
                flow = p.split_once(':').and_then(|(_, v)| lead(v));
            }
        }
        if b.is_none() && flow.is_none() {
            continue;
        }
        match points.last_mut() {
            Some(l) if l.0 == t => {
                l.1 = b.or(l.1);
                l.2 = flow.or(l.2);
            }
            _ => points.push((t, b, flow)),
        }
    }
    points
        .into_iter()
        .filter_map(|(t, b, f)| {
            obj(vec![
                ("t_min", Some(V::Num(t))),
                ("B_pct", num(b)),
                ("flusso_mL_min", num(f)),
            ])
        })
        .collect()
}

/// The percentage of channel B before the first timetable row (the `Solvent Composition` table).
fn initial_b(m: &LcMethod, pump: &str) -> Option<f64> {
    let rows: Vec<&Rc> =
        m.rc.iter()
            .filter(|r| r.kind == pump && r.table.as_deref() == Some("Solvent Composition"))
            .collect();
    let row = rows
        .iter()
        .find(|r| r.name == "Channel" && r.value.as_deref() == Some("B"))?
        .row?;
    rows.iter()
        .find(|r| r.row == Some(row) && r.name == "Percent")
        .and_then(|r| r.value.as_deref())
        .and_then(whole)
}

/// The pump that drives the run: the one with a timetable, else the one with a flow, else the first (`Binary Pump`, `Quat. Pump`, `Iso. Pump`…).
fn main_pump(m: &LcMethod) -> Option<&str> {
    let pumps: Vec<&str> =
        m.rc.iter()
            .map(|r| r.kind.as_str())
            .filter(|k| k.contains("Pump"))
            .collect();
    let has = |k: &&str, f: &dyn Fn(&Rc) -> bool| m.rc.iter().any(|r| r.kind == *k && f(r));
    pumps
        .iter()
        .find(|k| has(k, &|r| r.table.as_deref() == Some("Timetable")))
        .or_else(|| {
            pumps.iter().find(|k| {
                has(k, &|r| {
                    r.name == "Flow"
                        && r.table.is_none()
                        && r.value.as_deref().and_then(whole).is_some_and(|v| v > 0.0)
                })
            })
        })
        .or(pumps.first())
        .copied()
}

pub(super) fn lc(m: &LcMethod) -> Option<V> {
    let pump = main_pump(m).unwrap_or("Binary Pump");
    let value = |kind: &str, name: &str| m.rc_param(kind, name);
    let oven =
        m.rc.iter()
            .find(|r| {
                r.kind == "Column Comp."
                    && (r.name == "Temperature" || r.name == "Right temperature")
                    && r.value.as_deref().and_then(whole).is_some()
            })
            .and_then(|r| r.value.as_deref())
            .and_then(whole);
    let pressure = value(pump, "High Pressure Limit");
    obj(vec![
        ("gradiente", Some(V::Arr(gradient(m, pump)))),
        (
            "flusso_mL_min",
            num(value(pump, "Flow").and_then(|r| r.value.as_deref().and_then(whole))),
        ),
        ("B_iniziale_pct", num(initial_b(m, pump))),
        ("forno_C", num(oven)),
        (
            "durata_min",
            num(m
                .rc
                .iter()
                .find(|r| r.kind == pump && r.name == "Stoptime")
                .and_then(|r| r.value.as_deref())
                .and_then(whole)),
        ),
        (
            "pressione_max",
            pressure.and_then(|r| {
                obj(vec![
                    ("valore", num(r.value.as_deref().and_then(whole))),
                    ("unita", r.unit.as_deref().and_then(text)),
                ])
            }),
        ),
    ])
}

pub(super) fn sampler(m: &LcMethod) -> Option<V> {
    let params: Vec<V> =
        m.rc.iter()
            .filter(|r| r.kind.contains("Sampler") || r.kind.contains("ALS"))
            .filter(|r| r.table.is_none())
            .filter_map(|r| {
                let v = r.value.as_deref().and_then(whole)?;
                param(&r.name, &r.section, v, r.unit.as_deref().unwrap_or(""))
            })
            .collect();
    obj(vec![("parametri", Some(V::Arr(params)))])
}

/// The diode-array detector: the signals in use (wavelength, bandwidth, reference), the spectrum range and step, the data rate, the slit and the lamps.
pub(super) fn detectors(m: &LcMethod) -> Vec<V> {
    let dad: Vec<&Rc> = m.rc.iter().filter(|r| r.kind == "DAD").collect();
    if dad.is_empty() {
        return vec![];
    }
    let own = |name: &str| {
        dad.iter()
            .find(|r| r.table.is_none() && r.name == name)
            .and_then(|r| r.value.as_deref())
    };
    let signal = |row: usize, name: &str| {
        dad.iter()
            .find(|r| {
                r.table.as_deref() == Some("Signal table") && r.row == Some(row) && r.name == name
            })
            .and_then(|r| r.value.as_deref())
    };
    let last = dad
        .iter()
        .filter(|r| r.table.as_deref() == Some("Signal table"))
        .filter_map(|r| r.row)
        .max();
    let channels: Vec<V> = (0..=last.unwrap_or(0).min(64))
        .filter(|&k| last.is_some() && signal(k, "Use Sig.") == Some("Yes"))
        .filter_map(|k| {
            let reference = signal(k, "Use Ref.") == Some("Yes");
            obj(vec![
                ("nm", num(signal(k, "Wavelength").and_then(whole))),
                ("banda_nm", num(signal(k, "Bandwidth").and_then(whole))),
                (
                    "riferimento_nm",
                    num(signal(k, "Ref Wavel.").and_then(whole)).filter(|_| reference),
                ),
                (
                    "banda_riferimento_nm",
                    num(signal(k, "Ref Bandw.").and_then(whole)).filter(|_| reference),
                ),
                ("nome", signal(k, "Signal").and_then(text)),
            ])
        })
        .collect();
    let from = own("Spectrum Range WL from").and_then(whole);
    let to = own("Spectrum Range WL to").and_then(whole);
    // `>0.05 min (1.0 s response time) (5 Hz)`: the data rate is the last number before «Hz»
    let hz = own("Peakwidth").and_then(|p| {
        let head = p.rsplit_once("Hz")?.0.trim_end();
        whole(head.rsplit_once('(')?.1)
    });
    let lamps: Vec<&str> = dad
        .iter()
        .filter(|r| r.name.ends_with("Lamp Required") && r.value.as_deref() == Some("Yes"))
        .filter_map(|r| r.name.strip_suffix(" Lamp Required"))
        .collect();
    obj(vec![
        ("tipo", Some(V::s("DAD"))),
        (
            "nm",
            from.zip(to)
                .map(|(a, b)| V::Arr(vec![V::Num(a), V::Num(b)])),
        ),
        ("passo_nm", num(own("Spectrum Step").and_then(whole))),
        ("frequenza_Hz", num(hz)),
        ("lampada", text(&lamps.join("+"))),
        ("fenditura_nm", num(own("Slit").and_then(lead))),
        ("canali", Some(V::Arr(channels))),
    ])
    .into_iter()
    .collect()
}
