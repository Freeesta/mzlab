//! The cps-to-ions factor of a scan type (`raw xic` with `ions=1`, `docs/agenti/raw-access.md`): ions = cps x t, t the seconds one point of the
//! trace accumulates ions. t comes from the method (fill time of a trap scan, dwell of SIM, scan time per step of a Q1 scan); without it the
//! data estimate 1 / q (`ionstat::quantum`) is used. Both are logged next to the factor, never one silently in place of the other.

use crate::json::{round, Out};

/// Relative gap of the method time and the data estimate beyond which `agree` is false.
pub const AGREE_TOL: f64 = 0.05;

/// The scan type behind a trace, as far as the time of a point goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanKind {
    /// Enhanced MS: one fill time per scan.
    Ems,
    /// Enhanced product ion: one fill time per scan.
    Epi,
    /// Q1 scan: the scan time shared by its steps.
    Q1,
    /// SIM: the dwell time of the ion.
    Sim,
    /// Anything else: only the data estimate can say.
    Other,
}

impl ScanKind {
    pub fn parse(s: &str) -> Option<ScanKind> {
        Some(match s {
            "ems" => ScanKind::Ems,
            "epi" => ScanKind::Epi,
            "q1" => ScanKind::Q1,
            "sim" => ScanKind::Sim,
            "other" => ScanKind::Other,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            ScanKind::Ems => "ems",
            ScanKind::Epi => "epi",
            ScanKind::Q1 => "q1",
            ScanKind::Sim => "sim",
            ScanKind::Other => "other",
        }
    }

    /// The method fields the time of a point of this kind needs.
    pub fn needs(self) -> &'static str {
        match self {
            ScanKind::Ems | ScanKind::Epi => "fill_ms",
            ScanKind::Sim => "dwell_ms",
            ScanKind::Q1 => "scan_s, step_da, mass_lo, mass_hi",
            ScanKind::Other => "a scan type (scan_type=ems|epi|q1|sim)",
        }
    }

    /// The kind of the scans of a level: the commonest SCIEX experiment name (`Scan::scan_type`: `EMS`, `EPI`, `Q1`), `SIM` when the filter of
    /// the scans says so, else `Other`. Ties go to the first name in that order, so the answer never depends on the file order.
    pub fn of<'a>(scans: impl Iterator<Item = (Option<&'a str>, &'a str)>) -> ScanKind {
        let mut n = [0usize; 4];
        for (t, filter) in scans {
            let k = match t {
                Some("EMS") => 0,
                Some("EPI") | Some("PI") => 1,
                Some("Q1") => 2,
                _ if filter.contains("SIM") => 3,
                _ => continue,
            };
            n[k] += 1;
        }
        let best = (0..4).fold(0, |b, i| if n[i] > n[b] { i } else { b });
        if n[best] == 0 {
            return ScanKind::Other;
        }
        [ScanKind::Ems, ScanKind::Epi, ScanKind::Q1, ScanKind::Sim][best]
    }
}

/// What the method says about the time of a point (every field absent when the method does not say).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Facts {
    pub fill_ms: Option<f64>,
    pub dwell_ms: Option<f64>,
    pub scan_s: Option<f64>,
    pub step_da: Option<f64>,
    pub mass_lo: Option<f64>,
    pub mass_hi: Option<f64>,
}

/// Where the factor comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// A field of the method (named in `Factor::field`).
    Method,
    /// The ion quantum of the data: t = 1 / q.
    Data,
}

/// The factor with its evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct Factor {
    pub kind: ScanKind,
    /// Seconds one point accumulates ions: ions = cps x `seconds`.
    pub seconds: f64,
    pub source: Source,
    /// The method fields the time was worked out from (`fill_ms`; `scan_s,step_da,mass_lo,mass_hi`) or `ion_quantum`.
    pub field: &'static str,
    /// The seconds the data estimate gives (1 / q), when the quantum was accepted.
    pub data_seconds: Option<f64>,
    /// Why the method was not used, when the kind needs a field it lacks.
    pub missing: Option<&'static str>,
}

impl Factor {
    /// Method time / data time, when both exist.
    pub fn ratio(&self) -> Option<f64> {
        match (self.source, self.data_seconds) {
            (Source::Method, Some(d)) => Some(self.seconds / d),
            _ => None,
        }
    }

    /// Cps per ion at this factor.
    pub fn cps_per_ion(&self) -> f64 {
        1.0 / self.seconds
    }

    /// The members of `ion_scale`: the factor, its source and every number behind it.
    pub fn json(&self) -> String {
        let mut o = Out::default();
        o.raw("{\"scan_type\":").str(self.kind.name());
        o.raw(",\"ions_per_cps\":").num(round(self.seconds, 9));
        o.raw(",\"cps_per_ion\":").num(round(self.cps_per_ion(), 4));
        o.raw(",\"source\":").str(match self.source {
            Source::Method => "method",
            Source::Data => "data",
        });
        o.raw(",\"field\":").str(self.field);
        o.raw(",\"data_ions_per_cps\":")
            .opt(self.data_seconds.map(|d| round(d, 9)));
        o.raw(",\"ratio\":").opt(self.ratio().map(|r| round(r, 4)));
        o.raw(",\"agree\":");
        match self.ratio() {
            Some(r) => o.raw(if (r - 1.0).abs() <= AGREE_TOL {
                "true"
            } else {
                "false"
            }),
            None => o.raw("null"),
        };
        o.raw(",\"missing\":");
        match self.missing {
            Some(m) => o.str(m),
            None => o.raw("null"),
        };
        o.raw("}");
        o.0
    }
}

fn pos(v: Option<f64>) -> Option<f64> {
    v.filter(|x| x.is_finite() && *x > 0.0)
}

/// The seconds a point of `kind` accumulates ions and the fields behind it, or the name of the field the method lacks.
fn from_method(
    kind: ScanKind,
    f: &Facts,
) -> std::result::Result<(f64, &'static str), &'static str> {
    match kind {
        ScanKind::Ems | ScanKind::Epi => pos(f.fill_ms)
            .map(|ms| (ms / 1000.0, "fill_ms"))
            .ok_or("fill_ms"),
        ScanKind::Sim => pos(f.dwell_ms)
            .map(|ms| (ms / 1000.0, "dwell_ms"))
            .ok_or("dwell_ms"),
        ScanKind::Q1 => {
            let (s, step) = (
                pos(f.scan_s).ok_or("scan_s")?,
                pos(f.step_da).ok_or("step_da")?,
            );
            let (lo, hi) = (
                pos(f.mass_lo).ok_or("mass_lo")?,
                pos(f.mass_hi).ok_or("mass_hi")?,
            );
            if hi <= lo {
                return Err("mass_hi");
            }
            // the scan time is shared by the steps of the range
            Ok((s * step / (hi - lo), "scan_s,step_da,mass_lo,mass_hi"))
        }
        ScanKind::Other => Err("scan_type"),
    }
}

/// The factor of `kind`: the method when it gives the time, else the data estimate from the accepted quantum `q` (cps per ion); `None` when neither
/// exists (the caller then keeps the cps columns and says so).
pub fn factor(kind: ScanKind, facts: &Facts, q: Option<f64>) -> Option<Factor> {
    let data = pos(q).map(|q| 1.0 / q);
    match from_method(kind, facts) {
        Ok((seconds, field)) => Some(Factor {
            kind,
            seconds,
            source: Source::Method,
            field,
            data_seconds: data,
            missing: None,
        }),
        Err(missing) => data.map(|d| Factor {
            kind,
            seconds: d,
            source: Source::Data,
            field: "ion_quantum",
            data_seconds: Some(d),
            missing: Some(missing),
        }),
    }
}

#[cfg(test)]
#[path = "ionscale_tests.rs"]
mod tests;
