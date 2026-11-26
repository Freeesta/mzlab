//! `trace xic --ions`: the opt-in second scale of every trace. One `/api/xicions` question answers the factor of the level (`ionscale::factor`, its
//! evidence and the log of the choice) and the ions of every ion of the list; this module builds the question, slices the answer verbatim and writes the
//! one-line note. The bytes of a call without `--ions` never pass through here.

use mzlab_core::error::{Error, Result};
use mzlab_core::ionscale::{Facts, ScanKind};
use mzlab_core::json::parse;
use mzlab_core::routes::Query;

use crate::extract::{self, Parsed, Sample};
use crate::raw_xic_ions::{fill_from_method, FACT_KEYS};

/// The ion-scale values of a call: the scan type named (`None`: read from the scans) and the method values given with `--set`.
pub(crate) type IonAsk = (Option<ScanKind>, Facts);

/// `--ions [--scan-type K] [--set NAME=VALUE...]`; a scan type or a method value without `--ions` is a usage error.
pub(crate) fn parse_flags(p: &Parsed) -> std::result::Result<Option<IonAsk>, String> {
    let kind = match p.get("--scan-type") {
        None => None,
        Some(v) => Some(ScanKind::parse(v).ok_or("--scan-type needs ems, epi, q1, sim or other")?),
    };
    let mut f = Facts::default();
    // NAME=VALUE, repeated or separated by commas as in `raw xic --set`
    let given: Vec<&str> = p
        .all("--set")
        .into_iter()
        .flat_map(|v| v.split(','))
        .collect();
    for kv in &given {
        let (k, v) = kv.split_once('=').ok_or("--set needs NAME=VALUE")?;
        let x = v
            .parse::<f64>()
            .ok()
            .filter(|x| x.is_finite() && *x > 0.0)
            .ok_or(format!("{k} must be a number above 0"))?;
        *match k {
            "fill_ms" => &mut f.fill_ms,
            "dwell_ms" => &mut f.dwell_ms,
            "scan_s" => &mut f.scan_s,
            "step_da" => &mut f.step_da,
            "mass_lo" => &mut f.mass_lo,
            "mass_hi" => &mut f.mass_hi,
            _ => {
                return Err(format!(
                    "--set takes a method value here: {}",
                    FACT_KEYS.join(", ")
                ))
            }
        } = Some(x);
    }
    if !p.flag("--ions") {
        if kind.is_some() || !given.is_empty() {
            return Err("err.cli.tracexic.needsIons".into());
        }
        return Ok(None);
    }
    Ok(Some((kind, f)))
}

/// What the route answered, sliced as written: the factor, the log of the choice and the ions of each ion of the list (`None`: no factor).
pub(crate) struct Split<'a> {
    pub scale: &'a str,
    pub choice: &'a str,
    pub ions: Vec<Option<Vec<&'a str>>>,
}

/// The `/api/xicions` answer of the whole list, and the log lines of the method values read from the file (`ion_method ...`).
pub(crate) fn ask(
    s: &mut Sample,
    mz: &[f64],
    tol: &[f64],
    level: u8,
    pol: Option<i8>,
    smooth: Option<usize>,
    ions: &IonAsk,
) -> Result<(String, Vec<String>)> {
    // the method of the file fills what `--set` left out, as in `raw xic --ions`; a value given always wins
    let method = s
        .mzml
        .method(&s.name, mzlab_core::method_fit::DOC_MAX as u64);
    let mut set = *ions;
    let log = fill_from_method(&mut set, &method, &s.name);
    let join = |v: &[f64]| {
        v.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut q = vec![
        ("mz".to_string(), join(mz)),
        ("tol".into(), join(tol)),
        ("ions".into(), "1".into()),
        ("ms_level".into(), level.to_string()),
    ];
    if let Some(k) = smooth {
        q.push(("smooth".into(), k.to_string()));
    }
    if let Some(k) = set.0 {
        q.push(("scan_type".into(), k.name().into()));
    }
    let f = set.1;
    for (n, v) in [
        ("fill_ms", f.fill_ms),
        ("dwell_ms", f.dwell_ms),
        ("scan_s", f.scan_s),
        ("step_da", f.step_da),
        ("mass_lo", f.mass_lo),
        ("mass_hi", f.mass_hi),
    ] {
        if let Some(v) = v {
            q.push((n.into(), v.to_string()));
        }
    }
    s.routes.pol = pol;
    let text = s
        .routes
        .xic_ions_route(&Query(q))?
        .ok_or_else(|| Error::new("error.badIndex"))?;
    Ok((text, log))
}

/// The slices of an answer of `xic_ions_route`: `{"ion_scale":S,"ion_scale_choice":C,"traces":[{"mz":..,"ions":[..]|null},..]}`.
pub(crate) fn split(text: &str) -> Result<Split<'_>> {
    let bad = || Error::new("error.badIndex");
    let a = text.strip_prefix("{\"ion_scale\":").ok_or_else(bad)?;
    let (scale, rest) = a.split_once(",\"ion_scale_choice\":").ok_or_else(bad)?;
    let (choice, traces) = rest.split_once(",\"traces\":[").ok_or_else(bad)?;
    let traces = traces.strip_suffix("]}").ok_or_else(bad)?;
    let ions = traces
        .split(",{\"mz\":")
        .map(|t| extract::tokens(t, "ions"))
        .collect();
    Ok(Split {
        scale,
        choice,
        ions,
    })
}

/// The evidence members every trace carries with the opt-in: the factor, the log of the choice and what was read from the method.
pub(crate) fn evidence(sp: &Split, log: &[String]) -> String {
    format!(
        ",\"ion_scale\":{},\"ion_scale_choice\":{},\"ion_method_choices\":[{}]",
        sp.scale,
        sp.choice,
        log.join(",")
    )
}

/// The stderr line of the factor: its value and where it came from, or why the cps columns stand alone.
pub(crate) fn note(sp: &Split) -> String {
    let v = parse(sp.scale.as_bytes()).ok();
    let why = parse(sp.choice.as_bytes())
        .ok()
        .and_then(|c| c.get("evidence").and_then(|e| e.str().map(String::from)))
        .unwrap_or_default();
    match v
        .as_ref()
        .and_then(|v| v.get("ions_per_cps").and_then(|f| f.f64()))
    {
        Some(f) => {
            let g = |k: &str| {
                v.as_ref()
                    .and_then(|v| v.get(k).and_then(|x| x.str().map(String::from)))
                    .unwrap_or_default()
            };
            format!(
                "note: ion scale of the {} scans: {f} ions per cps ({}: {}); {why}\n",
                g("scan_type"),
                g("source"),
                g("field")
            )
        }
        None => format!("note: no ion scale, the traces stay in cps; {why}\n"),
    }
}

#[cfg(test)]
#[path = "trace_xic_ions_tests.rs"]
mod tests;
