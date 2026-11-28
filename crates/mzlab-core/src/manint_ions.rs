//! The opt-in ion scale of `peak table --ions` (`manint::table_text_ions`): after the cps columns, per row the area, height and noise in ions, the
//! Poisson S/N and the factor with its source. The factor is per trace (`ionscale::factor`): the seconds the trace supplies itself (`trace xic --ions`
//! writes them), else the scan type of the sample with the method values (a `--set` value or the dwell the caller read from the file) and, where the
//! method gives no time, the ion quantum of the trace. It lives apart from `manint` so the cps table (the `rotte` module) never links it.

use super::*;
use crate::ionscale::{Facts, ScanKind, Source};
use crate::ionstat::{measure, FloorAsk, FloorParams, OneIon};
use crate::ionstat_ions::{measure_ions, IonsAsk};
use crate::json::{round, Out};

/// The header cells added with `--ions`, after every other column.
pub const IONS_HEAD: &str =
    ",area_ions,height_ions,noise_ions,sn_ions,ions_per_cps,ion_source,ion_scan_type";

/// What the caller says about one trace of a sample.
#[derive(Clone, Copy)]
enum Given {
    /// Seconds one point accumulates ions, from the trace itself (`trace xic --ions` evidence).
    Seconds(f64),
    /// The dwell (ms) of this transition, used where the sample has no `dwell_ms` of its own.
    Dwell(f64),
}

struct SampleIons {
    kind: Option<ScanKind>,
    from: String,
    facts: Facts,
    traces: Vec<Option<Given>>,
}

/// The ion-scale request of a table: one entry per sample, in the order of the files.
pub(super) struct Ions(Vec<SampleIons>);

fn pos(v: &Value, k: &str) -> Option<f64> {
    v.get(k)
        .and_then(Value::f64)
        .filter(|x| x.is_finite() && *x > 0.0)
}

impl Ions {
    /// `{"samples":[{"scan_type"?, "scan_from"?, "fill_ms"?, "dwell_ms"?, "scan_s"?, "step_da"?, "mass_lo"?, "mass_hi"?,
    /// "traces"?: [{"ions_per_cps"} | {"dwell_ms"} | null, ...]}]}`; an unknown scan type is an error, never ignored.
    pub(super) fn parse(text: &str) -> Result<Ions> {
        let doc = parse(text.as_bytes())?;
        let mut out = Vec::new();
        for s in doc.get("samples").map_or(&[][..], Value::arr) {
            let kind = match s.get("scan_type").and_then(Value::str) {
                None => None,
                Some(k) => Some(
                    ScanKind::parse(k)
                        .ok_or_else(|| Error::new("err.manint.ions").with("scan_type", k))?,
                ),
            };
            let traces = s
                .get("traces")
                .map_or(&[][..], Value::arr)
                .iter()
                .map(|t| {
                    pos(t, "ions_per_cps")
                        .map(Given::Seconds)
                        .or_else(|| pos(t, "dwell_ms").map(Given::Dwell))
                })
                .collect();
            out.push(SampleIons {
                kind,
                from: s
                    .get("scan_from")
                    .and_then(Value::str)
                    .unwrap_or("")
                    .to_string(),
                facts: Facts {
                    fill_ms: pos(s, "fill_ms"),
                    dwell_ms: pos(s, "dwell_ms"),
                    scan_s: pos(s, "scan_s"),
                    step_da: pos(s, "step_da"),
                    mass_lo: pos(s, "mass_lo"),
                    mass_hi: pos(s, "mass_hi"),
                },
                traces,
            });
        }
        Ok(Ions(out))
    }
}

/// The factor of one row's trace and the noise at it.
struct At {
    kind: ScanKind,
    seconds: Option<f64>,
    source: &'static str,
    noise_cps: Option<f64>,
    sn: Option<f64>,
}

/// The median spacing (s) of the points inside the window, else of the whole trace: the cycle one point stands for.
fn cycle_s(t: &[f64], a: f64, b: f64) -> Option<f64> {
    let gaps = |lo: f64, hi: f64| -> Vec<f64> {
        t.windows(2)
            .filter(|w| w[0] >= lo && w[1] <= hi && w[1] > w[0])
            .map(|w| (w[1] - w[0]) * 60.0)
            .collect()
    };
    let mut g = gaps(a, b);
    if g.is_empty() {
        g = gaps(f64::NEG_INFINITY, f64::INFINITY);
    }
    crate::sortc::by(&mut g, |x, y| x.total_cmp(y));
    g.get(g.len() / 2).copied()
}

impl Ions {
    fn at(&self, t: &Trace, r: &Row) -> At {
        let none = At {
            kind: ScanKind::Other,
            seconds: None,
            source: "",
            noise_cps: None,
            sn: None,
        };
        let Some(s) = self.0.get(r.file) else {
            return none;
        };
        let ask = FloorAsk {
            from: None,
            to: None,
            peak: Some((r.a, r.b)),
        };
        let given = s.traces.get(r.tr).copied().flatten();
        let kind = s.kind.unwrap_or(ScanKind::Other);
        if let Some(Given::Seconds(sec)) = given {
            let p = FloorParams {
                one_ion: OneIon::Cps(1.0 / sec),
                ..FloorParams::default()
            };
            let m = measure(&t.t, &t.y, ask, p).ok();
            return At {
                kind,
                seconds: Some(sec),
                source: "trace",
                noise_cps: m.as_ref().map(|m| m.floor.noise),
                sn: m.and_then(|m| m.peak).and_then(|p| p.sn),
            };
        }
        let mut facts = s.facts;
        if let (Some(Given::Dwell(d)), None) = (given, facts.dwell_ms) {
            facts.dwell_ms = Some(d);
        }
        let ask_ions = IonsAsk {
            kind: Some(kind),
            facts,
        };
        match measure_ions(&t.t, &t.y, ask, FloorParams::default(), Some(ask_ions)) {
            Ok((_, Some(o))) => {
                let m = o.at_factor.as_ref();
                At {
                    kind,
                    seconds: o.factor.as_ref().map(|f| f.seconds),
                    source: match o.factor.as_ref().map(|f| f.source) {
                        Some(Source::Method) => "method",
                        Some(Source::Data) => "data",
                        None => "",
                    },
                    noise_cps: m.map(|m| m.floor.noise),
                    sn: m.and_then(|m| m.peak.as_ref()).and_then(|p| p.sn),
                }
            }
            _ => none,
        }
    }
}

impl csv::Extra for Ions {
    fn head(&self) -> String {
        IONS_HEAD.to_string()
    }
    fn width(&self) -> usize {
        7
    }
    fn cells(&self, t: &Trace, r: &Row) -> String {
        let a = self.at(t, r);
        let cell = |v: Option<f64>| {
            v.filter(|x| x.is_finite())
                .map_or(String::new(), |x| sci(x, 5))
        };
        let s = a.seconds;
        // ions in the peak = the sum of the points x the seconds each accumulates; the area is that sum x the cycle
        let area = s
            .zip(cycle_s(&t.t, r.a, r.b))
            .map(|(s, dt)| r.area / dt * s);
        format!(
            ",{},{},{},{},{},{},{}",
            cell(area),
            cell(s.map(|s| r.height * s)),
            cell(s.zip(a.noise_cps).map(|(s, n)| n * s)),
            a.sn.filter(|v| v.is_finite())
                .map_or(String::new(), |v| format!("{v:.1}")),
            s.map_or(String::new(), |s| format!("{}", round(s, 9))),
            a.source,
            a.kind.name()
        )
    }
}

/// What the call did per sample, as JSON `[{"sample", "scan_type", "scan_type_from", "facts", "trace_factors"}]`: the scan type and where it came
/// from, the method values the sample was given and how many of its traces brought their own factor. The factor of each row is in its columns.
pub fn choices_text(files: &str, ions: &str) -> Result<String> {
    let (samples, _) = samples_and_keys(files)?;
    let ions = Ions::parse(ions)?;
    let mut o = Out::default();
    o.raw("[");
    for (i, (name, _, _, _)) in samples.iter().enumerate() {
        let s = ions.0.get(i);
        o.raw(if i > 0 {
            ",{\"sample\":"
        } else {
            "{\"sample\":"
        })
        .str(name);
        let kind = s.and_then(|s| s.kind).unwrap_or(ScanKind::Other);
        o.raw(",\"scan_type\":").str(kind.name());
        o.raw(",\"scan_type_from\":").str(match s {
            Some(s) if !s.from.is_empty() => &s.from,
            _ => "not named: the traces carry no scans",
        });
        let f = s.map(|s| s.facts).unwrap_or_default();
        o.raw(",\"facts\":{");
        let mut first = true;
        for (k, v) in [
            ("fill_ms", f.fill_ms),
            ("dwell_ms", f.dwell_ms),
            ("scan_s", f.scan_s),
            ("step_da", f.step_da),
            ("mass_lo", f.mass_lo),
            ("mass_hi", f.mass_hi),
        ] {
            if let Some(v) = v {
                o.raw(if first { "\"" } else { ",\"" })
                    .raw(k)
                    .raw("\":")
                    .num(v);
                first = false;
            }
        }
        let own = s.map_or(0, |s| {
            s.traces
                .iter()
                .filter(|g| matches!(g, Some(Given::Seconds(_))))
                .count()
        });
        o.raw("},\"trace_factors\":").int(own as i64);
        o.raw(",\"needs\":").str(kind.needs());
        o.raw("}");
    }
    o.raw("]");
    Ok(o.0)
}
