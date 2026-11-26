//! `/api/xicions` (`mzlab-cli trace xic --ions`, `docs/agenti/raw-access.md`): the ion-scale companion of `/api/xic`. For a list of ions it answers the
//! cps-to-ions factor of the level (`ionscale::factor`, with its evidence and the log of the choice) and, per ion, the trace of `/api/xic` in ions: the
//! intensity as `/api/xic` rounds it times the seconds one point accumulates ions. The CLI, the app and the site (the lazy `.wasm` `rawxic`) answer
//! through here, so the bytes are one.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::ionscale;
use crate::json::{round, Out};
use crate::mzcal::Spec;
use crate::rawxic::{bad, ions_of, PROBE};
use crate::routes::{Query, Routes};

/// Most ions one question may hold: the same limit as `trace xic`.
pub const MAX_IONS: usize = 500;

/// A list of positive numbers (`mz` or `tol`), at most `MAX_IONS`.
fn list(q: &Query, k: &str) -> Result<Vec<f64>> {
    let v = q.some(k).ok_or_else(|| bad(k, ""))?;
    let l: Vec<f64> = v
        .split(',')
        .map(|t| {
            t.trim()
                .parse::<f64>()
                .ok()
                .filter(|x| x.is_finite() && *x > 0.0)
                .ok_or_else(|| bad(k, v))
        })
        .collect::<Result<_>>()?;
    if l.len() > MAX_IONS {
        return Err(Error::new("err.raw.xic.many")
            .with("count", l.len())
            .with("max", MAX_IONS));
    }
    Ok(l)
}

impl<R: ReadAt> Routes<R> {
    /// `mz=LIST&tol=LIST&ions=1[&ms_level=N][&smooth=3|5][&mz_offset=TEXT][&scan_type=..&fill_ms=..&dwell_ms=..&scan_s=..&step_da=..&mass_lo=..&mass_hi=..]`:
    /// `{"ion_scale","ion_scale_choice","traces":[{"mz","ions":[..]}]}`, one trace per ion in the order given (`tol`: the half-windows in Da, one per ion).
    /// The `ions` of a trace have the length of `/api/xic`'s `y`. A question without `ions=1` is refused: this route exists for the ion scale.
    pub fn xic_ions_route(&mut self, q: &Query) -> Result<Option<String>> {
        let num = |k: &str| -> Result<Option<f64>> {
            q.some(k).map_or(Ok(None), |v| {
                v.trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|x| x.is_finite())
                    .map(Some)
                    .ok_or_else(|| bad(k, v))
            })
        };
        let ions = ions_of(q, num)?.ok_or_else(|| bad("ions", q.some("ions").unwrap_or("")))?;
        let mz = list(q, "mz")?;
        let tol = list(q, "tol")?;
        if tol.len() != mz.len() {
            return Err(bad("tol", q.some("tol").unwrap_or("")));
        }
        let level = match num("ms_level")? {
            None => 1,
            Some(x) if (1.0..=10.0).contains(&x) && x.fract() == 0.0 => x as u8,
            Some(x) => return Err(bad("ms_level", &x.to_string())),
        };
        let smooth = match q.some("smooth") {
            None => None,
            Some("3") => Some(3),
            Some("5") => Some(5),
            Some(v) => return Err(bad("smooth", v)),
        };
        let keep = self.mz_off;
        let r = self.xic_ions_at(q, &ions, &mz, &tol, level, smooth);
        self.mz_off = keep;
        r.map(Some)
    }

    fn xic_ions_at(
        &mut self,
        q: &Query,
        ions: &(Option<ionscale::ScanKind>, ionscale::Facts),
        mz: &[f64],
        tol: &[f64],
        level: u8,
        smooth: Option<usize>,
    ) -> Result<String> {
        if let Some(spec) = q.some("mz_offset").map(Spec::parse).transpose()? {
            self.mz_off = spec.resolve(&mut self.idx, self.pol)?.da;
        }
        let probe = self.probe(level, PROBE)?;
        let quant = crate::rawinfo::quantum_of(Some(&probe)).filter(|q| q.accepted);
        let (factor, kind, named) = self.ion_scale_of(ions, level, quant.map(|q| q.q));
        let mut o = Out::default();
        o.raw("{\"ion_scale\":");
        match &factor {
            Some(f) => o.raw(&f.json()),
            None => o.raw("null"),
        };
        o.raw(",\"ion_scale_choice\":").raw(&ionscale::choice_json(
            factor.as_ref(),
            kind,
            if named {
                "scan_type given"
            } else {
                "the scans of the level"
            },
            "the ions arrays are absent",
        ));
        o.raw(",\"traces\":[");
        // without a factor there is nothing to scale: the ions stay null and the question still has one entry per ion
        let m = match &factor {
            Some(_) if self.xic_len(level) > 0 => Some(self.sweep_matrix(mz, tol, level, smooth)?),
            _ => None,
        };
        for (i, &x) in mz.iter().enumerate() {
            o.raw(if i > 0 { ",{\"mz\":" } else { "{\"mz\":" })
                .num(x)
                .raw(",\"ions\":");
            match (&factor, &m) {
                (Some(f), Some(m)) => {
                    let v: Vec<f64> = m.y[i]
                        .iter()
                        .map(|&y| round(round(y, 1) * f.seconds, 3))
                        .collect();
                    o.raw("[");
                    for (k, v) in v.iter().enumerate() {
                        o.raw(if k > 0 { "," } else { "" }).num(*v);
                    }
                    o.raw("]")
                }
                _ => o.raw("null"),
            };
            o.raw("}");
        }
        o.raw("]}");
        Ok(o.0)
    }
}

#[cfg(test)]
#[path = "rawxic_trace_tests.rs"]
mod tests;
