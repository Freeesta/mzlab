//! Facts of one file for `ispezione/1`: polarity, resolution class, acquisition type, ranges, MRM, gradient and estimates read from the engine
//! answers (`Item.info()`, `metodo/1`, `features.estimate`) by whitelisted keys only. Used by `inspect`.

use mzlab_core::json::Value;

use crate::inspect::{clean, n, num, path, text, Obj, GRADIENT_POINTS};

pub(crate) fn strs(items: impl IntoIterator<Item = String>) -> Value {
    Value::Arr(items.into_iter().map(Value::Str).collect())
}

/// The name of an acquisition type for a code of `Item.info().acq`, in the order the types are listed.
pub(crate) const ACQ: [(&str, &str); 11] = [
    ("MS1", "full_scan"),
    ("SIM", "sim"),
    ("DDA", "dda"),
    ("DIA", "dia"),
    ("AIF", "aif"),
    ("PRM", "product_ion"),
    ("MS2", "ms2"),
    ("MSn", "msn"),
    ("PREC", "precursor_ion"),
    ("NL", "neutral_loss"),
    ("NG", "neutral_gain"),
];

pub(crate) fn polarity_of(items: &[Value]) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for p in items.iter().filter_map(|i| i.get("polarity")?.str()) {
        if !seen.contains(&p) {
            seen.push(p);
        }
    }
    match seen.as_slice() {
        [] => "unknown",
        ["positive"] => "positive",
        ["negative"] => "negative",
        ["unknown"] => "unknown",
        _ => "alternating",
    }
    .to_string()
}

pub(crate) fn class_of(items: &[Value], level2: bool) -> Option<(bool, Vec<String>, Option<f64>)> {
    let (count, prof, res) = if level2 {
        ("ms2", "prof2", "res2")
    } else {
        ("ms1", "prof1", "res1")
    };
    let item = items
        .iter()
        .find(|i| num(i.get(count)).is_some_and(|c| c > 0.0))?;
    let p = item.get(prof)?;
    let hr = p.get("hr")?.bool()?;
    let mut basis = Vec::new();
    if let Some(a) = p.get("an").and_then(Value::str).filter(|a| *a != "?") {
        basis.push(format!("analyzer:{}", clean(a)));
    }
    let rp = num(item.get(res));
    if rp.is_some() {
        basis.push("resolving_power".to_string());
    }
    if let Some(d) = num(p.get("dec")) {
        basis.push(format!("mass_decimals:{d}"));
    }
    Some((hr, basis, rp))
}

pub(crate) fn range(items: &[Value], lo: &str, hi: &str) -> Option<Value> {
    let a = items
        .iter()
        .filter_map(|i| num(i.get(lo)))
        .fold(f64::INFINITY, f64::min);
    let b = items
        .iter()
        .filter_map(|i| num(i.get(hi)))
        .fold(f64::NEG_INFINITY, f64::max);
    (a.is_finite() && b.is_finite()).then(|| Value::Arr(vec![n(a), n(b)]))
}

pub(crate) fn acquisition(items: &[Value]) -> Vec<String> {
    let codes: Vec<&str> = items
        .iter()
        .flat_map(|i| i.get("acq").map_or(&[][..], Value::arr))
        .filter_map(Value::str)
        .collect();
    let mut out: Vec<String> = ACQ
        .iter()
        .filter(|(c, _)| codes.contains(c))
        .map(|(_, name)| name.to_string())
        .collect();
    if items.iter().any(|i| {
        i.get("kind").and_then(Value::str) == Some("mrm")
            || num(i.get("srm")).is_some_and(|s| s > 0.0)
    }) {
        out.push("mrm".into());
    }
    if items.iter().any(|i| {
        i.get("infusion").and_then(Value::bool) == Some(true)
            || i.get("kind").and_then(Value::str) == Some("infusion")
    }) {
        out.push("infusion".into());
    }
    out
}

pub(crate) fn mode_of(items: &[Value]) -> &'static str {
    let mut seen: Vec<&str> = items
        .iter()
        .filter_map(|i| i.get("spectrum_mode")?.str())
        .collect();
    seen.dedup();
    match seen.as_slice() {
        ["profile"] => "profile",
        ["centroid"] => "centroid",
        [] => "unknown",
        _ => "mixed",
    }
}

pub(crate) fn mrm_section(src: &Value, items: &[Value]) -> Option<Value> {
    let list = src.get("mrm").map_or(&[][..], Value::arr);
    let mut o = Obj(Vec::new());
    if list.is_empty() {
        let k: f64 = items
            .iter()
            .filter_map(|i| num(i.get("srm")))
            .fold(0.0, f64::max);
        if k <= 0.0 {
            return None;
        }
        o.put("transitions", Some(Value::Num(k)));
        return Some(Value::Obj(o.0));
    }
    o.put("transitions", Some(Value::Num(list.len() as f64)));
    for key in ["q1", "q3"] {
        let xs: Vec<f64> = list.iter().filter_map(|t| num(t.get(key))).collect();
        if !xs.is_empty() {
            let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            o.put(key, Some(Value::Arr(vec![n(lo), n(hi)])));
        }
    }
    Some(Value::Obj(o.0))
}

pub(crate) fn gradient(method: &Value) -> Option<Value> {
    let g = path(method, &["lc", "gradiente"])?.arr();
    let points: Vec<Value> = g
        .iter()
        .filter_map(|p| {
            Some(Value::Obj(vec![
                ("t_min".into(), n(num(p.get("t_min"))?)),
                ("B_pct".into(), n(num(p.get("B_pct"))?)),
            ]))
        })
        .take(GRADIENT_POINTS)
        .collect();
    (!points.is_empty()).then_some(Value::Arr(points))
}

pub(crate) fn estimate_fields(o: &mut Obj, est: &Value, unit_class: bool) {
    o.put("dead_min", num(est.get("dead_min")).map(n));
    o.put("noise", num(est.get("noise")).map(n));
    let (key, unit) = if unit_class {
        ("da", "Da")
    } else {
        ("ppm", "ppm")
    };
    if let Some(v) = num(est.get(key)) {
        let mut m = Obj(vec![
            ("unit".into(), Value::Str(unit.into())),
            ("value".into(), n(v)),
        ]);
        m.put("sigma", num(est.get(&format!("{key}_sigma"))).map(n));
        m.put("n", num(est.get(&format!("{key}_n"))).map(Value::Num));
        o.put("mass_error", Some(Value::Obj(m.0)));
    }
    if let Some(w) = num(est.get("fwhm_min")) {
        let mut m = Obj(vec![("fwhm_min".into(), n(w))]);
        m.put("n", num(est.get("fwhm_n")).map(Value::Num));
        o.put("peak_width", Some(Value::Obj(m.0)));
    }
}

/// The signals of a file for the QC section: `[{mz, rt, area}]` from the strongest traces of its feature detection.
pub(crate) fn signals(src: &Value) -> Vec<[f64; 3]> {
    src.get("signals")
        .map_or(&[][..], Value::arr)
        .iter()
        .filter_map(|s| Some([num(s.get("mz"))?, num(s.get("rt"))?, num(s.get("area"))?]))
        .collect()
}

/// Format, vendor, model and source of the instrument, from the method and then from the items.
pub(crate) fn put_instrument(o: &mut Obj, method: &Value, items: &[Value]) {
    o.put(
        "format",
        text(path(method, &["origine", "formato"])).map(Value::Str),
    );
    o.put(
        "vendor",
        text(path(method, &["strumento", "produttore"])).map(Value::Str),
    );
    let model = text(path(method, &["strumento", "modello"]))
        .or_else(|| items.iter().find_map(|i| text(i.get("instrument"))));
    o.put("model", model.map(Value::Str));
    o.put(
        "source",
        text(path(method, &["ms", "sorgente", "tipo"])).map(Value::Str),
    );
}

/// Levels, acquisition, parts, ranges, cycle and isolation windows.
pub(crate) fn put_shape(o: &mut Obj, items: &[Value], stats: &Value) {
    let mut levels: Vec<f64> = Vec::new();
    for (key, l) in [("ms1", 1.0), ("ms2", 2.0)] {
        if items
            .iter()
            .any(|i| num(i.get(key)).is_some_and(|c| c > 0.0))
        {
            levels.push(l);
        }
    }
    if items
        .iter()
        .any(|i| num(i.get("max_level")).is_some_and(|m| m >= 3.0))
    {
        levels.push(3.0);
    }
    o.put(
        "levels",
        Some(Value::Arr(levels.into_iter().map(Value::Num).collect())),
    );
    let acq = acquisition(items);
    o.put("acquisition", Some(strs(acq)));
    if items.len() > 1 {
        let parts = items.iter().map(|i| {
            let mut p = Obj(Vec::new());
            p.put("part", text(i.get("part")).map(Value::Str));
            p.put("polarity", text(i.get("polarity")).map(Value::Str));
            p.put("kind", text(i.get("kind")).map(Value::Str));
            p.put("scans", num(i.get("scans")).map(Value::Num));
            Value::Obj(p.0)
        });
        o.put("parts", Some(Value::Arr(parts.collect())));
    }
    o.put("rt_range_min", range(items, "rt_min", "rt_max"));
    o.put("mz_range", range(items, "mz_min", "mz_max"));
    o.put("mz2_range", range(items, "mz2_min", "mz2_max"));
    o.put("cycle_ms1_s", num(stats.get("cycle_ms1_s")).map(n));
    o.put("ms2_per_cycle", num(stats.get("ms2_per_cycle")).map(n));
    if num(stats.get("windows_distinct")).is_some() {
        let mut w = Obj(Vec::new());
        w.put(
            "windows",
            num(stats.get("windows_distinct")).map(Value::Num),
        );
        w.put("width_mz", stats.get("width_mz").cloned());
        w.put("list", stats.get("windows").cloned());
        o.put("isolation", Some(Value::Obj(w.0)));
    }
}
