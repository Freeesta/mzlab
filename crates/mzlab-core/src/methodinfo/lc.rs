//! LC method extraction (`VendorAppMethod` XML for Shimadzu and Waters Acquity, plus binary `LCPumpStep` streams) for `methodinfo.rs`.

use super::{f32_at, find, round, GPoint, Lc, Pda};
use crate::ole::Ole;
use crate::xmltree::{self, El};

fn num(x: &str) -> Option<f64> {
    x.trim().parse().ok()
}

type Cat = (
    String,
    Vec<(String, (String, String))>,
    Vec<(String, Vec<Vec<String>>)>,
);

fn cats(root: &El) -> Vec<Cat> {
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
                        .chain(t.findall("Row"))
                        .map(|r| {
                            (1..=8)
                                .filter_map(|i| {
                                    r.find(&format!("Field{i}"))
                                        .map(|f| f.text.trim().to_string())
                                })
                                .collect()
                        })
                        .collect();
                    (
                        t.findtext("Name").unwrap_or("").trim().to_string(),
                        rows.into_iter().filter(|r| !r.is_empty()).collect(),
                    )
                })
                .collect();
            (title, par, tabs)
        })
        .collect()
}

/// Extracts a Waters Acquity gradient table (`ACQUITY BSM Method` -> `Gradient Table`) when present in `cats`.
fn acquity_lc(cats: &[Cat]) -> Option<Lc> {
    let cat = |title: &str| cats.iter().rev().find(|c| c.0 == title);
    let par = |title: &str, name: &str| {
        cat(title)
            .and_then(|c| c.1.iter().rev().find(|p| p.0 == name))
            .map(|p| p.1 .0.as_str())
            .unwrap_or("")
    };
    let gt = cat("ACQUITY BSM Method")?
        .2
        .iter()
        .rev()
        .find(|t| t.0 == "Gradient Table")?;
    let mut grad = Vec::new();
    for r in gt.1.iter().skip(1) {
        if r.len() < 4 {
            continue;
        }
        let t = if r[0].eq_ignore_ascii_case("initial") {
            Some(0.0)
        } else {
            num(&r[0])
        };
        let Some(t) = t else { continue };
        grad.push(GPoint {
            t,
            b: num(&r[3]),
            flow: num(&r[1]),
        });
    }
    if grad.is_empty() {
        return None;
    }
    let flow = grad.first().and_then(|g| g.flow);
    let b0 = grad.first().and_then(|g| g.b);
    let run_time = grad.last().map(|g| g.t).filter(|&t| t > 0.0);
    let oven = num(par("ACQUITY SM Method", "Target Column Temperature"));
    Some(Lc {
        gradient: grad,
        flow,
        b0,
        run_time,
        pda: Pda::default(),
        channels: Vec::new(),
        oven,
    })
}

/// Fallback binary LC pump program from `DeviceMethod1/LCPumpStepK/LCPumpStep` (90-byte records).
fn binary_pump_steps(ole: &Ole, base: &str) -> Option<Lc> {
    let mut grad = Vec::new();
    for k in 0..64 {
        let Some(b) = ole.read(&format!("{base}DeviceMethod1/LCPumpStep{k}/LCPumpStep")) else {
            break;
        };
        if b.len() < 74 {
            continue;
        }
        let (Some(t), Some(flow_ul), Some(pct_b)) =
            (f32_at(&b, 54), f32_at(&b, 58), f32_at(&b, 70))
        else {
            continue;
        };
        grad.push(GPoint {
            t: round(t, 3),
            b: Some(round(pct_b, 2)),
            flow: Some(round(flow_ul / 1000.0, 4)),
        });
    }
    if grad.is_empty() {
        return None;
    }
    let flow = grad.first().and_then(|g| g.flow);
    let b0 = grad.first().and_then(|g| g.b);
    let run_time = grad.last().map(|g| g.t).filter(|&t| t > 0.0);
    Some(Lc {
        gradient: grad,
        flow,
        b0,
        run_time,
        pda: Pda::default(),
        channels: Vec::new(),
        oven: None,
    })
}

/// LC method (pump gradient, flow, PDA, oven) from `DeviceMethod1/VendorAppMethod` (Shimadzu or Waters Acquity XML) or binary `LCPumpStep`.
pub(super) fn lc(ole: &Ole, base: &str) -> Option<Lc> {
    let Some(raw) = ole.read(&format!("{base}DeviceMethod1/VendorAppMethod")) else {
        return binary_pump_steps(ole, base);
    };
    let a = find(&raw, 0, raw.len(), b"<SCIEX_ANALYST_ACQUISITION_METHOD")?;
    let end = b"</SCIEX_ANALYST_ACQUISITION_METHOD>";
    let b = find(&raw, 0, raw.len(), end)?;
    let root = xmltree::parse(&String::from_utf8_lossy(raw.get(a..b + end.len())?))?;
    let cats = cats(&root);
    acquity_lc(&cats).or_else(|| shimadzu_lc(&cats))
}

fn shimadzu_lc(cats: &[Cat]) -> Option<Lc> {
    let cat = |title: &str| cats.iter().rev().find(|c| c.0 == title);
    let par = |title: &str, name: &str| {
        cat(title)
            .and_then(|c| c.1.iter().rev().find(|p| p.0 == name))
            .map(|p| p.1 .0.as_str())
            .unwrap_or("")
    };
    let (flow0, b0) = (
        num(par("Pumps", "Total Flow")),
        num(par("Pumps", "B Concentration")),
    );
    let tp = cat("System Controller")
        .and_then(|c| c.2.iter().rev().find(|t| t.0 == "Time Program"))
        .map(|t| t.1.iter().skip(1).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let mut events: Vec<(f64, String, Option<f64>)> = tp
        .iter()
        .filter(|r| r.len() >= 5)
        .filter_map(|r| num(&r[0]).map(|t| (t, r[3].clone(), num(&r[4]))))
        .collect();
    crate::sortc::any_by(&mut events, |a, b| a.0.total_cmp(&b.0));
    let (mut grad, mut flow, mut bconc, mut stop) = (Vec::new(), flow0, b0, None);
    if b0.is_some() {
        grad.push(GPoint {
            t: 0.0,
            b: b0,
            flow,
        });
    }
    for (t, name, val) in events {
        match (name.as_str(), val) {
            ("Pump B Conc." | "B.Conc", Some(v)) => bconc = Some(v),
            ("Total Flow" | "T.Flow", Some(v)) => flow = Some(v),
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
        .map(|t| {
            t.1.iter()
                .skip(1)
                .filter(|r| r.len() >= 2)
                .filter_map(|r| num(&r[0]).map(|w| (w, num(&r[1]))))
                .collect()
        })
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
