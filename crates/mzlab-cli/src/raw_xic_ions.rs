//! `raw xic --ions` in the CLI (`raw_xic_cmd`): the method values that are `--set` keys, the `ions` member of `query`, and the line of `provenance.choices`
//! that says where the cps-to-ions factor of a file came from. The factor itself and the keys of the rows are the route's (`mzlab_core::ionscale`).

use mzlab_core::ionfacts::Read;
use mzlab_core::ionscale::Facts;
use mzlab_core::json::{parse, Out};
use mzlab_core::rawxic::Ask;

use crate::extract::jstr;
use crate::raw_cmd::choice;

/// The method values of `--ions` (`ionscale::Facts`), also `--set` keys.
pub(crate) const FACT_KEYS: &[&str] = &[
    "fill_ms", "dwell_ms", "scan_s", "step_da", "mass_lo", "mass_hi",
];

/// `query.ions`: the scan type named (null: read from the scans) and every method value, only with the opt-in so the bytes without it are unchanged.
pub(crate) fn params_member(o: &mut Out, k: &Ask) {
    let Some((kind, f)) = &k.ions else {
        return;
    };
    o.raw(",\"ions\":{\"scan_type\":")
        .raw(&kind.map_or("null".into(), |k| jstr(k.name())));
    for (n, v) in [
        ("fill_ms", f.fill_ms),
        ("dwell_ms", f.dwell_ms),
        ("scan_s", f.scan_s),
        ("step_da", f.step_da),
        ("mass_lo", f.mass_lo),
        ("mass_hi", f.mass_hi),
    ] {
        o.raw(&format!(",\"{n}\":")).opt(v);
    }
    o.raw("}");
}

fn slot<'a>(f: &'a mut Facts, field: &str) -> Option<&'a mut Option<f64>> {
    Some(match field {
        "fill_ms" => &mut f.fill_ms,
        "dwell_ms" => &mut f.dwell_ms,
        "scan_s" => &mut f.scan_s,
        "step_da" => &mut f.step_da,
        "mass_lo" => &mut f.mass_lo,
        "mass_hi" => &mut f.mass_hi,
        _ => return None,
    })
}

/// The ask of one file with the method values the file's `method/1` gives (`method`: its text, `None` when it carries none) for every value `--set` left
/// out, and the `provenance.choices` lines that say what was read, from which experiment, and what `--set` kept. A value given with `--set` always wins.
pub(crate) fn with_method(ask: &Ask, method: Option<&str>, name: &str) -> (Ask, Vec<String>) {
    let mut ask = ask.clone();
    let Some((kind, set)) = ask.ions.as_mut() else {
        return (ask, Vec::new());
    };
    let what = format!("ion_method {name}");
    let doc = method
        .filter(|m| !m.is_empty())
        .map(|m| parse(m.as_bytes()));
    let (_, read): (Facts, Vec<Read>) = match doc {
        Some(Ok(d)) => Facts::from_method(&d, *kind),
        Some(Err(_)) => {
            let why = "the method of the file could not be read; the values are --set or the data estimate";
            return (ask.clone(), vec![choice(&what, "null", why)]);
        }
        None => {
            let why =
                "the file carries no method/1 document; the values are --set or the data estimate";
            return (ask.clone(), vec![choice(&what, "null", why)]);
        }
    };
    let mut log = Vec::new();
    if read.is_empty() {
        let why = "the method names no value the ion scale uses (the dwell of a transition); the values are --set or the data estimate";
        log.push(choice(&what, "null", why));
    }
    for r in read {
        let given = slot(set, r.field).and_then(|s| *s);
        let used = given.or(r.value);
        if let (None, Some(v), Some(s)) = (given, r.value, slot(set, r.field)) {
            *s = Some(v);
        }
        let why = match (given, r.value) {
            (Some(g), Some(m)) if g != m => format!("--set wins over the method value {m}: {}", r.from),
            (Some(_), Some(_)) => format!("--set given, the method says the same: {}", r.from),
            (Some(_), None) => format!("--set given: {}", r.from),
            (None, Some(_)) => format!("read from the method of the file: {}; the scan type decides whether the factor uses it (see ion_scale)", r.from),
            (None, None) => r.from.clone(),
        };
        log.push(choice(
            &format!("{what} {}", r.field),
            &used.map_or("null".into(), |v| v.to_string()),
            &why,
        ));
    }
    (ask, log)
}

/// The log line of the ion scale of one file: the factor, its source and the fields or the data estimate behind it.
pub(crate) fn scale_choice(name: &str, members: &str) -> String {
    let v = parse(format!("{{{members}}}").as_bytes()).ok();
    let sc = v.as_ref().and_then(|v| v.get("ion_scale"));
    let (value, why) = match sc {
        Some(sc) if sc.get("ions_per_cps").is_some() => {
            let f = sc.get("ions_per_cps").and_then(|c| c.f64()).unwrap_or(0.0);
            let src = sc.get("source").and_then(|c| c.str()).unwrap_or("");
            let field = sc.get("field").and_then(|c| c.str()).unwrap_or("");
            (f.to_string(), format!("ions per cps of the {} scans from the {src} ({field}); a method value is --set or read from the method of the file (ion_method choices), the data estimate is 1 / ion quantum", sc.get("scan_type").and_then(|c| c.str()).unwrap_or("")))
        }
        _ => ("null".to_string(), "no method time for the scan type and no accepted ion quantum: the ions_* keys are absent, the cps columns stand (see ion_scale_choice)".to_string()),
    };
    choice(&format!("ion_scale {name}"), &value, &why)
}

#[cfg(test)]
#[path = "raw_xic_ions_cmd_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "raw_xic_ions_method_tests.rs"]
mod method_tests;
