//! `reflist-match/1`: which entries of a set of reference ions lie within a window of an m/z. The table of the Known contaminants panel, the app and
//! `mzlab-cli db ref-list --match` ask this one function; the page used to do it in JavaScript. Items arrive as columns (m/z, polarity, group).

use crate::error::{Error, Result};
use crate::json::{parse, Value};
use crate::jsonb::{int, num, text, write, Obj};

/// Window of a ppm search: the tolerance in Da, never below 1 mDa under m/z 200 (the same rule the page always had).
fn window_ppm(mz: f64, ppm: f64) -> f64 {
    (mz * ppm * 1e-6).max(if mz < 200.0 { 0.001 } else { 0.0 })
}

fn column<'a>(items: &'a Value, key: &str) -> Result<&'a [Value]> {
    items
        .get(key)
        .map(Value::arr)
        .ok_or_else(|| Error::new("err.reflist.request"))
}

/// `{mz, pol: "+"|"-"|"", unit: "ppm"|"da", tol, items: {mz:[..], pol:[-1|0|1,..], group:[..]}}` -> `{schema, unit, tol, hits:[{i, diff, err}]}`.
/// `i` is the position of the entry in `items`; `diff` = mz - entry m/z (Da), `err` in ppm. Sorted by |diff| (Da) or |err| (ppm), ties in list order.
/// An entry with polarity 0 lies in both polarities and is reported once; entries with the same `group` are the same ion and are reported once.
pub fn route(body: &str) -> Result<String> {
    let bad = || Error::new("err.reflist.request");
    let req = parse(body.as_bytes()).map_err(|_| bad())?;
    let mz = req
        .get("mz")
        .and_then(Value::f64)
        .filter(|v| v.is_finite())
        .ok_or_else(bad)?;
    let da = match req.get("unit").and_then(Value::str) {
        Some("da") => true,
        Some("ppm") | None => false,
        _ => return Err(bad()),
    };
    let tol = req
        .get("tol")
        .and_then(Value::f64)
        .filter(|t| t.is_finite() && *t > 0.0)
        .unwrap_or(if da { 0.5 } else { 5.0 });
    let pol = req.get("pol").and_then(Value::str).unwrap_or("");
    let it = req.get("items").ok_or_else(bad)?;
    let (m, p, g) = (column(it, "mz")?, column(it, "pol")?, column(it, "group")?);
    if m.len() != p.len() || m.len() != g.len() {
        return Err(bad());
    }
    let ion = |i: usize| {
        (
            m[i].f64().unwrap_or(f64::NAN),
            p[i].f64().unwrap_or(0.0),
            g[i].f64().unwrap_or(i as f64),
        )
    };
    let t = if da { tol } else { window_ppm(mz, tol) };
    // one array per polarity, sorted by m/z (stable: ties keep the list order); no polarity asked = positive then negative
    let sides: &[f64] = match pol {
        "+" => &[1.0],
        "-" => &[-1.0],
        _ => &[1.0, -1.0],
    };
    let (mut hits, mut seen) = (
        Vec::<(usize, f64, f64)>::new(),
        std::collections::HashSet::new(),
    );
    for side in sides {
        let mut a: Vec<usize> = (0..m.len()).filter(|&i| ion(i).1 != -side).collect();
        crate::sortc::slice_by(&mut a, |&x, &y| ion(x).0.total_cmp(&ion(y).0));
        let from = a.partition_point(|&i| ion(i).0 < mz - t);
        for &i in a[from..].iter().take_while(|&&i| ion(i).0 <= mz + t) {
            if seen.insert(ion(i).2.to_bits()) {
                let diff = mz - ion(i).0;
                hits.push((i, diff, diff / ion(i).0 * 1e6));
            }
        }
    }
    crate::sortc::slice_by(&mut hits, |a, b| {
        let (x, y) = if da {
            (a.1.abs(), b.1.abs())
        } else {
            (a.2.abs(), b.2.abs())
        };
        x.total_cmp(&y)
    });
    let hits = hits
        .into_iter()
        .map(|(i, d, e)| {
            Obj::new()
                .put("i", int(i as i64))
                .put("diff", num(d))
                .put("err", num(e))
                .done()
        })
        .collect();
    Ok(write(
        &Obj::new()
            .put("schema", text("reflist-match/1"))
            .put("unit", text(if da { "da" } else { "ppm" }))
            .n("tol", tol)
            .put("hits", Value::Arr(hits))
            .done(),
    ))
}
