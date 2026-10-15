//! The LC/MS part of the `metodo/1` document of an Agilent `.D` (feature `estesi`): the sections that `AcqData/AcqMethod.xml` (`lcmethod.rs`) fills for a QqQ or a
//! Q-TOF run: `ms` (source, one experiment per time segment with its MRM transitions, dMRM windows, Q-TOF precursor lists), `lc` (flow, oven, gradient),
//! `campionatore`, `rivelatori` (DAD) and the modules of `Devices.xml`, and the fields of the method as `campi` items. Only what the method says: a cell that is not
//! a number stays a text in `campi`, an empty cell is absent.

use super::doc::param;
use super::lcmethod::{LcMethod, MsTable, Rc};
use super::{meta, AgilentD};
use crate::metodo::{num, obj, text, V};

pub(super) const RC_ORIGIN: &str = "AcqData/AcqMethod.xml#RC";
pub(super) const MS_ORIGIN: &str = "AcqData/AcqMethod.xml#MS";
const SAMPLE_INFO: &str = "AcqData/sample_info.xml";

/// A whole text that is a number.
fn whole(s: &str) -> Option<f64> {
    s.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

/// The number a text starts with (`4 nm`, `Flow: 0.5 mL/min` is not one).
fn lead(s: &str) -> Option<f64> {
    whole(s.split_whitespace().next()?)
}

fn unit_of(name: &str) -> (&str, &str) {
    match name.strip_suffix(')').and_then(|n| n.rsplit_once(" (")) {
        Some((label, unit)) => (label, unit),
        None => (name, ""),
    }
}

fn rc_value(r: &Rc) -> Option<String> {
    let v = r.value.as_deref()?;
    Some(match r.unit.as_deref() {
        Some(u) => format!("{v} {u}"),
        None => v.to_string(),
    })
}

fn rc_name(r: &Rc) -> String {
    match (&r.table, r.row) {
        (Some(t), Some(row)) => format!("{}/{t}[{row:04}]/{}", r.module, r.name),
        _ if !r.section.is_empty() && r.section != r.name => {
            format!("{}/{}/{}", r.module, r.section, r.name)
        }
        _ => format!("{}/{}", r.module, r.name),
    }
}

fn seg_name(seg: u32) -> String {
    if seg == 0 {
        "global".to_string()
    } else {
        format!("segment{seg}")
    }
}

/// The columns of the transitions table that `transizioni` carries: their cells are not repeated in `campi` (a dMRM method has thousands of rows).
const TRANSITION_COLUMNS: [&str; 14] = [
    "Prec Ion",
    "Prod Ion",
    "Dwell",
    "CE (V)",
    "Cpd Name",
    "Cpd Group",
    "ISTD?",
    "Frag (V)",
    "Cell Acc (V)",
    "Ret Time (min)",
    "Ret Window",
    "Polarity",
    "MS1 Res",
    "MS2 Res",
];

/// Every parameter and cell of the method as a metadata item, so that `campi` lists them (the row of a table in the name: `Timetable[0003]/Flow`); the cells of
/// the transitions are in `ms.periodi[].esperimenti[].transizioni` only.
pub(super) fn items(m: &LcMethod) -> Vec<meta::Item> {
    let mut out = Vec::new();
    for r in &m.rc {
        if let Some(value) = rc_value(r) {
            out.push(meta::Item {
                origin: RC_ORIGIN.into(),
                name: rc_name(r),
                value,
            });
        }
    }
    for p in &m.ms {
        if let Some(v) = &p.value {
            out.push(meta::Item {
                origin: MS_ORIGIN.into(),
                name: format!("{}/{}/{}", seg_name(p.seg), p.section, p.name),
                value: v.clone(),
            });
        }
    }
    for t in &m.tables {
        let mrm = t.column("Prec Ion").is_some() && t.column("Prod Ion").is_some();
        for (k, row) in t.rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if mrm && TRANSITION_COLUMNS.contains(&t.columns[c].as_str()) {
                    continue;
                }
                if let Some(v) = cell {
                    out.push(meta::Item {
                        origin: MS_ORIGIN.into(),
                        name: format!("{}/{}[{k:04}]/{}", seg_name(t.seg), t.name, t.columns[c]),
                        value: v.clone(),
                    });
                }
            }
        }
    }
    out
}

/// The section that carries a field of the method (`mappato`).
pub(super) fn mapped(origin: &str, name: &str, value: &str) -> Option<&'static str> {
    let in_rc = |p: &str| name.starts_with(p);
    if origin == RC_ORIGIN {
        return if in_rc("Binary Pump/Timetable[") {
            Some("lc.gradiente")
        } else if name == "Binary Pump/Flow" {
            Some("lc.flusso_mL_min")
        } else if in_rc("Binary Pump/") && name.ends_with("/Stoptime") {
            Some("lc.durata_min")
        } else if name == "Binary Pump/High Pressure Limit" {
            Some("lc.pressione_max")
        } else if in_rc("Column Comp.")
            && (name.ends_with("/Temperature") || name.ends_with("/Right temperature"))
        {
            Some("lc.forno_C")
        } else if in_rc("DAD/")
            && (name.contains("Signal table[")
                || name.contains("Spectrum Range WL")
                || name.ends_with("/Spectrum Step")
                || name.ends_with("/Peakwidth")
                || name.ends_with("/Slit")
                || name.ends_with("Lamp Required"))
        {
            Some("rivelatori")
        } else if (in_rc("HiP") || name.contains("Sampler") || name.contains("ALS"))
            && !name.contains('[')
            && whole(value).is_some()
        {
            Some("campionatore.parametri")
        } else {
            None
        };
    }
    if origin != MS_ORIGIN {
        return None;
    }
    if name.contains("/Scan Segments[") && (name.ends_with("/Prec Ion") || name.contains("/Prod")) {
        Some("ms.periodi.esperimenti.transizioni")
    } else if name.contains("/Scan Segments[") && name.ends_with("/Collision Energy") {
        Some("ms.periodi.esperimenti.ce_eV")
    } else if name.contains("/Auto MS/MS Preferred/Exclude Table[") {
        Some("ms.periodi.esperimenti.liste_precursori")
    } else if name.contains("/Reference Masses[")
        || (name.contains("Parameters[")
            && name
                .rsplit('/')
                .next()
                .is_some_and(|c| c.starts_with("Value"))
            && whole(value).is_some())
    {
        Some("ms.sorgente.parametri")
    } else if name.matches('/').count() == 2 && whole(value).is_some() && !name.contains('[') {
        Some("ms.periodi.esperimenti.parametri")
    } else {
        None
    }
}

fn cell_num(t: &MsTable, row: usize, col: &str) -> Option<V> {
    num(t.cell(row, col).and_then(whole))
}

fn cell_text(t: &MsTable, row: usize, col: &str) -> Option<V> {
    t.cell(row, col).and_then(text)
}

/// The transitions of a `Scan Segments` table of a QqQ method: precursor, product, and the settings of the row.
fn transitions(t: &MsTable) -> Vec<V> {
    if t.column("Prec Ion").is_none() || t.column("Prod Ion").is_none() {
        return vec![];
    }
    (0..t.rows.len())
        .filter_map(|k| {
            obj(vec![
                ("q1", cell_num(t, k, "Prec Ion")),
                ("q3", cell_num(t, k, "Prod Ion")),
                ("dwell_ms", cell_num(t, k, "Dwell")),
                ("ce_eV", cell_num(t, k, "CE (V)")),
                ("nome", cell_text(t, k, "Cpd Name")),
                ("gruppo", cell_text(t, k, "Cpd Group")),
                ("frammentatore_V", cell_num(t, k, "Frag (V)")),
                ("cell_acc_V", cell_num(t, k, "Cell Acc (V)")),
                ("rt_min", cell_num(t, k, "Ret Time (min)")),
                ("finestra_rt_min", cell_num(t, k, "Ret Window")),
                ("polarita", cell_text(t, k, "Polarity")),
                ("istd", cell_text(t, k, "ISTD?")),
                ("risoluzione_q1", cell_text(t, k, "MS1 Res")),
                ("risoluzione_q3", cell_text(t, k, "MS2 Res")),
            ])
        })
        .collect()
}

/// The inclusion/exclusion list of a Q-TOF method (`Auto MS/MS Preferred/Exclude Table`).
fn precursor_list(t: &MsTable) -> Vec<V> {
    (0..t.rows.len())
        .filter_map(|k| {
            obj(vec![
                ("mz", cell_num(t, k, "Mass")),
                ("carica", cell_num(t, k, "Charge")),
                ("tipo", cell_text(t, k, "Type")),
                ("rt_min", cell_num(t, k, "Retention Time (min)")),
                ("finestra_rt_min", cell_num(t, k, "Delta Ret. Time (min)")),
                ("delta_ppm", cell_num(t, k, "Delta Mass (ppm)")),
                ("isolamento", cell_text(t, k, "Isolation Width")),
                ("ce_eV", cell_num(t, k, "Collision Energy")),
            ])
        })
        .collect()
}

/// The parameters of a segment that are numbers; the m/z range goes to `mz`.
fn segment_params(m: &LcMethod, seg: u32, first: bool) -> (Vec<V>, Option<(f64, f64)>) {
    let mut out = Vec::new();
    let (mut lo, mut hi) = (None, None);
    for p in
        m.ms.iter()
            .filter(|p| p.seg == seg || (first && p.seg == 0))
    {
        let Some(v) = p.value.as_deref().and_then(whole) else {
            continue;
        };
        match p.name.as_str() {
            "Min Range (m/z)" => lo = Some(v),
            "Max Range (m/z)" => hi = Some(v),
            n => {
                let (label, unit) = unit_of(n);
                out.extend(param(n, label, v, unit));
            }
        }
    }
    (out, lo.zip(hi))
}

fn source(m: &LcMethod) -> Option<V> {
    let mut params: Vec<V> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for t in &m.tables {
        let is_ref = t.columns.first().is_some_and(|c| c == "Reference Masses");
        let is_par = (t.name.ends_with("Source Parameters") || t.name == "Instrument Parameters")
            && t.column("Parameter").is_some();
        if is_ref {
            for (k, row) in t.rows.iter().enumerate() {
                if let Some(v) = row.first().and_then(|c| c.as_deref()).and_then(whole) {
                    params.extend(param(
                        &format!("ref_mass_{}", k + 1),
                        "Reference mass",
                        v,
                        "m/z",
                    ));
                }
            }
        } else if is_par {
            for k in 0..t.rows.len() {
                let Some(name) = t.cell(k, "Parameter") else {
                    continue;
                };
                for col in t.columns.iter().filter(|c| c.starts_with("Value")) {
                    let id = match col.strip_prefix("Value") {
                        Some(s) if !s.trim().is_empty() => format!("{name} {}", s.trim()),
                        _ => name.to_string(),
                    };
                    if seen.contains(&id) {
                        continue;
                    }
                    if let Some(v) = t.cell(k, col).and_then(whole) {
                        let (label, unit) = unit_of(name);
                        params.extend(param(&id, label, v, unit));
                        seen.push(id);
                    }
                }
            }
        }
    }
    obj(vec![
        ("tipo", m.param("Ion Source").and_then(text)),
        ("parametri", Some(V::Arr(params))),
    ])
}

/// The `ms` section: the source and one experiment per time segment.
pub(super) fn ms(d: &AgilentD) -> Option<V> {
    let m = d.lcm.as_ref()?;
    let segments = m.segments();
    let mut experiments = Vec::new();
    for (n, &seg) in segments.iter().enumerate() {
        let list = m.table("Scan Segments", Some(seg));
        let trans = list.map(transitions).unwrap_or_default();
        let (params, range) = segment_params(m, seg, n == 0);
        let ce: Vec<V> = list
            .filter(|_| trans.is_empty())
            .map(|t| {
                (0..t.rows.len())
                    .filter_map(|k| cell_num(t, k, "Collision Energy"))
                    .collect()
            })
            .unwrap_or_default();
        let mode = m
            .modes
            .iter()
            .find(|(s, _)| *s == seg)
            .map(|(_, mode)| mode.as_str());
        let lists = m
            .table("Auto MS/MS Preferred/Exclude Table", Some(seg))
            .map(precursor_list)
            .unwrap_or_default();
        experiments.extend(obj(vec![
            ("indice", Some(V::Num(seg as f64))),
            (
                "tipo",
                (!trans.is_empty())
                    .then(|| V::s("MRM"))
                    .or(mode.and_then(text)),
            ),
            ("mz", range.map(|(a, b)| V::Arr(vec![V::Num(a), V::Num(b)]))),
            ("ce_eV", Some(V::Arr(ce))),
            ("parametri", Some(V::Arr(params))),
            ("transizioni", Some(V::Arr(trans))),
            ("liste_precursori", Some(V::Arr(lists))),
        ]));
    }
    obj(vec![
        ("sorgente", source(m)),
        (
            "periodi",
            obj(vec![("esperimenti", Some(V::Arr(experiments)))]).map(|p| V::Arr(vec![p])),
        ),
    ])
}

/// The gradient of the binary pump: a table of time, A, B, flow rows, or a timetable of functions (`Change Solvent Composition`, `Change Flow`).
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

fn lc(m: &LcMethod) -> Option<V> {
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

fn sampler(m: &LcMethod) -> Option<V> {
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
fn detectors(m: &LcMethod) -> Vec<V> {
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

/// `lc`, `campionatore` and `rivelatori` of the method, in this order.
pub(super) fn sections(d: &AgilentD) -> Vec<(&'static str, Option<V>)> {
    let Some(m) = &d.lcm else { return vec![] };
    vec![
        ("lc", lc(m)),
        ("campionatore", sampler(m)),
        ("rivelatori", Some(V::Arr(detectors(m)))),
    ]
}

/// The other modules of `Devices.xml` besides the mass spectrometer (the first device): name, model, serial number, firmware.
pub(super) fn modules(d: &AgilentD) -> Option<V> {
    let list: Vec<V> = d
        .devices_all
        .iter()
        .skip(1)
        .filter_map(|v| {
            obj(vec![
                ("nome", text(&v.name)),
                ("modello", text(&v.model)),
                ("serie", text(&v.serial)),
                ("firmware", text(&v.firmware)),
            ])
        })
        .collect();
    (!list.is_empty()).then_some(V::Arr(list))
}

/// The sections of an LC/MS run over the base ones: the sample from `sample_info.xml` when the folder has no sequence, the other modules in `strumento`, and
/// the method's own `ms` where the GC/MS one is empty.
pub(super) fn extend(
    d: &AgilentD,
    sample: Option<V>,
    instrument: Option<V>,
    acquisition: Option<V>,
) -> (Option<V>, Option<V>, Option<V>) {
    let instrument = match (instrument, modules(d)) {
        (Some(V::Obj(mut f)), Some(m)) => {
            f.push(("moduli", m));
            Some(V::Obj(f))
        }
        (i, _) => i,
    };
    (
        sample.or_else(|| self::sample(d)),
        instrument,
        ms(d).or(acquisition),
    )
}

/// The sample from `sample_info.xml` when the folder has no `sequence.xml` (the case of a run started from the instrument).
pub(super) fn sample(d: &AgilentD) -> Option<V> {
    let f = |name: &str| d.field(SAMPLE_INFO, name).and_then(text);
    let name_of = |p: &str| p.rsplit(['\\', '/']).next().unwrap_or(p).to_string();
    obj(vec![
        ("nome", f("Sample Name")),
        ("vial", f("Sample Position")),
        (
            "iniezione_uL",
            num(d.field(SAMPLE_INFO, "Inj Vol (µl)").and_then(whole)),
        ),
        (
            "metodo_file",
            d.field(SAMPLE_INFO, "Method")
                .and_then(|p| text(&name_of(p))),
        ),
        (
            "diluizione",
            num(d.field(SAMPLE_INFO, "Dilution").and_then(whole)),
        ),
        ("tipo", f("Sample Type")),
    ])
}

impl AgilentD {
    /// The LC/MS files of the folder: the method report (`AcqMethod.xml`, parsed into `lcm` and listed in `campi`), `Devices.xml` for every module, the other
    /// small files (`extras`: text and XML are flattened, binaries are kept as size and SHA-1) and the members read by size only, and the member that holds the
    /// points. A method report that cannot be read is kept as undecoded bytes.
    pub fn with_lc_files(
        mut self,
        method: Option<&[u8]>,
        devices: Option<&[u8]>,
        extras: &[(&str, &[u8])],
        sizes: &[(&str, u64)],
        data_file: &'static str,
    ) -> Self {
        use sha1::{Digest as _, Sha1};
        let hex =
            |b: &[u8]| -> String { Sha1::digest(b).iter().map(|x| format!("{x:02x}")).collect() };
        self.data_file = data_file;
        self.devices_all = devices.map(super::info::devices).unwrap_or_default();
        let mut listed: Vec<(&str, &[u8])> = extras.to_vec();
        if let Some(b) = method {
            let text = super::method::decode(b);
            match LcMethod::parse(&text) {
                Some(m) => {
                    self.metadata.extend(items(&m));
                    self.lcm = Some(m);
                }
                None => {
                    // an older plain-XML method: every element is in `campi`, none is placed in a section
                    self.metadata.push(meta::Item {
                        origin: "AcqData/AcqMethod.xml".into(),
                        name: "undecoded bytes".into(),
                        value: b.len().to_string(),
                    });
                    listed.push(("AcqData/AcqMethod.xml", b));
                }
            }
            self.file_sha1
                .push(("AcqData/AcqMethod.xml".into(), hex(b)));
        }
        listed.retain(|(n, b)| !n.ends_with(".xml") || super::shallow(b));
        self.metadata.extend(meta::collect(&listed));
        self.file_sha1
            .extend(extras.iter().map(|(n, b)| (n.to_string(), hex(b))));
        for (name, size) in sizes {
            self.metadata.push(meta::Item {
                origin: name.to_string(),
                name: "undecoded bytes".into(),
                value: size.to_string(),
            });
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn method() -> LcMethod {
        let esc = |s: &str| {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        };
        let row = |module: &str,
                   kind: &str,
                   table: Option<(&str, usize)>,
                   name: &str,
                   v: &str,
                   unit: &str| {
            let t = table
                .map(|(t, r)| format!("<TableName>{t}</TableName><RowIndex>{r}</RowIndex>"))
                .unwrap_or_default();
            format!("<Section><ModuleDisplayName>{module}</ModuleDisplayName><DeviceType>{kind}</DeviceType>{t}<ParameterName>{name}</ParameterName><ParameterValue>{v}</ParameterValue><ParameterUnit>{unit}</ParameterUnit></Section>")
        };
        let mut rc = String::new();
        for (r, (t, b, f)) in [("0.00", "5", "0.4"), ("10.00", "95", "0.4")]
            .iter()
            .enumerate()
        {
            let tt = Some(("Timetable", r));
            rc += &row("Binary Pump", "Binary Pump", tt, "Time", t, "min");
            rc += &row("Binary Pump", "Binary Pump", tt, "B", b, "%");
            rc += &row("Binary Pump", "Binary Pump", tt, "Flow", f, "mL/min");
        }
        rc += &row(
            "Binary Pump",
            "Binary Pump",
            None,
            "Flow",
            "0.400",
            "mL/min",
        );
        rc += &row(
            "Column Comp.",
            "Column Comp.",
            None,
            "Temperature",
            "40.00",
            "°C",
        );
        rc += &row(
            "DAD",
            "DAD",
            None,
            "Peakwidth",
            ">0.05 min (1.0 s response time) (5 Hz)",
            "",
        );
        rc += &row("DAD", "DAD", None, "UV Lamp Required", "Yes", "");
        rc += &row(
            "DAD",
            "DAD",
            Some(("Signal table", 0)),
            "Use Sig.",
            "Yes",
            "",
        );
        rc += &row(
            "DAD",
            "DAD",
            Some(("Signal table", 0)),
            "Wavelength",
            "254.0",
            "nm",
        );
        rc += &row(
            "DAD",
            "DAD",
            Some(("Signal table", 0)),
            "Use Ref.",
            "Yes",
            "",
        );
        rc += &row(
            "DAD",
            "DAD",
            Some(("Signal table", 0)),
            "Ref Wavel.",
            "360.0",
            "nm",
        );
        let cell = |r: usize, c: usize, col: &str, v: &str| {
            format!("<SectionInfo><SectionType>Column</SectionType><Name>{col}</Name><RowIndex>{r}</RowIndex><ColumnIndex>{c}</ColumnIndex><Value>{v}</Value></SectionInfo>")
        };
        let mut ms = String::from("<SectionInfo><SectionType>Repeater</SectionType><Name>Time Segment 1</Name></SectionInfo><SectionInfo><SectionType>Table</SectionType><Name>Scan Segments</Name></SectionInfo>");
        for (r, (q1, q3)) in [("399.1", "250.2"), ("399.1", "180.0")].iter().enumerate() {
            ms += &cell(r, 0, "Prec Ion", q1);
            ms += &cell(r, 1, "Prod Ion", q3);
            ms += &cell(r, 2, "Dwell", "50");
            ms += &cell(r, 3, "CE (V)", "20");
            ms += &cell(r, 4, "Ret Time (min)", "2.5");
        }
        let text = format!(
            "<R><RCDevicesXml>{}</RCDevicesXml><SCICDevicesXml>{}</SCICDevicesXml></R>",
            esc(&format!("<D>{rc}</D>")),
            esc(&format!("<N>{ms}</N>"))
        );
        LcMethod::parse(&text).unwrap()
    }

    #[test]
    fn gradient_oven_and_detector_come_from_the_rows() {
        let m = method();
        let doc = lc(&m).unwrap().json();
        assert!(doc.contains("\"forno_C\":40"), "{doc}");
        assert!(doc.contains("\"flusso_mL_min\":0.4"), "{doc}");
        assert!(
            doc.contains("{\"t_min\":10,\"B_pct\":95,\"flusso_mL_min\":0.4}"),
            "{doc}"
        );
        let det = V::Arr(detectors(&m)).json();
        assert!(det.contains("\"frequenza_Hz\":5"), "{det}");
        assert!(det.contains("\"lampada\":\"UV\""), "{det}");
        assert!(det.contains("\"riferimento_nm\":360"), "{det}");
    }

    #[test]
    fn the_scan_segments_become_transitions() {
        let m = method();
        let t = transitions(m.table("Scan Segments", Some(1)).unwrap());
        assert_eq!(t.len(), 2);
        let j = V::Arr(t).json();
        assert!(j.contains("\"q1\":399.1,\"q3\":180"), "{j}");
        assert!(j.contains("\"rt_min\":2.5"), "{j}");
        // the cells are carried by the transitions: not repeated in campi
        assert!(items(&m).iter().all(|i| !i.name.contains("Scan Segments[")));
    }
}
