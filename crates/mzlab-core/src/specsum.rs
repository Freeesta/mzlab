//! `/api/specsum` (`spectrum get --sum`): what a summed spectrum is made of, so that a reader can judge whether it is credible: the scans added, their
//! total counts (cps summed over scans) per scan type, the ions (counts x fill time: the time `--fill-ms` states for every scan, else the one the file states
//! for each scan (index version 4, `Scan::fill_ms`), only when every scan has one) and the ion quantum of the level
//! (`ionstat::quantum`, the one of `raw info`). Read from the index only (the arrays of the section), so the site (the lazy `rawxic` `.wasm`), the app and
//! the CLI answer the same bytes. Nothing is subtracted: the answer says so (`background.applied`). The scans are chosen here from the section's arrays
//! (window, level, precursor within 0.6 Da), not by `select_scans`: its filters (`msn:` tree, channels) would add ~65 KB to the lazy `.wasm`, so a channel is not asked of this route.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::idxread::Scan;
use crate::json::{round, Out};
use crate::routes::{Query, Routes};

/// The key of the note that says why `ions` is null.
pub const NO_FILL: &str = "note.spec.sum.noFill";
/// `NO_FILL` for an index of version 3, which has no fill-time column: the note asks for the index to be built again.
pub const NO_FILL_OLD: &str = "note.spec.sum.noFillOld";
/// The key of the note that says why `ions_by_quantum` is null.
pub const NO_QUANTUM: &str = "note.spec.sum.noQuantum";

/// The `background` member of a sum that subtracts nothing.
pub(crate) const NO_BACKGROUND: &str = "{\"applied\":\"none\",\"method\":null}";

/// `fill_ms` of a request: a positive number or nothing; anything else is `err.spec.fill`.
pub(crate) fn fill_of(q: &Query) -> Result<Option<f64>> {
    match q.num("fill_ms")? {
        Some(t) if t > 0.0 && t.is_finite() => Ok(Some(t)),
        Some(_) => Err(Error::new("err.spec.fill")),
        None => Ok(None),
    }
}

/// What a set of scans adds up to: scans, counts (cps), the ions of the scans that state a fill time and how many do.
#[derive(Default, Clone, Copy)]
struct Sum {
    n: usize,
    cps: f64,
    ions: f64,
    timed: usize,
}

impl Sum {
    fn add(&mut self, cps: f64, fill_ms: Option<f64>) {
        self.n += 1;
        self.cps += cps;
        if let Some(t) = fill_ms {
            self.ions += cps * t / 1000.0;
            self.timed += 1;
        }
    }

    /// The ions when every scan states its fill time (`user` states one for all), else none: a part of the scans would understate them.
    fn ions(&self, user: Option<f64>) -> Option<f64> {
        match user {
            Some(t) => Some(self.cps * t / 1000.0),
            None => (self.n > 0 && self.timed == self.n).then_some(self.ions),
        }
    }
}

/// Per scan type: its key and its sum.
type Types = Vec<(String, Sum)>;

/// The scans of a window as `tally` finds them: all together, per scan type, and the fill times the file states (count, smallest, largest, total in ms).
#[derive(Default)]
pub(crate) struct Tally {
    all: Sum,
    types: Types,
    fills: (f64, f64, f64),
}

impl<R: ReadAt> Routes<R> {
    /// `/api/specsum?rt0=A&rt1=B[&level=N][&precursor=P][&fill_ms=T]`: the evidence of the summed spectrum of the window, `None` when
    /// the request has no window.
    pub fn spec_sum_route(&mut self, q: &Query) -> Result<Option<String>> {
        let (Some(rt0), Some(rt1)) = (q.num("rt0")?, q.num("rt1")?) else {
            return Ok(None);
        };
        let level = q.level();
        let fill = fill_of(q)?;
        let precursor = q.num("precursor")?;
        let (lo, hi) = (rt0.min(rt1), rt0.max(rt1));
        Ok(Some(self.sum_evidence(
            level,
            (lo, hi),
            precursor,
            fill,
            NO_BACKGROUND,
        )?))
    }

    /// The evidence of the sum of a window, with the `background` member the caller states (`NO_BACKGROUND` or what `specpeak` subtracted).
    pub(crate) fn sum_evidence(
        &mut self,
        level: u8,
        (lo, hi): (f64, f64),
        precursor: Option<f64>,
        fill: Option<f64>,
        background: &str,
    ) -> Result<String> {
        // the precursor window of the nominal-mass spectrum (`/api/spectrum` widens it in ppm only for a high-resolution centroid file, which `--sum` never sends)
        let prec = (precursor, 0.6);
        let probe = self.probe(level, crate::rawxic::PROBE)?;
        let quant = crate::rawinfo::quantum_of(Some(&probe)).filter(|q| q.accepted);
        let t = self.tally(level, (lo, hi), prec)?;
        // an index of version 3 has no fill-time column: the note asks for it to be built again
        let old = self.idx.version < 4;
        Ok(answer(level, (fill, old), &t, (&probe, quant), background))
    }

    /// The scans of a window of one level (inside it and, for a product-ion level, near the precursor): how many, their counts, and the same per scan type
    /// (in order of first appearance). Read from the arrays of the section and the scan table, borrowed.
    pub(crate) fn tally(
        &mut self,
        level: u8,
        (lo, hi): (f64, f64),
        (precursor, tol): (Option<f64>, f64),
    ) -> Result<Tally> {
        let mut t = Tally {
            fills: (f64::INFINITY, 0.0, 0.0),
            ..Tally::default()
        };
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok(t);
        };
        let want = if level > 1 { precursor } else { None };
        let (ar, scans) = self.idx.arrays_scans(si)?;
        for j in 0..ar.rt.len() {
            let s = &scans[ar.scan_ids[j] as usize];
            if ar.rt[j] < lo
                || ar.rt[j] > hi
                || want.is_some_and(|p| (s.prec(-1.0) - p).abs() > tol)
            {
                continue;
            }
            let (key, c, f) = (scan_type(s), ar.sumint[j], s.fill_ms());
            t.all.add(c, f);
            if let Some(f) = f {
                t.fills = (t.fills.0.min(f), t.fills.1.max(f), t.fills.2 + f);
            }
            match t.types.iter_mut().find(|k| k.0 == key) {
                Some(k) => k.1.add(c, f),
                None => {
                    let mut m = Sum::default();
                    m.add(c, f);
                    t.types.push((key, m));
                }
            }
        }
        Ok(t)
    }
}

/// The type of a scan for the split of a sum, as `Scan::key` names it for the survey and the product-ion scans (the filter without its range, or analyzer and
/// polarity, level and activation) but without that function's Unicode tables and MSn path, which weigh ~10 KB in the lazy `.wasm`: an MS3 scan is named by its
/// level and last activation only. `specsum_tests` pins the two names equal for the survey and product-ion scans of the instrument families.
pub(crate) fn scan_type(s: &Scan) -> String {
    // ASCII only (the spaces of a filter are ASCII): no Unicode tables in the lazy `.wasm`
    let f = s.filter.trim_end_matches(' ');
    if s.level <= 1 {
        let t = match f
            .strip_suffix(']')
            .and_then(|t| t.rfind('[').map(|p| &t[..p]))
        {
            Some(t) => t.trim_end_matches(' '),
            None => f,
        };
        return if t.is_empty() { "ms1".into() } else { t.into() };
    }
    let mut o = String::new();
    let mut w = f.split(' ').filter(|x| !x.is_empty());
    if let (Some(a), Some(b @ ("+" | "-"))) = (w.next(), w.next()) {
        o.push_str(a);
        o.push(' ');
        o.push_str(b);
        o.push(' ');
    }
    o.push_str("ms");
    o.push_str(&s.level.to_string());
    let act = s
        .act
        .as_deref()
        .filter(|a| !a.is_empty())
        .or_else(|| s.path.as_ref().and_then(|p| p.last()).map(|l| l.1.as_str()))
        .unwrap_or("");
    if !act.is_empty() {
        o.push_str(" @");
        o.extend(act.chars().map(|c| c.to_ascii_lowercase()));
    }
    o
}

/// The answer text (see the module header for the keys). `fill`: the time the user states (ms) and whether the index is of version 3.
fn answer(
    level: u8,
    (fill, old): (Option<f64>, bool),
    t: &Tally,
    (probe, quant): (&crate::rawinfo::Probe, Option<crate::ionstat::Quantum>),
    background: &str,
) -> String {
    let opt = |o: &mut Out, v: Option<f64>| match v {
        Some(v) => {
            o.num(round(v, 1));
        }
        None => {
            o.raw("null");
        }
    };
    let mut o = Out::default();
    o.raw("{\"mode\":\"sum\",\"ms_level\":")
        .int(i64::from(level));
    o.raw(",\"n_scans\":").int(t.all.n as i64);
    o.raw(",\"counts_cps\":").num(round(t.all.cps, 1));
    o.raw(",\"fill_time\":{\"ms\":");
    let ions = t.all.ions(fill);
    match fill {
        Some(f) => o.num(f).raw(",\"source\":\"user\",\"note\":null"),
        // the file's times: the mean, the range and how many scans state one; the note says why when some or all lack it
        None if t.all.timed > 0 => {
            let (lo, hi, sum) = t.fills;
            o.num(round(sum / t.all.timed as f64, 3))
                .raw(",\"source\":\"file\",\"min_ms\":")
                .num(round(lo, 3))
                .raw(",\"max_ms\":")
                .num(round(hi, 3))
                .raw(",\"n_timed\":")
                .int(t.all.timed as i64)
                .raw(",\"note\":");
            match ions {
                Some(_) => o.raw("null"),
                None => o.str(NO_FILL),
            }
        }
        None => {
            o.raw("null,\"source\":\"none\",\"note\":")
                .str(if old { NO_FILL_OLD } else { NO_FILL })
        }
    };
    o.raw("},\"ions\":");
    opt(&mut o, ions);
    o.raw(",\"by_scan_type\":[");
    for (i, (k, m)) in t.types.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"scan_type\":");
        o.str(k).raw(",\"n_scans\":").int(m.n as i64);
        o.raw(",\"counts_cps\":")
            .num(round(m.cps, 1))
            .raw(",\"ions\":");
        opt(&mut o, m.ions(fill));
        o.raw("}");
    }
    o.raw("],\"ion_quantum\":")
        .raw(&crate::rawinfo::quantum_json(Some(probe)));
    o.raw(",\"ions_by_quantum\":");
    opt(&mut o, quant.map(|q| t.all.cps / q.q));
    o.raw(",\"ions_by_quantum_note\":");
    match quant {
        Some(_) => o.raw("null"),
        None => o.str(NO_QUANTUM),
    };
    o.raw(",\"background\":").raw(background).raw("}");
    o.0
}
