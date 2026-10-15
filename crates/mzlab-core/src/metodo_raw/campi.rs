//! `metodo/1.campi` of a Thermo `.raw`: every field the file holds that the sections of the document do not carry, as `{origine, chiave, valore, unita?, mappato?, identita?}`
//! (spec: `docs/agenti/metodo.md`). Fixed places of the container (header, sequence row, InstID, run header), the 62 fields of the scan trailer
//! (the value when it is the same in every scan, else the range), the lines of the LC script and the items of the XML method.

use crate::agilent::meta::Item;
use crate::metodo::V;
pub use crate::metodo_campi::{Campo, Ident};
use opentfraw::generic_data::GenericValue;
use opentfraw::RawFileReader;

fn shown(v: &GenericValue) -> String {
    match v {
        GenericValue::Gap => String::new(),
        GenericValue::Int8(x) => x.to_string(),
        GenericValue::Bool(x) => x.to_string(),
        GenericValue::UInt8(x) => x.to_string(),
        GenericValue::Int16(x) => x.to_string(),
        GenericValue::UInt16(x) => x.to_string(),
        GenericValue::Int32(x) => x.to_string(),
        GenericValue::UInt32(x) => x.to_string(),
        GenericValue::Float32(x) => x.to_string(),
        GenericValue::Float64(x) => x.to_string(),
        GenericValue::String(x) => x.trim().to_string(),
    }
}

/// The 62 fields of the scan trailer: the value when it is the same in every scan, else `min..max` (numbers) or the number of distinct values.
fn trailer(raw: &RawFileReader) -> Vec<Campo> {
    const FROM: &str = "thermo-raw:ScanParameters";
    let mut out = Vec::new();
    for (i, d) in raw.scan_parameters_header.fields.iter().enumerate() {
        let label = d.label.trim().trim_end_matches(':').trim();
        let vals: Vec<&GenericValue> = raw
            .scan_parameters
            .iter()
            .filter_map(|r| r.values.get(i).map(|v| &v.1))
            .collect();
        let Some(first) = vals.first() else { continue };
        let texts: Vec<String> = vals.iter().map(|v| shown(v)).collect();
        if label.is_empty() || texts.iter().all(String::is_empty) {
            continue;
        }
        let same = texts.iter().all(|t| Some(t) == texts.first());
        let nums: Vec<f64> = vals.iter().filter_map(|v| v.as_f64()).collect();
        let (value, key) = if same {
            (shown(first), label.to_string())
        } else if nums.len() == vals.len() {
            let lo = nums.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = nums.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (format!("{lo}..{hi}"), format!("{label} (min..max)"))
        } else {
            let mut u = texts.clone();
            crate::sortc::slice_ord(&mut u);
            u.dedup();
            (u.len().to_string(), format!("{label} (distinct values)"))
        };
        out.push(Campo::new(FROM, &key, value));
    }
    out
}

/// Seconds since the epoch of a clock reading without zone → `YYYY-MM-DDTHH:MM:SS`.
pub fn clock(secs: f64) -> Option<String> {
    if !secs.is_finite() || secs < 0.0 {
        return None;
    }
    let s = secs.floor() as i64;
    let (days, rem) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    ))
}

/// The fixed places of the container.
fn container(raw: &RawFileReader) -> Vec<Campo> {
    let (h, s, i) = (&raw.header, &raw.seq_row, &raw.seq_row.injection);
    let mut v = vec![Campo::new("thermo-raw:FileHeader", "Version", h.version)];
    for (tag, a) in [("AuditStart", &h.audit_start), ("AuditEnd", &h.audit_end)] {
        let from = format!("thermo-raw:FileHeader/{tag}");
        let when = clock(a.time).unwrap_or_default();
        v.push(if tag == "AuditStart" {
            Campo::new(&from, "time", when).map("origine.file_creato")
        } else {
            Campo::new(&from, "time", when)
        });
        v.push(Campo::new(&from, "tag1", &a.tag1).id());
        v.push(if tag == "AuditStart" {
            Campo::new(&from, "tag2", &a.tag2)
                .map("operatore.utente")
                .id()
        } else {
            Campo::new(&from, "tag2", &a.tag2).id()
        });
    }
    let q = "thermo-raw:SeqRow";
    v.push(Campo::new(q, "RowNumber", i.row_number));
    v.push(Campo::new(q, "InjectionVial", &i.vial));
    v.push(
        Campo::new(q, "InjectionVolume", i.injection_volume)
            .unit("µL")
            .map("campione.iniezione_uL"),
    );
    v.push(Campo::new(q, "SampleWeight", i.sample_weight));
    v.push(Campo::new(q, "SampleVolume", i.sample_volume));
    v.push(Campo::new(q, "IstdAmount", i.istd_amount));
    v.push(Campo::new(q, "DilutionFactor", i.dilution_factor));
    v.push(Campo::new(q, "Id", &s.id).map("campione.id"));
    v.push(Campo::new(q, "Comment", &s.comment).map("campione.commento"));
    v.push(Campo::new(q, "Vial", &s.vial).map("campione.vial"));
    v.push(
        Campo::new(q, "InstMethod", &s.inst_method)
            .map("campione.metodo_file")
            .id(),
    );
    v.push(Campo::new(q, "ProcMethod", &s.proc_method).id());
    v.push(Campo::new(q, "FileName", &s.file_name).id());
    v.push(Campo::new(q, "Path", &s.path).id());
    for (n, value) in s.user_labels.iter().enumerate() {
        let name = raw
            .raw_file_info
            .label_headings
            .get(n)
            .filter(|h| !h.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("UserLabel{n}"));
        v.push(Campo::new("thermo-raw:SeqRow/UserLabels", &name, value).id());
    }
    let r = "thermo-raw:RawFileInfo";
    v.push(
        Campo::new(r, "ComputerName", &raw.raw_file_info.computer_name)
            .map("operatore.computer")
            .id(),
    );
    v.push(Campo::new(
        r,
        "MethodFilePresent",
        raw.raw_file_info.preamble.method_file_present,
    ));
    v
}

/// InstID strings by position (measured on v66: model twice, serial number, software, hardware).
fn instid(raw: &RawFileReader) -> Vec<Campo> {
    const NAMES: [(&str, Option<&str>); 5] = [
        ("Model", None),
        ("Model (2)", None),
        ("SerialNumber", Some("strumento.serie")),
        ("SoftwareVersion", Some("strumento.software")),
        ("HardwareVersion", Some("strumento.hardware")),
    ];
    raw.inst_id_strings
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.trim().is_empty())
        .map(|(n, t)| {
            let from = format!("thermo-raw:InstID#{n}");
            match NAMES.get(n) {
                Some((k, m)) => {
                    let c = Campo::new(&from, k, t).map_opt(*m);
                    if n == 2 {
                        c.id()
                    } else {
                        c
                    }
                }
                None => Campo::new(&from, &format!("String{n}"), t),
            }
        })
        .collect()
}

/// The analog controllers (pump pressure ...): the channel, its unit and the device it comes from.
fn analog(traces: &[opentfraw::AnalogTrace]) -> Vec<Campo> {
    let mut v = Vec::new();
    for t in traces {
        let from = format!("thermo-raw:Controller{}", t.controller);
        v.push(Campo::new(&from, "ChannelName", &t.name));
        v.push(Campo::new(&from, "Unit", &t.unit));
        v.push(Campo::new(&from, "Samples", t.values.len()));
        v.push(Campo::new(&from, "DeviceDriver", &t.driver));
        v.push(Campo::new(&from, "DeviceModel", &t.model));
        v.push(Campo::new(&from, "DeviceSerialNumber", &t.serial).id());
    }
    v
}

fn run_header(raw: &RawFileReader) -> Vec<Campo> {
    let i = &raw.run_header.sample_info;
    let q = "thermo-raw:RunHeader/SampleInfo";
    vec![
        Campo::new(q, "FirstScan", i.first_scan_number),
        Campo::new(q, "LastScan", i.last_scan_number),
        Campo::new(q, "LowMz", i.low_mz).unit("m/z"),
        Campo::new(q, "HighMz", i.high_mz).unit("m/z"),
        Campo::new(q, "StartTime", i.start_time).unit("min"),
        Campo::new(q, "EndTime", i.end_time).unit("min"),
        Campo::new(q, "MaxIonCurrent", i.max_ion_current),
    ]
}

/// Where a key of the LC script ends up in the sections.
fn script_map(key: &str) -> Option<&'static str> {
    match key {
        "ColumnComp.CC.Temperature.Nominal" => Some("lc.forno_C"),
        "PumpModule.Pump.Pressure.UpperLimit" => Some("lc.pressione_max"),
        "PumpModule.Pump.Flow.Nominal" | "PumpModule.Pump.%B.Value" => Some("lc.gradiente"),
        k if k.starts_with("SamplerModule.") => Some("campionatore.parametri"),
        _ => None,
    }
}

/// All the fields of the file, in a stable order (origin, key); the ones that name a person or a computer carry `identita`.
pub fn fields(
    raw: &RawFileReader,
    traces: &[opentfraw::AnalogTrace],
    pairs: &[super::script::Pair],
    xml: &[Item],
    ident: &Ident,
) -> Vec<V> {
    let mut all = container(raw);
    all.extend(instid(raw));
    all.extend(run_header(raw));
    all.extend(analog(traces));
    all.extend(trailer(raw));
    if let Some(log) = &raw.status_log {
        all.extend(status::fields(log));
    }
    for (t, k, v, u) in pairs {
        let from = t.map_or("thermo-raw:InstrumentMethod/Script".to_string(), |t| {
            format!("thermo-raw:InstrumentMethod/Script@{t}min")
        });
        all.push(
            Campo::new(&from, k, v)
                .unit(u.as_deref().unwrap_or(""))
                .map_opt(script_map(k)),
        );
    }
    for it in xml {
        let c = Campo::new("thermo-raw:InstrumentMethod/XML", &it.name, &it.value);
        all.push(if it.name.contains("/ScanEvent/") {
            c.map("ms.periodi")
        } else {
            c
        });
    }
    for c in &mut all {
        c.identita |= ident.names(&c.valore);
    }
    all.retain(|c| !c.valore.is_empty());
    crate::sortc::slice_by(&mut all, |a, b| {
        (&a.origine, &a.chiave).cmp(&(&b.origine, &b.chiave))
    });
    all.iter().filter_map(Campo::json).collect()
}

mod status;
#[cfg(test)]
mod tests;
