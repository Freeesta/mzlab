//! `spectrum get --bin nominal|0.1 [--mz-offset DA|auto:MZ[@RT]]`: the spectrum of unit-resolution data summed into half-open bins around integers (or
//! tenths) after the offset is taken off (`spectrum-bins/1`, or CSV `bin,mz,counts,points`). The engine's `/api/spectrum` with `rebin=` and `mzoff=`
//! does it (`mzlab_core::nominalbin`), the same route the site and the app answer; this reads the flags, shapes the document and logs the rule and the offset.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, write_value, Out, Value};
use mzlab_core::nominalbin::Rule;

use super::{asked, query, scalar_members, text_of, Asked, At};
use crate::extract::{self, Parsed, Sample};

/// `--bin` and `--mz-offset` as the route takes them.
pub(crate) struct BinOpt {
    pub rule: String,
    pub offset: Option<String>,
}

/// The flags of a request: `None` without `--bin`; a usage message for a rule that is not known, an offset that is not a number or `auto:MZ[@RT]`, a
/// `--mz-offset` with no `--bin`, or a mix with `--minus` or `--sum` (their spectra are not binned).
pub(crate) fn options(
    p: &Parsed,
    minus: bool,
    sum: bool,
) -> std::result::Result<Option<BinOpt>, String> {
    let Some(rule) = p.get("--bin") else {
        return match p.get("--mz-offset") {
            Some(_) => Err("--mz-offset goes with --bin: it moves the bins".into()),
            None => Ok(None),
        };
    };
    if Rule::parse(rule).is_none() {
        return Err(format!("err.spectrum.rebin value={rule}"));
    }
    if minus || sum {
        return Err(
            "--bin is not for --minus or --sum (a difference or a sum is not cut into bins)".into(),
        );
    }
    if let Some(o) = p.get("--mz-offset") {
        crate::mz_offset::check(o)?;
    }
    Ok(Some(BinOpt {
        rule: rule.into(),
        offset: p.get("--mz-offset").map(String::from),
    }))
}

/// A member of the route answer as JSON text.
fn member(v: &Value, key: &str) -> String {
    let mut o = Out::default();
    write_value(&mut o, v.get(key).unwrap_or(&Value::Null));
    o.0
}

/// The stderr lines: the rule, the offset that was taken off and how it was found, and the warning of an estimate that did not go cleanly.
fn notes(name: &str, v: &Value) -> String {
    let rule = v.get("rule").and_then(Value::str).unwrap_or("");
    let width = v.get("width_da").and_then(Value::f64).unwrap_or(0.0);
    let da = v.get("mz_offset_da").and_then(Value::f64).unwrap_or(0.0);
    let ev = v.get("mz_offset");
    let num = |k: &str| {
        ev.and_then(|e| e.get(k))
            .and_then(Value::f64)
            .unwrap_or(0.0)
    };
    let status = ev
        .and_then(|e| e.get("status"))
        .and_then(Value::str)
        .unwrap_or("");
    let how = match (ev.and_then(|e| e.get("mode")).and_then(Value::str), status) {
        (Some("auto"), "not_found") => format!("auto on {}, not found", num("ion_mz")),
        (Some("auto"), _) => format!(
            "auto on {}, {} scans, apex {} min, spread {} Da",
            num("ion_mz"),
            num("n_scans"),
            num("apex_rt_min"),
            num("spread_da")
        ),
        (Some(_), _) if da == 0.0 => "no offset".into(),
        (Some(_), _) => "fixed".into(),
        _ => "none given".into(),
    };
    let mut s = format!(
        "note: bins {rule}: half-open [bin - {}, bin + {}) on the m/z minus the offset\n",
        width / 2.0,
        width / 2.0
    );
    s += &format!("mz offset {name}: {da:+} Da ({how})\n");
    let key = match status {
        "not_found" => Some("warn.mz.offset.notFound"),
        "wide" => Some("warn.mz.offset.wide"),
        _ => None,
    };
    if let Some(k) = key {
        s += &format!(
            "warning: {name}: {} ({k})\n",
            crate::error_text::catalog_text(k).unwrap_or_default()
        );
    }
    s
}

/// One spectrum as bins: the document or CSV, and the lines for the terminal.
pub(crate) fn get(
    s: &mut Sample,
    at: &At,
    level: Option<u8>,
    chan: Option<&str>,
    csv: bool,
    b: &BinOpt,
) -> Result<(String, String)> {
    let Asked {
        level,
        auto,
        resolved,
        mut pairs,
        ..
    } = asked(s, at, level, chan)?;
    pairs.push(("rebin", b.rule.clone()));
    if let Some(o) = &b.offset {
        pairs.push(("mzoff", o.clone()));
    }
    let text = s
        .routes
        .handle("/api/spectrum", &query(&pairs))?
        .ok_or_else(|| Error::new("err.scan.none"))?;
    let bad = || Error::new("error.badIndex");
    let col = |k: &str| extract::tokens(&text, k).ok_or_else(bad);
    let (bin, mz, y, pts) = (col("bin")?, col("mz")?, col("y")?, col("points")?);
    let v = parse(text.as_bytes())?;
    let notes = notes(&s.name, &v);
    if csv {
        let mut o = "bin,mz,counts,points\n".to_string();
        for i in 0..bin.len() {
            o += &format!("{},{},{},{}\n", bin[i], mz[i], y[i], pts[i]);
        }
        return Ok((o, notes));
    }
    let mut o = Out::default();
    o.raw("{\"schema\":\"spectrum-bins/1\",\"file\":")
        .str(&s.name);
    o.raw(",\"sample\":")
        .raw(&s.sample.map_or("null".to_string(), |i| i.to_string()));
    o.raw(&format!(",\"x_unit\":\"m/z\",\"y_unit\":\"counts\",\"ms_level\":{level},\"rule\":{},\"width_da\":{}", member(&v, "rule"), member(&v, "width_da")));
    o.raw(&format!(
        ",\"mz_offset_da\":{},\"mz_offset\":{},\"n_bins\":{},\"bins\":[",
        member(&v, "mz_offset_da"),
        member(&v, "mz_offset"),
        bin.len()
    ));
    for i in 0..bin.len() {
        o.raw(if i > 0 { "," } else { "" });
        o.raw(&format!(
            "{{\"bin\":{},\"mz\":{},\"counts\":{},\"points\":{}}}",
            bin[i], mz[i], y[i], pts[i]
        ));
    }
    let scan = format!(
        "{resolved}{}",
        auto.map_or(String::new(), |a| format!(",\"level_auto\":{a}"))
    );
    o.raw(&format!(
        "],\"evidence\":{{\"route\":\"/api/spectrum\",\"query\":{},\"scan\":{scan},\"answer\":{{{}}}}}}}",
        extract::jstr(&text_of(&pairs)),
        scalar_members(&text)?
    ));
    Ok((o.0, notes))
}
