//! `peak table --ions [--scan-type K] [--set NAME=VALUE...]`: the ion-scale request of the table (`mzlab_core::manint::table_text_ions`). Per sample it
//! says the scan type (named, or the one the source implies: the SIM / MRM channels of a file, the `ion_scale` of a `trace xic --ions` trace) and the
//! method values (`--set` wins over the `method/1` of the file), per trace the factor the trace brought. The factor of each row is worked out by the
//! engine and written in its columns; the lines here are the log of the automatic choices.

use mzlab_core::ionscale::{Facts, ScanKind};
use mzlab_core::json::Out;

use crate::raw_xic_ions::{fill_from_method, parse_facts};

/// The scan type named and the method values given with `--set`.
pub(crate) type IonAsk = (Option<ScanKind>, Facts);

/// `--ions [--scan-type K] [--set NAME=VALUE...]`; a scan type or a method value without `--ions` is a usage error.
pub(crate) fn parse_flags(
    ions: bool,
    scan_type: Option<&str>,
    sets: &[String],
    table: bool,
) -> Result<Option<IonAsk>, String> {
    if !ions {
        if scan_type.is_some() || !sets.is_empty() {
            return Err("err.cli.peaktable.needsIons".into());
        }
        return Ok(None);
    }
    if !table {
        return Err("--ions goes with the table, not --auto or --shifts".into());
    }
    let kind = match scan_type {
        None => None,
        Some(v) => Some(ScanKind::parse(v).ok_or("--scan-type needs ems, epi, q1, sim or other")?),
    };
    let given: Vec<&str> = sets.iter().flat_map(|v| v.split(',')).collect();
    Ok(Some((kind, parse_facts(&given)?)))
}

/// What the call knows of the ion scale of one sample.
pub(crate) struct Scale {
    pub kind: Option<ScanKind>,
    pub from: String,
    pub facts: Facts,
    /// Per trace, the seconds one point accumulates ions when the trace brought them.
    pub seconds: Vec<Option<f64>>,
    /// The `choices` lines of the method values read from the file.
    pub log: Vec<String>,
}

/// The scale of a sample read from a file: its channels are SIM / MRM chromatograms, so the scan type is `sim` unless named, and the dwell comes from
/// `--set`, else from the `method/1` of the file.
pub(crate) fn of_file(
    ask: &IonAsk,
    method: &mzlab_core::error::Result<Option<String>>,
    name: &str,
) -> Scale {
    let (named, _) = ask;
    let mut set = (Some(named.unwrap_or(ScanKind::Sim)), ask.1);
    let log = fill_from_method(&mut set, method, name);
    Scale {
        kind: set.0,
        from: if named.is_some() {
            "scan type given".into()
        } else {
            "the channels of the file are SIM / MRM chromatograms".into()
        },
        facts: set.1,
        seconds: Vec::new(),
        log,
    }
}

/// The scale of a sample of supplied traces: the scan type named, else the one its `trace xic --ions` traces carry; their factors per trace.
pub(crate) fn of_traces(ask: &IonAsk, from: Option<&crate::traces_in::SampleScale>) -> Scale {
    let carried = from
        .and_then(|s| s.scan_type.as_deref())
        .and_then(ScanKind::parse);
    let (kind, why) = match (ask.0, carried) {
        (Some(k), _) => (Some(k), "scan type given"),
        (None, Some(k)) => (Some(k), "the ion_scale of the traces (trace xic --ions)"),
        (None, None) => (None, ""),
    };
    Scale {
        kind,
        from: why.into(),
        facts: ask.1,
        seconds: from.map_or(Vec::new(), |s| s.seconds.clone()),
        log: Vec::new(),
    }
}

/// The scales of `--traces` samples: what each trace file carries of its own (`trace xic --ions`), else the request.
pub(crate) fn of_trace_files(
    ask: Option<&IonAsk>,
    paths: &[std::path::PathBuf],
) -> Result<Vec<Scale>, mzlab_core::error::Error> {
    let Some(a) = ask else { return Ok(Vec::new()) };
    Ok(crate::traces_in::scales(paths)?
        .iter()
        .map(|s| of_traces(a, Some(s)))
        .collect())
}

/// The ion table: its CSV, or with `json` the `{columns, rows}` of `to_json` plus the `ion_scale` choices; the choices are logged on `warn`.
pub(crate) fn table(
    files: &str,
    windows: &str,
    sums: bool,
    json: bool,
    scales: &[Scale],
    warn: &mut String,
    to_json: fn(&str) -> String,
) -> Result<String, mzlab_core::error::Error> {
    let doc = doc(scales);
    let csv = mzlab_core::manint::table_text_ions(files, windows, sums, &doc)?;
    let choices = mzlab_core::manint::choices_text(files, &doc)?;
    *warn += &log_lines(&choices, scales);
    if !json {
        return Ok(csv);
    }
    let mut j = to_json(&csv);
    j.truncate(j.trim_end().len() - 1);
    Ok(format!("{j},\"ion_scale\":{choices}}}\n"))
}

/// The `ions` document of the engine (`manint_ions`), one entry per sample in the order of the files.
pub(crate) fn doc(samples: &[Scale]) -> String {
    let mut o = Out::default();
    o.raw("{\"samples\":[");
    for (i, s) in samples.iter().enumerate() {
        o.raw(if i > 0 { ",{" } else { "{" });
        let mut sep = "";
        if let Some(k) = s.kind {
            o.raw("\"scan_type\":").str(k.name());
            sep = ",";
        }
        o.raw(sep).raw("\"scan_from\":").str(&s.from);
        for (n, v) in [
            ("fill_ms", s.facts.fill_ms),
            ("dwell_ms", s.facts.dwell_ms),
            ("scan_s", s.facts.scan_s),
            ("step_da", s.facts.step_da),
            ("mass_lo", s.facts.mass_lo),
            ("mass_hi", s.facts.mass_hi),
        ] {
            if let Some(v) = v {
                o.raw(",\"").raw(n).raw("\":").num(v);
            }
        }
        o.raw(",\"traces\":[");
        for (k, t) in s.seconds.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" });
            match t {
                Some(v) => o.raw("{\"ions_per_cps\":").num(*v).raw("}"),
                None => o.raw("null"),
            };
        }
        o.raw("]}");
    }
    o.raw("]}");
    o.0
}

/// The stderr lines of the ion scale: one per sample (scan type and where it came from), the method values read, and a reminder where the factor
/// of each row is.
pub(crate) fn log_lines(choices: &str, samples: &[Scale]) -> String {
    let mut out = String::new();
    let list = mzlab_core::json::parse(choices.as_bytes()).ok();
    for (i, c) in list.iter().flat_map(|l| l.arr().iter()).enumerate() {
        let g = |k: &str| {
            c.get(k)
                .and_then(mzlab_core::json::Value::str)
                .unwrap_or("")
        };
        out += &format!(
            "ion scale: {}: {} scans ({}); the factor of each row is in ions_per_cps and ion_source (method, data = 1 / ion quantum, trace)\n",
            g("sample"),
            g("scan_type"),
            g("scan_type_from")
        );
        for l in samples.get(i).map_or(&[][..], |s| &s.log[..]) {
            out += &format!("choice: {l}\n");
        }
    }
    out
}
