//! `raw xic --ions` in the CLI (`raw_xic_cmd`): the method values that are `--set` keys, the `ions` member of `query`, and the line of `provenance.choices`
//! that says where the cps-to-ions factor of a file came from. The factor itself and the keys of the rows are the route's (`mzlab_core::ionscale`).

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

/// The log line of the ion scale of one file: the factor, its source and the fields or the data estimate behind it.
pub(crate) fn scale_choice(name: &str, members: &str) -> String {
    let v = parse(format!("{{{members}}}").as_bytes()).ok();
    let sc = v.as_ref().and_then(|v| v.get("ion_scale"));
    let (value, why) = match sc {
        Some(sc) if sc.get("ions_per_cps").is_some() => {
            let f = sc.get("ions_per_cps").and_then(|c| c.f64()).unwrap_or(0.0);
            let src = sc.get("source").and_then(|c| c.str()).unwrap_or("");
            let field = sc.get("field").and_then(|c| c.str()).unwrap_or("");
            (f.to_string(), format!("ions per cps of the {} scans from the {src} ({field}); the method time was read from --set, the data estimate is 1 / ion quantum", sc.get("scan_type").and_then(|c| c.str()).unwrap_or("")))
        }
        _ => ("null".to_string(), "no method time for the scan type and no accepted ion quantum: the ions_* keys are absent, the cps columns stand (see ion_scale_choice)".to_string()),
    };
    choice(&format!("ion_scale {name}"), &value, &why)
}

#[cfg(test)]
#[path = "raw_xic_ions_cmd_tests.rs"]
mod tests;
