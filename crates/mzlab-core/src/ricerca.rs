//! Search box, numeric part (`ricerca/1`): what a typed number is compatible with. A number x is read as a neutral loss (Δm),
//! as a shift of an adduct, or as the m/z of a known contaminant. The tables stay in the page (losses, adducts, contaminant list);
//! this module owns the tolerance, the order and the numbers shown. Only compatibilities are returned, never verdicts.

use crate::elements::{mass, parse_formula};
use crate::error::{Error, Result};
use crate::json::{parse, Value};
use crate::jsonb::{int, num, opt, text, write, Obj};
use crate::sortc;

/// Unit resolution: a number is compatible within half a dalton, with no decimals shown.
const LR_TOL: f64 = 0.5;
/// Most contaminants listed (the list holds over a thousand entries).
const MAX_CONT: usize = 8;
const ELECTRON: f64 = 0.000548579909;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Loss,
    Adduct,
    Contaminant,
}

struct Hit {
    kind: Kind,
    id: String,
    reference: f64,
    delta: f64,
}

fn field<'a>(v: &'a Value, k: &str) -> Result<&'a Value> {
    v.get(k)
        .ok_or_else(|| Error::new("err.search.request").with("field", k))
}

fn formula_mass(f: &str) -> Result<f64> {
    if f.is_empty() {
        return Ok(0.0);
    }
    Ok(mass(&parse_formula(f)?))
}

/// `{x, hr, tol_mda, tol_ppm, pol, losses:[{id,f}], adducts:[{id,k,add,sub,z}], contam:[{id,mz,pol}]}` -> `{x, hr, hits, more}`.
/// `hr` false: tolerance ±0.5 and no deltas in the answer. `hr` true: `tol_mda` for losses and adduct shifts, `tol_ppm` for contaminants.
/// `pol` ("+", "-" or "") keeps the adducts and contaminants of that polarity; the losses have no polarity.
pub fn match_number(req: &str) -> Result<String> {
    let r = parse(req.as_bytes())?;
    let x = field(&r, "x")?
        .f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| Error::new("err.search.request").with("field", "x"))?;
    let hr = r.get("hr").and_then(Value::bool).unwrap_or(false);
    let tol_mda = r.get("tol_mda").and_then(Value::f64).unwrap_or(3.0);
    let tol_ppm = r.get("tol_ppm").and_then(Value::f64).unwrap_or(5.0);
    let pol = match r.get("pol").and_then(Value::str) {
        Some("+") => 1,
        Some("-") => -1,
        _ => 0,
    };
    let tol_dm = if hr { tol_mda / 1000.0 } else { LR_TOL };
    let mut hits: Vec<Hit> = Vec::new();

    for l in field(&r, "losses")?.arr() {
        let (id, f) = (
            field(l, "id")?.str().unwrap_or(""),
            field(l, "f")?.str().unwrap_or(""),
        );
        let m = formula_mass(f)?;
        if m > 0.0 && (x - m).abs() <= tol_dm {
            hits.push(Hit {
                kind: Kind::Loss,
                id: id.to_string(),
                reference: m,
                delta: x - m,
            });
        }
    }
    for a in field(&r, "adducts")?.arr() {
        let k = a.get("k").and_then(Value::f64).unwrap_or(1.0);
        let z = field(a, "z")?.f64().unwrap_or(1.0);
        // only the single-charge adducts of one molecule are a shift to add to a neutral mass
        if k != 1.0 || z.abs() != 1.0 || (pol != 0 && (z > 0.0) != (pol > 0)) {
            continue;
        }
        let get = |name: &str| a.get(name).and_then(Value::str).unwrap_or("");
        // [M]+ and [M]- add nothing: not a shift
        if get("add").is_empty() && get("sub").is_empty() {
            continue;
        }
        let shift = formula_mass(get("add"))? - formula_mass(get("sub"))? - z * ELECTRON;
        if (x - shift).abs() <= tol_dm {
            hits.push(Hit {
                kind: Kind::Adduct,
                id: field(a, "id")?.str().unwrap_or("").to_string(),
                reference: shift,
                delta: x - shift,
            });
        }
    }
    for c in field(&r, "contam")?.arr() {
        let mz = field(c, "mz")?.f64().unwrap_or(f64::NAN);
        let p = c.get("pol").and_then(Value::f64).unwrap_or(0.0);
        if !mz.is_finite() || (pol != 0 && p != 0.0 && (p > 0.0) != (pol > 0)) {
            continue;
        }
        let tol = if hr { x.abs() * tol_ppm * 1e-6 } else { LR_TOL };
        if (x - mz).abs() <= tol {
            hits.push(Hit {
                kind: Kind::Contaminant,
                id: field(c, "id")?.str().unwrap_or("").to_string(),
                reference: mz,
                delta: x - mz,
            });
        }
    }

    let rank = |k: Kind| match k {
        Kind::Loss => 0,
        Kind::Adduct => 1,
        Kind::Contaminant => 2,
    };
    let mut order: Vec<usize> = (0..hits.len()).collect();
    sortc::by(&mut order, |&i, &j| {
        let (a, b) = (&hits[i], &hits[j]);
        rank(a.kind)
            .cmp(&rank(b.kind))
            .then_with(|| a.delta.abs().total_cmp(&b.delta.abs()))
            .then_with(|| i.cmp(&j))
    });

    let mut shown = Vec::new();
    let (mut seen_cont, mut more) = (0usize, 0i64);
    for i in order {
        let h = &hits[i];
        if h.kind == Kind::Contaminant {
            seen_cont += 1;
            if seen_cont > MAX_CONT {
                more += 1;
                continue;
            }
        }
        let name = match h.kind {
            Kind::Loss => "loss",
            Kind::Adduct => "adduct",
            Kind::Contaminant => "contaminant",
        };
        // low resolution: the compatibility only, never decimals
        let o = Obj::new()
            .put("kind", text(name))
            .put("id", text(&h.id))
            .put("delta_da", opt(hr.then_some(h.delta)))
            .put("ppm", opt(hr.then_some(h.delta / h.reference * 1e6)));
        shown.push(o.done());
    }
    Ok(write(
        &Obj::new()
            .n("x", x)
            .b("hr", hr)
            .put("hits", Value::Arr(shown))
            .put("more", int(more))
            .put("tol", num(tol_dm))
            .done(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLES: &str = r#""losses":[{"id":"H2O","f":"H2O"},{"id":"NH3","f":"NH3"},{"id":"CO","f":"CO"}],
        "adducts":[{"id":"[M+H]+","k":1,"add":"H","z":1},{"id":"[M+NH4]+","k":1,"add":"NH4","z":1},{"id":"[M+Na]+","k":1,"add":"Na","z":1},
        {"id":"[M-H]-","k":1,"sub":"H","z":-1},{"id":"[2M+Na]+","k":2,"add":"Na","z":1},{"id":"[M]+","k":1,"add":"","z":1}],
        "contam":[{"id":"peg1","mz":63.04406,"pol":1},{"id":"x","mz":18.2,"pol":-1}]"#;

    fn ids(x: &str, extra: &str) -> Vec<(String, String)> {
        let out = match_number(&format!(r#"{{"x":{x},{extra}{TABLES}}}"#)).unwrap();
        let v = parse(out.as_bytes()).unwrap();
        v.get("hits")
            .unwrap()
            .arr()
            .iter()
            .map(|h| {
                (
                    h.get("kind").unwrap().str().unwrap().into(),
                    h.get("id").unwrap().str().unwrap().into(),
                )
            })
            .collect()
    }
    fn has(h: &[(String, String)], k: &str, id: &str) -> bool {
        h.iter().any(|(a, b)| a == k && b == id)
    }

    #[test]
    fn exact_masses_hr() {
        let hr = r#""hr":true,"tol_mda":3,"tol_ppm":5,"#;
        let h = ids("18.0106", hr);
        assert!(has(&h, "loss", "H2O") && !has(&h, "loss", "NH3"));
        assert!(!has(&h, "adduct", "[M+NH4]+"));
        assert!(has(&ids("22.9892", hr), "adduct", "[M+Na]+"));
        assert!(has(&ids("17.0265", hr), "loss", "NH3"));
        assert!(has(&ids("1.0073", hr), "adduct", "[M+H]+"));
    }

    #[test]
    fn unit_resolution_is_half_a_dalton_without_decimals() {
        let h = ids("18", "");
        assert!(has(&h, "loss", "H2O") && has(&h, "adduct", "[M+NH4]+"));
        assert!(!has(&h, "loss", "NH3") && !has(&h, "loss", "CO"));
        let out = match_number(&format!(r#"{{"x":18,{TABLES}}}"#)).unwrap();
        assert!(out.contains("\"delta_da\":null") && out.contains("\"ppm\":null"));
    }

    #[test]
    fn polarity_and_only_single_adducts() {
        let h = ids("22.99", r#""pol":"-","#);
        assert!(!has(&h, "adduct", "[M+Na]+"));
        assert!(
            has(&ids("22.99", ""), "adduct", "[M+Na]+")
                && !has(&ids("22.99", ""), "adduct", "[2M+Na]+")
        );
        assert!(!ids("0", "").iter().any(|(k, _)| k == "adduct"));
    }

    #[test]
    fn contaminant_and_order() {
        let hr = r#""hr":true,"tol_ppm":5,"#;
        assert!(has(&ids("63.0441", hr), "contaminant", "peg1"));
        assert!(!has(&ids("63.0500", hr), "contaminant", "peg1"));
        let h = ids("63.2", "");
        assert!(has(&h, "contaminant", "peg1"));
    }

    #[test]
    fn bad_request_is_an_error_with_a_key() {
        assert_eq!(
            match_number(r#"{"losses":[]}"#).unwrap_err().key,
            "err.search.request"
        );
    }
}
