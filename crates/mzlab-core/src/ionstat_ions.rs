//! The opt-in ion scale of `mzlab-cli peak noise --ions` and mzlab-wasm `noise_floor` (`ionstat`): the cps-to-ions factor of `ionscale::factor` (the
//! method values the caller gives, else 1 / the ion quantum of the trace) and, at that factor, the noise, the peak and the check next to the cps ones.
//! `noise-floor/1` without the opt-in keeps its bytes; with it one member `ions` is appended and nothing above changes.

use crate::ionscale::{self, Factor, Facts, ScanKind};
use crate::ionstat::{measure, FloorAsk, FloorParams, FloorResult, OneIon};
use crate::json::{round, Out};

/// What the caller says about the scan: its type (`None`: unknown, the data estimate alone decides) and the method values of that type.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IonsAsk {
    pub kind: Option<ScanKind>,
    pub facts: Facts,
}

impl IonsAsk {
    /// The numbers of the wasm interface, from `p[3]`: opt-in (> 0), scan type (1 ems, 2 epi, 3 q1, 4 sim, 5 other, anything else unknown), then
    /// fill_ms, dwell_ms, scan_s, step_da, mass_lo, mass_hi (NaN or <= 0 = not given). `None` without the opt-in.
    pub fn from_slice(p: &[f64]) -> Option<IonsAsk> {
        if p.get(3).copied().is_none_or(|v| v.is_nan() || v <= 0.0) {
            return None;
        }
        let g = |i: usize| p.get(i).copied().filter(|v| v.is_finite() && *v > 0.0);
        let kind = match p.get(4).copied() {
            Some(1.0) => Some(ScanKind::Ems),
            Some(2.0) => Some(ScanKind::Epi),
            Some(3.0) => Some(ScanKind::Q1),
            Some(4.0) => Some(ScanKind::Sim),
            Some(5.0) => Some(ScanKind::Other),
            _ => None,
        };
        Some(IonsAsk {
            kind,
            facts: Facts {
                fill_ms: g(5),
                dwell_ms: g(6),
                scan_s: g(7),
                step_da: g(8),
                mass_lo: g(9),
                mass_hi: g(10),
            },
        })
    }
}

/// The answer on the ion scale: the factor with its evidence and the trace measured again with one ion = 1 / `ions_per_cps` cps.
#[derive(Clone, Debug, PartialEq)]
pub struct IonsOut {
    pub kind: ScanKind,
    pub named: bool,
    pub factor: Option<Factor>,
    pub at_factor: Option<FloorResult>,
}

/// `measure` and, with `ions`, the same trace at the factor of `ionscale::factor` (the accepted quantum of the trace as the data estimate). The
/// result of the cps scale is `measure`'s, unchanged.
pub fn measure_ions(
    rt: &[f64],
    y: &[f64],
    ask: FloorAsk,
    p: FloorParams,
    ions: Option<IonsAsk>,
) -> Result<(FloorResult, Option<IonsOut>), String> {
    let r = measure(rt, y, ask, p)?;
    let Some(a) = ions else {
        return Ok((r, None));
    };
    let q = r.floor.quantum.filter(|q| q.accepted).map(|q| q.q);
    let kind = a.kind.unwrap_or(ScanKind::Other);
    let factor = ionscale::factor(kind, &a.facts, q);
    let at_factor = match &factor {
        Some(f) => Some(measure(
            rt,
            y,
            ask,
            FloorParams {
                one_ion: OneIon::Cps(f.cps_per_ion()),
                ..p
            },
        )?),
        None => None,
    };
    Ok((
        r,
        Some(IonsOut {
            kind,
            named: a.kind.is_some(),
            factor,
            at_factor,
        }),
    ))
}

fn opt(o: &mut Out, v: Option<f64>) {
    o.opt(v.map(|x| round(x, 6)));
}

/// The `ions` member: the factor (`ion_scale`) and its choice, then the noise, the peak and the check at that factor. Every value of the ion scale is
/// the cps value times `ions_per_cps`, except `sn_ions` and the check, which use the floor of one ion at that factor.
pub fn ions_json(i: &IonsOut) -> String {
    let mut o = Out::default();
    o.raw("{\"ion_scale\":");
    match &i.factor {
        Some(f) => o.raw(&f.json()),
        None => o.raw("null"),
    };
    o.raw(",\"ion_scale_choice\":").raw(&ionscale::choice_json(
        i.factor.as_ref(),
        i.kind,
        if i.named {
            "scan_type given"
        } else {
            "no scan type given: the trace has no scans to read it from"
        },
        "the ion-scale values are null, the cps ones stand",
    ));
    let sec = i.factor.as_ref().map(|f| f.seconds);
    let r = i.at_factor.as_ref();
    o.raw(",\"one_ion_cps\":");
    opt(&mut o, i.factor.as_ref().map(|f| f.cps_per_ion()));
    o.raw(",\"noise_ions\":");
    opt(&mut o, r.zip(sec).map(|(r, s)| r.floor.noise * s));
    o.raw(",\"estimator\":")
        .raw(&r.map_or("null".into(), |r| format!("\"{}\"", r.floor.estimator)));
    o.raw(",\"peak\":");
    match r.and_then(|r| r.peak.as_ref()).zip(sec) {
        Some((p, s)) => {
            o.raw("{\"height_ions\":");
            opt(&mut o, Some(p.height * s));
            o.raw(",\"sn_ions\":");
            opt(&mut o, p.sn);
            o.raw("}")
        }
        None => o.raw("null"),
    };
    o.raw(",\"check\":");
    match r.and_then(|r| r.check.as_ref().zip(r.peak.as_ref())) {
        Some((c, p)) => {
            o.raw("{\"id\":\"noise_floor\",\"status\":")
                .str(c.status)
                .raw(",\"value\":");
            opt(&mut o, p.sn);
            o.raw(",\"threshold\":")
                .num(r.map_or(0.0, |r| r.params.min_sn))
                .raw(",\"rule\":")
                .str(&c.rule)
                .raw("}")
        }
        None => o.raw("null"),
    };
    o.raw("}");
    o.0
}

/// `floor_json` with the `ions` member appended (the cps members are byte for byte those of `floor_json`).
pub fn floor_json_ions(r: &FloorResult, i: Option<&IonsOut>) -> String {
    let base = crate::ionstat::floor_json(r);
    match i {
        None => base,
        Some(i) => format!("{},\"ions\":{}}}", &base[..base.len() - 1], ions_json(i)),
    }
}

/// The one log line of the ion scale (stderr of the CLI): which factor, from where, and what it did to the check.
pub fn ions_log(i: &IonsOut) -> String {
    let from = if i.named {
        "scan type given"
    } else {
        "no scan type given"
    };
    match &i.factor {
        None => format!(
            "ion scale: none ({from}: {} needs {} and no ion quantum was accepted); the cps values stand",
            i.kind.name(),
            i.kind.needs()
        ),
        Some(f) => {
            let src = match f.missing {
                Some(m) => format!("data estimate 1 / ion quantum, the method gave no {m}"),
                None => format!("method {}", f.field),
            };
            let tail = i
                .at_factor
                .as_ref()
                .and_then(|r| r.check.as_ref())
                .map_or(String::new(), |c| format!("; at one ion = {} cps: {}", crate::ionstat::rd(f.cps_per_ion()), c.rule));
            format!(
                "ion scale: {} ions per cps ({} scan, {from}; {src}){tail}",
                round(f.seconds, 9),
                i.kind.name()
            )
        }
    }
}

#[cfg(test)]
#[path = "ionstat_ions_tests.rs"]
mod tests;
