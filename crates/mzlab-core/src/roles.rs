//! Hypotheses on the relation between an ion X and a candidate precursor P by the m/z difference alone, and the isotope envelope check:
//! ports of `ionfamily.annotate_roles` and `ionfamily.isotope_check`. Every role is a hint, never a conclusion.

use crate::elements::{fmt, mass, parse_formula, Formula};
use crate::error::Result;
use crate::json::Value;
use crate::jsonb::{text, Obj};
use crate::rois::Window;

pub const PROTON: f64 = 1.00727646688;
const ELECTRON: f64 = 0.000548579909;
const ISO_STEP: f64 = 1.003355;
pub(crate) const NEUTRAL_LOSSES: [&str; 20] = [
    "H2O", "NH3", "CO", "CO2", "CH2O", "CH4O", "C2H2O", "HCOOH", "C2H4", "C3H6", "HCN", "HF",
    "HCl", "HBr", "SO2", "SO3", "H2S", "CH3", "NO2", "C6H6",
];
/// Adducts of the neutral M: name, shift of the m/z from the neutral mass, multiplier of M.
pub(crate) const ADDUCTS: [(&str, f64, f64); 7] = [
    ("[M+H]+", PROTON, 1.0),
    ("[M+NH4]+", 18.0338262, 1.0),
    ("[M+Na]+", 22.989769282 - ELECTRON, 1.0),
    ("[M+K]+", 38.9637064 - ELECTRON, 1.0),
    ("[M+H+ACN]+", PROTON + 41.0265491, 1.0),
    ("[2M+H]+", PROTON, 2.0),
    ("[2M+Na]+", 22.989769282 - ELECTRON, 2.0),
];

/// The table of neutral losses with their parsed formulas and masses (built once per report).
pub struct Losses(Vec<(&'static str, Formula, f64)>);

impl Losses {
    pub fn new() -> Losses {
        Losses(
            NEUTRAL_LOSSES
                .iter()
                .map(|&n| {
                    let f = parse_formula(n).unwrap_or_default();
                    let m = mass(&f);
                    (n, f, m)
                })
                .collect(),
        )
    }

    /// Single losses and combinations of two of the table whose mass is within `tol` of `delta`: (name, mass), best first.
    fn candidates(&self, delta: f64, tol: f64) -> Vec<(String, f64)> {
        let mut out: Vec<(String, f64)> = Vec::new();
        let t = &self.0;
        for a in t {
            if (a.2 - delta).abs() <= tol {
                out.push((a.0.to_string(), a.2));
            }
        }
        for i in 0..t.len() {
            for j in i..t.len() {
                let m = t[i].2 + t[j].2;
                if (m - delta).abs() <= tol {
                    out.push((format!("{}+{}", t[i].0, t[j].0), m));
                }
            }
        }
        out.sort_by(|a, b| {
            (delta - a.1)
                .abs()
                .total_cmp(&(delta - b.1).abs())
                .then(a.0.matches('+').count().cmp(&b.0.matches('+').count()))
        });
        out
    }

    /// The fragment formula `parent - losses` if it is a chemical subset of the parent (H may be exchanged by one).
    fn subformula(&self, parent: &Formula, parts: &[&str]) -> (bool, Formula) {
        let mut frag: Vec<(String, i64)> =
            parent.iter().map(|(e, n)| (e.clone(), *n as i64)).collect();
        for part in parts {
            for (el, k) in parse_formula(part).unwrap_or_default() {
                match frag.iter_mut().find(|(e, _)| *e == el) {
                    Some(x) => x.1 -= k as i64,
                    None => frag.push((el, -(k as i64))),
                }
            }
        }
        let ok = frag.iter().all(|(e, v)| e == "H" || *v >= 0)
            && frag.iter().find(|(e, _)| e == "H").map_or(0, |x| x.1) >= -1;
        (
            ok,
            frag.into_iter()
                .filter(|(_, v)| *v > 0)
                .map(|(e, v)| (e, v as u64))
                .collect(),
        )
    }
}

/// Observed (M+1)/M and (M+2)/M of the ion at `mz` from the areas of the ROIs at +1 and +2 in the scans `a..b` of the window.
pub fn isotope_check(w: &Window, mz: f64, a: usize, b: usize) -> Value {
    let area = |m: f64| window_area(w, m, a, b);
    let (a0, a1, a2) = (area(mz), area(mz + ISO_STEP), area(mz + 2.00671));
    let r = |a: f64| if a0 > 0.0 { a / a0 } else { f64::NAN };
    Obj::new().n("obs_m1", r(a1)).n("obs_m2", r(a2)).done()
}

impl Losses {
    /// `annotate_roles(mz_x, mz_p, formula_p)` with `tol = 0.5`.
    pub fn annotate(&self, mz_x: f64, mz_p: f64, formula_p: Option<&str>) -> Result<Value> {
        let tol = 0.5;
        let fp = formula_p.map(parse_formula).transpose()?;
        let mut out = Vec::new();
        let d = mz_x - mz_p;
        for k in 1..=3 {
            let e = k as f64 * ISO_STEP;
            if (d - e).abs() <= tol {
                out.push(
                    Obj::new()
                        .put("role", text("isotope"))
                        .put("label", text(&format!("M+{k} of P")))
                        .put("label_key", text("of.role.isoP"))
                        .put("params", Obj::new().i("k", k).done())
                        .n("delta_obs", d)
                        .n("delta_exp", e)
                        .n("err", d - e)
                        .done(),
                );
            }
            if (-d - e).abs() <= tol {
                out.push(
                    Obj::new()
                        .put("role", text("isotope"))
                        .put(
                            "label",
                            text(&format!("X is M+{k} (P is the isotope of X)")),
                        )
                        .put("label_key", text("of.role.isoX"))
                        .put("params", Obj::new().i("k", k).done())
                        .n("delta_obs", d)
                        .n("delta_exp", -e)
                        .n("err", d + e)
                        .done(),
                );
            }
        }
        let m = mz_p - PROTON;
        for (name, shift, mult) in ADDUCTS.iter().skip(1) {
            let exp = mult * m + shift - (m + PROTON);
            if (d - exp).abs() <= tol {
                let role = if *mult == 2.0 { "dimer" } else { "adduct" };
                out.push(
                    Obj::new()
                        .put("role", text(role))
                        .put("label", text(name))
                        .n("delta_obs", d)
                        .n("delta_exp", exp)
                        .n("err", d - exp)
                        .done(),
                );
            }
        }
        if d < -0.5 {
            for (loss, lm) in self.candidates(-d, tol) {
                let mut row = Obj::new()
                    .put("role", text("fragment_candidate"))
                    .put("label", text(&format!("P - {loss}")))
                    .n("delta_obs", d)
                    .n("delta_exp", -lm)
                    .n("err", d + lm);
                if let Some(f) = &fp {
                    let parts: Vec<&str> = loss.split('+').collect();
                    let (ok, frag) = self.subformula(f, &parts);
                    row = row
                        .b("formula_ok", ok)
                        .put("fragment_formula", text(&fmt(&frag)));
                }
                out.push(row.done());
            }
        }
        Ok(Value::Arr(out))
    }
}

/// Area of the ROI nearest to `m` (within 0.5) in the scans `a..b` of the window; 0 when there is none.
fn window_area(w: &Window, m: f64, a: usize, b: usize) -> f64 {
    if w.mz.is_empty() {
        return 0.0;
    }
    let j = crate::tracestat::argmin(&w.mz.iter().map(|v| (v - m).abs()).collect::<Vec<_>>());
    if (w.mz[j] - m).abs() > 0.5 {
        return 0.0;
    }
    let row = &w.x[j * w.n_scans..(j + 1) * w.n_scans];
    let (a, b) = (a.min(row.len()), b.min(row.len()));
    if a < b {
        crate::stats::psum(&row[a..b])
    } else {
        0.0
    }
}

impl Default for Losses {
    fn default() -> Self {
        Losses::new()
    }
}
