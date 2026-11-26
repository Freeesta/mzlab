//! `raw isotopes` (`/api/rawiso`, `docs/agenti/raw-access.md`): do M+1 and M+2 fit a formula, within counting error? Net ion counts of the bins M, M+1 and
//! M+2 over the peak region of M (the sides subtracted), the observed ratios with their delta-method sigma, the expected pattern of `ion pattern` with z
//! scores, or without a formula the carbon range and the M+2 hint. The CLI, the app (`rawinfo::handle`) and the site (the lazy `.wasm` `rawiso`) answer
//! through here, so the bytes are one.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::ionpattern::ion_counts;
use crate::json::round;
use crate::mzcal::{Offset, Spec};
use crate::rawxic::{self, PROBE};
use crate::routes::{Query, Routes};

/// Defaults of the verdicts (`--set`): the smallest net count that counts, the |z| under which a ratio is consistent and from which it is not, the
/// carbon-13 share per carbon (%), the half width of the carbon range in sigma, the M+2 hint (a share of R1 and sigma), the least correlation of M+1 with M.
pub const MIN_COUNTS: f64 = 10.0;
pub const Z_OK: f64 = 2.0;
pub const Z_BAD: f64 = 3.0;
pub const C13_PCT: f64 = 1.1;
pub const CARBON_K: f64 = 2.0;
pub const M2_RATIO: f64 = 0.2;
pub const M2_K: f64 = 3.0;
pub const MIN_R: f64 = 0.8;
/// The adduct taken when a formula comes without one.
pub const DEFAULT_ADDUCT: &str = "[M+H]+";

/// The thresholds of the verdicts and the ones of the judge of M (`raw xic`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub min_counts: f64,
    pub z_ok: f64,
    pub z_bad: f64,
    pub c13_pct: f64,
    pub carbon_k: f64,
    pub m2_ratio: f64,
    pub m2_k: f64,
    pub min_r: f64,
    pub x: rawxic::Thresholds,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            min_counts: MIN_COUNTS,
            z_ok: Z_OK,
            z_bad: Z_BAD,
            c13_pct: C13_PCT,
            carbon_k: CARBON_K,
            m2_ratio: M2_RATIO,
            m2_k: M2_K,
            min_r: MIN_R,
            x: rawxic::Thresholds::default(),
        }
    }
}

fn bad(name: &str, v: &str) -> Error {
    Error::new("err.raw.isotopes.param")
        .with("name", name.to_string())
        .with("value", v.to_string())
}

fn num(q: &Query, k: &str) -> Result<Option<f64>> {
    match q.some(k) {
        None => Ok(None),
        Some(v) => v
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|x| x.is_finite())
            .map(Some)
            .ok_or_else(|| bad(k, v)),
    }
}

fn range(q: &Query, k: &str, d: f64, lo: f64, hi: f64) -> Result<f64> {
    match num(q, k)? {
        None => Ok(d),
        Some(x) if x >= lo && x <= hi => Ok(x),
        Some(x) => Err(bad(k, &x.to_string())),
    }
}

fn count(q: &Query, k: &str, d: usize) -> Result<usize> {
    match num(q, k)? {
        None => Ok(d),
        Some(x) if x >= 1.0 && x <= 1e6 && x.fract() == 0.0 => Ok(x as usize),
        Some(x) => Err(bad(k, &x.to_string())),
    }
}

/// `rt=A,B`: two numbers, the first under the second.
fn window_of(q: &Query) -> Result<(f64, f64)> {
    let rt = q.some("rt").ok_or_else(|| bad("rt", ""))?;
    let w: Vec<f64> = rt
        .split(',')
        .filter_map(|t| t.trim().parse::<f64>().ok().filter(|x| x.is_finite()))
        .collect();
    match w[..] {
        [a, b] if a < b && rt.split(',').count() == 2 => Ok((a, b)),
        _ => Err(bad("rt", rt)),
    }
}

/// The formula and the adduct (the default one when a formula comes without): an unknown adduct or element is refused here, before any file is opened, and a
/// charge other than 1 has no M+1 at +1 Da.
fn formula_of(q: &Query) -> Result<(Option<String>, Option<String>)> {
    let formula = q
        .some("formula")
        .filter(|f| !f.is_empty())
        .map(str::to_string);
    if formula.is_none() && q.some("adduct").is_some() {
        return Err(Error::new("err.raw.isotopes.adductNeedsFormula"));
    }
    let adduct = formula.as_ref().map(|_| {
        q.some("adduct")
            .filter(|a| !a.is_empty())
            .unwrap_or(DEFAULT_ADDUCT)
            .to_string()
    });
    if let (Some(f), Some(a)) = (&formula, &adduct) {
        let (_, z) = ion_counts(f, a)?;
        if z.abs() != 1 {
            return Err(Error::new("err.raw.isotopes.charge")
                .with("adduct", a.as_str())
                .with("z", i64::from(z)));
        }
    }
    Ok((formula, adduct))
}

/// The thresholds of the verdicts and of the judge of M, each inside its range.
fn params_of(q: &Query) -> Result<Params> {
    let d = Params::default();
    let r = |k: &str, d: f64, lo: f64, hi: f64| range(q, k, d, lo, hi);
    Ok(Params {
        min_counts: r("min_counts", d.min_counts, 0.0, 1e9)?,
        z_ok: r("z_ok", d.z_ok, 0.0, 1e3)?,
        z_bad: r("z_bad", d.z_bad, 0.0, 1e3)?,
        c13_pct: r("c13_pct", d.c13_pct, 1e-3, 100.0)?,
        carbon_k: r("carbon_k", d.carbon_k, 0.0, 1e3)?,
        m2_ratio: r("m2_ratio", d.m2_ratio, 0.0, 1e3)?,
        m2_k: r("m2_k", d.m2_k, 0.0, 1e3)?,
        min_r: r("min_r", d.min_r, -1.0, 1.0)?,
        x: rawxic::Thresholds {
            min_sn: r("min_sn", d.x.min_sn, 0.0, 1e6)?,
            min_points: count(q, "min_points", d.x.min_points)?,
            alpha: r("alpha", d.x.alpha, 1e-6, 0.5)?,
            side_min: r("side_min", d.x.side_min, 0.0, 1e4)?,
            gap_min: r("gap_min", d.x.gap_min, 0.0, 1e4)?,
        },
    })
}

/// The question of `raw isotopes` as the route reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Ask {
    pub mz: f64,
    pub rt: (f64, f64),
    pub level: u8,
    pub offset: Option<Spec>,
    pub formula: Option<String>,
    /// The adduct asked, else `DEFAULT_ADDUCT` when a formula is given.
    pub adduct: Option<String>,
    pub p: Params,
}

impl Ask {
    /// `mz=M&rt=A,B[&formula=F][&adduct=A][&bin=nominal][&ms_level=N][&mz_offset=TEXT][&min_counts=..&z_ok=..&z_bad=..&c13_pct=..&carbon_k=..&m2_ratio=..&m2_k=..
    /// &min_r=..&min_sn=..&min_points=..&alpha=..&side_min=..&gap_min=..]`; a value out of its range is refused, so is a formula the catalogue cannot read.
    pub fn of(q: &Query) -> Result<Ask> {
        let mz = match num(q, "mz")? {
            Some(x) if x > 0.0 => x,
            _ => return Err(bad("mz", q.some("mz").unwrap_or(""))),
        };
        let rt = window_of(q)?;
        if let Some(v) = q.some("bin").filter(|v| *v != "nominal") {
            return Err(bad("bin", v));
        }
        let level = match num(q, "ms_level")? {
            None => 1,
            Some(x) if (1.0..=10.0).contains(&x) && x.fract() == 0.0 => x as u8,
            Some(x) => return Err(bad("ms_level", &x.to_string())),
        };
        let (formula, adduct) = formula_of(q)?;
        Ok(Ask {
            mz,
            rt,
            level,
            offset: q.some("mz_offset").map(Spec::parse).transpose()?,
            formula,
            adduct,
            p: params_of(q)?,
        })
    }

    /// `z_ok` must be under `z_bad`: a ratio cannot be consistent and inconsistent at once.
    fn check(&self) -> Result<()> {
        if self.p.z_ok > self.p.z_bad {
            return Err(bad("z_ok", &self.p.z_ok.to_string()));
        }
        Ok(())
    }
}

/// What one sample answered: the members (no braces) and the `result` object.
pub struct IsoOut {
    pub members: String,
    pub result: String,
}

impl<R: ReadAt> Routes<R> {
    /// `/api/rawiso?mz=M&rt=A,B[&formula=..&adduct=..][&ms_level=..][&mz_offset=..][&min_counts=..]...`: the members of `raw-isotopes/1` for this sample and
    /// its `result`.
    pub fn raw_isotopes_route(&mut self, q: &Query) -> Result<Option<String>> {
        let o = self.raw_isotopes(&Ask::of(q)?)?;
        Ok(Some(format!("{{{},\"result\":{}}}", o.members, o.result)))
    }

    /// The answer of `ask` on this sample (its polarity `pol`).
    pub fn raw_isotopes(&mut self, ask: &Ask) -> Result<IsoOut> {
        ask.check()?;
        let keep = self.mz_off;
        let r = self.raw_isotopes_at(ask);
        self.mz_off = keep;
        r
    }

    fn raw_isotopes_at(&mut self, ask: &Ask) -> Result<IsoOut> {
        let off: Option<Offset> = ask
            .offset
            .as_ref()
            .map(|s| s.resolve(&mut self.idx, self.pol))
            .transpose()?;
        if let Some(o) = &off {
            self.mz_off = o.da;
        }
        let probe = self.probe(ask.level, PROBE)?;
        let quant = crate::rawinfo::quantum_of(Some(&probe)).filter(|q| q.accepted);
        let q = quant.map(|q| q.q);
        // the bins of `raw scan --bin nominal`: M is the whole mass nearest to --mz, M+1 and M+2 the next two
        let base = ask.mz.round();
        let centres = [base, base + 1.0, base + 2.0];
        let n = self.xic_len(ask.level);
        self.forget_sweep();
        let (rt, y): (Vec<f64>, Vec<Vec<f64>>) = if n == 0 {
            (Vec::new(), vec![Vec::new(); 3])
        } else {
            let m = self.sweep_matrix(&centres, &[0.5; 3], ask.level, None)?;
            (
                m.rt.iter().map(|&x| round(x, 4)).collect(),
                m.y.iter()
                    .map(|c| c.iter().map(|&v| round(v, 1)).collect())
                    .collect(),
            )
        };
        self.forget_sweep();
        let found = crate::rawiso_found::found_of(ask, &rt, &y, centres, q)?;
        Ok(IsoOut {
            members: crate::rawiso_json::members_json(ask, n, &probe, off.as_ref(), q.is_some()),
            result: crate::rawiso_json::result_json(ask, &found),
        })
    }
}

#[cfg(test)]
#[path = "rawiso_tests.rs"]
mod tests;
