//! The ion forms of each polarity, the relations between two ions and the shape of two profiles, and `polarity_pairs`.

use super::*;
use crate::sortc::SortC;

/// The adducts of the neutral M: (name, m/z shift, multiplier, base ion), `FORMS[pol]`.
pub(super) type Form = (&'static str, f64, f64, bool);

pub(super) fn forms(t: &Iimn, pol: i32) -> Vec<Form> {
    let p = t.proton;
    let a = |n: &str| {
        t.adducts_of(pol)
            .iter()
            .find(|x| x.0 == n)
            .map(|x| x.1)
            .unwrap_or(0.0)
    };
    if pol > 0 {
        vec![
            ("[M+H]+", p, 1.0, true),
            ("[M+NH4]+", a("[M+NH4]+") + p, 1.0, false),
            ("[M+Na]+", a("[M+Na]+") + p, 1.0, false),
            ("[M+K]+", a("[M+K]+") + p, 1.0, false),
            ("[M+H+ACN]+", a("[M+H+ACN]+") + p, 1.0, false),
            ("[2M+H]+", p, 2.0, false),
        ]
    } else {
        vec![
            ("[M-H]-", -p, 1.0, true),
            ("[M+FA-H]-", a("[M+FA-H]-") - p, 1.0, false),
            ("[M+Cl]-", a("[M+Cl]-") - p, 1.0, false),
            ("[M+Na-2H]-", a("[M+Na-2H]-") - p, 1.0, false),
            ("[2M-H]-", -p, 2.0, false),
        ]
    }
}

pub(super) fn base(pol: i32) -> &'static str {
    if pol > 0 {
        "[M+H]+"
    } else {
        "[M-H]-"
    }
}

/// `_cross_pairs`: differences between two adducts that are not the base ion.
pub(super) fn cross_pairs(f: &[Form]) -> Vec<(String, f64)> {
    let ad: Vec<&Form> = f.iter().filter(|x| !x.3 && x.2 == 1.0).collect();
    let mut out = Vec::new();
    for a in &ad {
        for b in &ad {
            if a.0 != b.0 {
                out.push((format!("{} vs {}", b.0, a.0), b.1 - a.1));
            }
        }
    }
    out
}

/// `shifts`: every signed difference a relation can give.
pub(super) fn shifts(t: &Iimn, f: &[Form], pol: i32) -> Vec<f64> {
    let ad: Vec<f64> = t.adducts_of(pol).iter().map(|x| x.1).collect();
    let mut out: Vec<f64> = ad.iter().copied().chain(ad.iter().map(|v| -v)).collect();
    out.extend(cross_pairs(f).into_iter().map(|x| x.1));
    for v in edge::ISOTOPES
        .iter()
        .map(|x| x.1)
        .chain(t.losses.iter().map(|x| x.1))
    {
        out.push(v);
        out.push(-v);
    }
    out.sortc_by(f64::total_cmp);
    out.dedup();
    out
}

/// One relation of an edge: role, name, label, expected step, error in ppm.
pub(super) struct Relation {
    pub(super) role: &'static str,
    pub(super) name: String,
    pub(super) label: String,
    pub(super) expected: f64,
}

/// `pair_relations`.
pub(super) fn pair_relations(
    t: &Iimn,
    f: &[Form],
    delta: f64,
    mz_ref: f64,
    pol: i32,
    ppm: f64,
    abs_da: f64,
) -> Vec<Relation> {
    let mut out: Vec<Relation> = relations(t, delta, mz_ref, ppm, abs_da, pol)
        .into_iter()
        .map(|r| {
            let label = match r.role {
                "adduct" => format!("{} / {}", r.name, base(pol)),
                "isotope" => format!("{} isotope", r.name),
                _ => format!("-{} in source", r.name),
            };
            Relation {
                role: r.role,
                name: r.name,
                label,
                expected: r.delta,
            }
        })
        .collect();
    let tol = (mz_ref * ppm * 1e-6).max(abs_da);
    for (label, d) in cross_pairs(f) {
        if (delta - d).abs() <= tol {
            out.push(Relation {
                role: "adduct",
                name: label.clone(),
                label,
                expected: d,
            });
        }
    }
    out
}

/// `_shape_r`: Pearson r over the samples where both are detected, the number of them and which.
pub(super) fn shape_r(a: &[f64], b: &[f64], min_common: usize) -> (Option<f64>, usize, Vec<usize>) {
    let k: Vec<usize> = (0..a.len()).filter(|&s| a[s] > 0.0 && b[s] > 0.0).collect();
    if k.len() < min_common {
        return (None, k.len(), k);
    }
    let x: Vec<f64> = k.iter().map(|&s| a[s]).collect();
    let y: Vec<f64> = k.iter().map(|&s| b[s]).collect();
    let (sx, sy) = (std(&x, 0), std(&y, 0));
    if sx <= 0.0 || sy <= 0.0 {
        return (None, k.len(), k);
    }
    let (mx, my) = (mean(&x), mean(&y));
    let p: Vec<f64> = x.iter().zip(&y).map(|(u, v)| (u - mx) * (v - my)).collect();
    (Some(mean(&p) / (sx * sy)), k.len(), k)
}

/// `api.polarity_pairs(payload)`: features of the positive and negative tables with the same neutral mass under the usual adduct pairs.
pub fn polarity_pairs(payload_json: &str) -> Res<String> {
    let p = json::parse(payload_json.as_bytes()).map_err(|e| e.to_string())?;
    let ppm = p.get("ppm").and_then(Value::f64).unwrap_or(5.0);
    let rt_min = p.get("rt_min").and_then(Value::f64).unwrap_or(0.15);
    let t = Iimn::new();
    let (fp, fneg) = (forms(&t, 1), forms(&t, -1));
    let form = |f: &[Form], n: &str| *f.iter().find(|x| x.0 == n).unwrap();
    let list = |k: &str| p.get(k).map(Value::arr).unwrap_or(&[]).to_vec();
    let (pos, neg) = (list("pos"), list("neg"));
    let num = |v: &Value, k: &str| v.get(k).and_then(Value::f64).unwrap_or(f64::NAN);
    let mut rows: Vec<(f64, &str, &Value)> = Vec::new();
    for fe in &neg {
        for name in ["[M-H]-", "[M+FA-H]-", "[M+Cl]-"] {
            let (_, sh, mult, _) = form(&fneg, name);
            rows.push(((num(fe, "mz") - sh) / mult, name, fe));
        }
    }
    rows.sortc_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(&Value, f64, Out)> = Vec::new();
    for fe in &pos {
        for name in ["[M+H]+", "[M+Na]+", "[M+NH4]+"] {
            let (_, sh, mult, _) = form(&fp, name);
            let m = (num(fe, "mz") - sh) / mult;
            let tol = m * ppm * 1e-6;
            let a = rows.partition_point(|r| r.0 < m - tol);
            let b = rows.partition_point(|r| r.0 <= m + tol);
            for &(mn, nname, nf) in rows.get(a..b.max(a)).unwrap_or(&[]) {
                if (num(nf, "rt") - num(fe, "rt")).abs() > rt_min {
                    continue;
                }
                let ppm_v = json::round((mn - m) / m * 1e6, 2);
                let mut o = Out::default();
                o.raw("{\"pos\":");
                write_value(fe.get("k").unwrap_or(&Value::Null), &mut o);
                o.raw(",\"neg\":");
                write_value(nf.get("k").unwrap_or(&Value::Null), &mut o);
                o.raw(",\"pos_form\":")
                    .str(name)
                    .raw(",\"neg_form\":")
                    .str(nname);
                o.raw(",\"mass\":")
                    .num(json::round(m, 5))
                    .raw(",\"ppm\":")
                    .num(ppm_v);
                o.raw(",\"drt\":")
                    .num(json::round(num(nf, "rt") - num(fe, "rt"), 3))
                    .raw("}");
                out.push((fe.get("k").unwrap_or(&Value::Null), ppm_v.abs(), o));
            }
        }
    }
    out.sortc_by(|a, b| cmp_value(a.0, b.0).then(a.1.total_cmp(&b.1)));
    let mut o = Out::default();
    o.raw("[");
    for (k, x) in out.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).raw(&x.2 .0);
    }
    o.raw("]");
    Ok(o.0)
}
