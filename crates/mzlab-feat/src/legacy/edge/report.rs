//! `origin_evidence` (`TP_Mine/py/tpmine/hr/edge_evidence.py::report_evidence`): the evidence, without a label, for the report «where does this ion
//! come from?» of the HR world. X = the ion, P = the candidate parent; in order the m/z step, the co-elution, the MS2 of the parent, the shared
//! skeleton, the ratio across the samples and the typical loss of an adduct. Stateless: the report comes in the payload `{report, pol}`.

use super::*;
use crate::legacy::Res;
use mzlab_core::json::{self, Value};

/// `_spec` of the report: `{mz, it}` or `[mz, it]`; None without peaks.
fn spec_of(v: Option<&Value>) -> Option<Spec> {
    let v = v.filter(|v| !v.is_null())?;
    let (a, i) = match v {
        Value::Arr(_) => (v.at(0)?, v.at(1)?),
        _ => (v.get("mz")?, v.get("it")?),
    };
    if a.is_null() || i.is_null() {
        return None;
    }
    let num = |x: &Value| {
        x.arr()
            .iter()
            .map(|y| y.f64().unwrap_or(f64::NAN))
            .collect::<Vec<f64>>()
    };
    spec(&num(a), &num(i))
}

/// Python's `str()` of the number of samples (`None` when absent).
fn py_int(v: Option<&Value>) -> String {
    match v.and_then(Value::f64) {
        Some(x) if x.fract() == 0.0 && x.abs() < 1e16 => format!("{}", x as i64),
        Some(x) => format!("{x:?}"),
        None => "None".into(),
    }
}

fn push(
    out: &mut Vec<(&'static str, String, &'static str)>,
    key: &'static str,
    text: String,
    ok: bool,
) {
    out.push((key, text, if ok { "evidence" } else { "against" }));
}

/// The MS2 part: the ion among the fragments of the parent and the shared skeleton. Returns the MS2 of the ion (for the typical loss).
fn ms2_evidence(
    rep: &Value,
    mz: f64,
    par: f64,
    ev: &mut Vec<(&'static str, String, &'static str)>,
) -> Option<Spec> {
    let pk = rep.get("ms2_peaks");
    let (sp, sx) = (
        spec_of(pk.and_then(|p| p.get("p"))),
        spec_of(pk.and_then(|p| p.get("x"))),
    );
    if let (Some(p), true) = (&sp, mz < par) {
        let hit = near(p, mz, Some(par));
        let text = match hit {
            Some((h, rel)) => format!("m/z {mz:.4} is a fragment in the MS2 of {par:.4} ({:+.1} ppm, {rel:.0} % of the base peak)", (h - mz) / mz * 1e6),
            None => format!("m/z {mz:.4} is not among the fragments of the MS2 of {par:.4}"),
        };
        push(ev, "frag_ms2", text, hit.is_some());
    }
    if let (Some(p), Some(x)) = (&sp, &sx) {
        let (n, shared, frac) = if mz < par {
            skeleton(x, p, mz, par)
        } else {
            skeleton(p, x, par, mz)
        };
        if let Some(f) = frac {
            let text = format!("{shared} of {n} fragments of the lighter ion also in the heavier (same m/z or shifted by Δm/z)");
            push(ev, "skeleton", text, f >= SKELETON_MIN);
        }
    }
    sx
}

/// The evidence of one report (`report_evidence(rep, pol)`): (key, text, strength).
pub fn report_evidence(rep: &Value, pol: i32) -> Res<Vec<(&'static str, String, &'static str)>> {
    let num = |k: &str| {
        rep.get(k)
            .and_then(Value::f64)
            .ok_or_else(|| format!("report without {k}"))
    };
    let (mz, par) = (num("mz")?, num("parent_mz")?);
    let (lo, up) = if mz < par { (mz, par) } else { (par, mz) };
    let t = Iimn::new();
    let rels = relations_of(&t, lo, up, pol);
    let mut ev = Vec::new();
    for r in &rels {
        let text = format!(
            "Δm/z {:.4} = {} {} ({:+.1} ppm)",
            up - lo,
            r.role,
            r.name,
            r.ppm
        );
        ev.push(("dm", text, "info"));
    }
    if let Some(d) = rep
        .get("profile")
        .and_then(|p| p.get("apex_diff_s"))
        .and_then(Value::f64)
        .filter(|d| d.is_finite())
    {
        let within = d.abs() <= COELUTION_MIN * 60.0;
        let text = format!(
            "apex difference {:.1} s ({} {COELUTION_MIN} min)",
            d.abs(),
            if within { "within" } else { "beyond" }
        );
        push(&mut ev, "coelution", text, within);
    }
    let sx = ms2_evidence(rep, mz, par, &mut ev);
    let rc = rep.get("ratio_constancy");
    if let Some(cv) = rc
        .and_then(|r| r.get("cv"))
        .and_then(Value::f64)
        .filter(|c| c.is_finite())
        .map(|c| c.min(3.0))
    {
        let stable = cv <= CV_STABLE;
        let text = format!(
            "ratio ion/parent {}: CV {:.0} % over {} samples",
            if stable { "stable" } else { "not stable" },
            100.0 * cv,
            py_int(rc.and_then(|r| r.get("k")))
        );
        push(&mut ev, "ratio", text, stable);
    }
    for r in rels
        .iter()
        .filter(|r| r.role == "adduct" && r.name == "[M+NH4]+")
    {
        if let (Some(x), true) = (&sx, mz > par) {
            let hit = near(x, mz - t.nh3, Some(mz)).is_some();
            let text = format!(
                "typical loss of {} (-{:.3}, NH3) {} in the MS2 of the ion",
                r.name,
                t.nh3,
                if hit { "seen" } else { "not seen" }
            );
            push(&mut ev, "typical_loss", text, hit);
        }
    }
    Ok(ev)
}

/// `api.origin_evidence`: `{report, pol}` → `[{key, text, strength}]`.
pub fn origin_evidence(payload_json: &str) -> Res<String> {
    let p = json::parse(payload_json.as_bytes()).map_err(|e| e.to_string())?;
    let pol = p
        .get("pol")
        .and_then(Value::f64)
        .filter(|x| *x != 0.0)
        .unwrap_or(1.0); // `int(d.get("pol") or 1)`
    let pol = if pol.trunc() >= 0.0 { 1 } else { -1 };
    let ev = report_evidence(p.get("report").ok_or("payload without report")?, pol)?;
    let mut o = Out::default();
    o.raw("[");
    for (i, (k, text, s)) in ev.iter().enumerate() {
        o.raw(if i > 0 { ",{\"key\":" } else { "{\"key\":" }).str(k);
        o.raw(",\"text\":")
            .str(text)
            .raw(",\"strength\":")
            .str(s)
            .raw("}");
    }
    o.raw("]");
    Ok(o.0)
}
