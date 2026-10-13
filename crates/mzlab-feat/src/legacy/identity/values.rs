//! The features of the payload of `identity_graph` and the Python way of printing and sorting its values.

use super::*;

/// Python's `str()` of a JSON scalar (the keys of the objects that Python builds from the values).
pub(super) fn py_str(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Num(x) if x.fract() == 0.0 && x.abs() < 1e16 => format!("{}", *x as i64),
        Value::Num(x) => format!("{x}"),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Null => "None".into(),
        _ => String::new(),
    }
}

/// Python's order of two keys: numbers by value, anything else by its text.
pub(super) fn cmp_value(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Num(x), Value::Num(y)) => x.total_cmp(y),
        _ => py_str(a).cmp(&py_str(b)),
    }
}

/// A JSON value written back as it came.
pub(crate) fn write_value(v: &Value, o: &mut Out) {
    match v {
        Value::Null => {
            o.raw("null");
        }
        Value::Bool(b) => {
            o.raw(if *b { "true" } else { "false" });
        }
        Value::Num(x) => {
            o.num(*x);
        }
        Value::Str(s) => {
            o.str(s);
        }
        Value::Arr(a) => {
            o.raw("[");
            for (i, x) in a.iter().enumerate() {
                o.raw(if i > 0 { "," } else { "" });
                write_value(x, o);
            }
            o.raw("]");
        }
        Value::Obj(m) => {
            o.raw("{");
            for (i, (k, x)) in m.iter().enumerate() {
                o.raw(if i > 0 { "," } else { "" }).str(k).raw(":");
                write_value(x, o);
            }
            o.raw("}");
        }
    }
}

/// The options of `build` (`DEFAULTS` updated by `opt`).
pub(super) struct Opt {
    pub(super) ppm: f64,
    pub(super) abs_da: f64,
    pub(super) r_min: f64,
    pub(super) apex_scans: f64,
    pub(super) cycle_s: f64,
    pub(super) min_common: usize,
    pub(super) max_hyp: usize,
}

pub(super) fn opt_of(v: Option<&Value>) -> Opt {
    let g = |k: &str, d: f64| v.and_then(|o| o.get(k)).and_then(Value::f64).unwrap_or(d);
    Opt {
        ppm: g("ppm", 5.0),
        abs_da: g("abs_da", 0.002),
        r_min: g("r_min", 0.85),
        apex_scans: g("apex_scans", 2.0),
        cycle_s: g("cycle_s", 1.0),
        min_common: g("min_common", 3.0) as usize,
        max_hyp: g("max_hyp_members", 12.0) as usize,
    }
}

/// One feature of the payload.
pub(super) struct Feat<'a> {
    pub(super) v: &'a Value,
    pub(super) key: String,
    pub(super) mz: f64,
    pub(super) rt: f64,
    pub(super) areas: Vec<f64>,
    pub(super) ms2: Option<Spec>,
    /// `iin` when its id is set (the groups of mzMine); `iin_any` as it came (its neutral mass is a candidate).
    pub(super) iin: Option<&'a Value>,
    pub(super) iin_any: Option<&'a Value>,
}

pub(super) fn feats_of(list: &[Value], s: usize) -> Res<Vec<Feat<'_>>> {
    list.iter()
        .map(|f| {
            let num = |k: &str| {
                f.get(k)
                    .and_then(Value::f64)
                    .ok_or_else(|| format!("feature without {k}"))
            };
            let mut areas = vec![0.0; s.max(1)];
            for p in f.get("a").map(Value::arr).unwrap_or(&[]) {
                if let (Some(i), Some(a)) =
                    (p.at(0).and_then(Value::f64), p.at(1).and_then(Value::f64))
                {
                    if i >= 0.0 && (i as usize) < s {
                        areas[i as usize] = a;
                    }
                }
            }
            let ms2 = f.get("ms2").filter(|m| !m.is_null()).and_then(|m| {
                let fl = |k| {
                    m.get(k)
                        .map(Value::arr)
                        .unwrap_or(&[])
                        .iter()
                        .filter_map(Value::f64)
                        .collect::<Vec<f64>>()
                };
                edge::spec(&fl("mz"), &fl("it"))
            });
            let iin = f.get("iin").filter(|g| {
                g.get("id")
                    .is_some_and(|i| !i.is_null() && i.str() != Some(""))
            });
            Ok(Feat {
                v: f,
                key: py_str(f.get("k").unwrap_or(&Value::Null)),
                mz: num("mz")?,
                rt: num("rt")?,
                areas,
                ms2,
                iin,
                iin_any: f.get("iin").filter(|g| !g.is_null()),
            })
        })
        .collect()
}
