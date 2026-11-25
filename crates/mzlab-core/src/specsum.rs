//! `/api/specsum` (`spectrum get --sum`): what a summed spectrum is made of, so that a reader can judge whether it is credible: the scans added, their
//! total counts (cps summed over scans) per scan type, the ions (counts x fill time, only when the fill time is known) and the ion quantum of the level
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
/// The key of the note that says why `ions_by_quantum` is null.
pub const NO_QUANTUM: &str = "note.spec.sum.noQuantum";

/// Per scan type: its key, the scans of that type and their counts.
type Types = Vec<(String, usize, f64)>;

impl<R: ReadAt> Routes<R> {
    /// `/api/specsum?rt0=A&rt1=B[&level=N][&precursor=P][&fill_ms=T]`: the evidence of the summed spectrum of the window, `None` when
    /// the request has no window.
    pub fn spec_sum_route(&mut self, q: &Query) -> Result<Option<String>> {
        let (Some(rt0), Some(rt1)) = (q.num("rt0")?, q.num("rt1")?) else {
            return Ok(None);
        };
        let level = q.level();
        let fill = match q.num("fill_ms")? {
            Some(t) if t > 0.0 && t.is_finite() => Some(t),
            Some(_) => return Err(Error::new("err.spec.fill")),
            None => None,
        };
        let precursor = q.num("precursor")?;
        // the precursor window of the nominal-mass spectrum (`/api/spectrum` widens it in ppm only for a high-resolution centroid file, which `--sum` never sends)
        let prec = (precursor, 0.6);
        let (lo, hi) = (rt0.min(rt1), rt0.max(rt1));
        let probe = self.probe(level, crate::rawxic::PROBE)?;
        let quant = crate::rawinfo::quantum_of(Some(&probe)).filter(|q| q.accepted);
        let (n, total, types) = self.tally(level, (lo, hi), prec)?;
        Ok(Some(answer(level, fill, n, total, &types, &probe, quant)))
    }

    /// The scans of a window of one level (inside it and, for a product-ion level, near the precursor): how many, their counts, and the same per scan type
    /// (in order of first appearance). Read from the arrays of the section and the scan table, borrowed.
    fn tally(
        &mut self,
        level: u8,
        (lo, hi): (f64, f64),
        (precursor, tol): (Option<f64>, f64),
    ) -> Result<(usize, f64, Types)> {
        let mut types: Types = Vec::new();
        let (mut n, mut total) = (0usize, 0.0f64);
        let Some(si) = self.idx.section_of(level, self.pol) else {
            return Ok((n, total, types));
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
            let (key, c) = (scan_type(s), ar.sumint[j]);
            n += 1;
            total += c;
            match types.iter_mut().find(|t| t.0 == key) {
                Some(t) => {
                    t.1 += 1;
                    t.2 += c;
                }
                None => types.push((key, 1, c)),
            }
        }
        Ok((n, total, types))
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
    if let (Some(a), Some(b)) = (w.next(), w.next()) {
        if b == "+" || b == "-" {
            o.push_str(a);
            o.push(' ');
            o.push_str(b);
            o.push(' ');
        }
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

/// The answer text (see the module header for the keys).
fn answer(
    level: u8,
    fill: Option<f64>,
    n: usize,
    total: f64,
    types: &Types,
    probe: &crate::rawinfo::Probe,
    quant: Option<crate::ionstat::Quantum>,
) -> String {
    let ions = |counts: f64, ms: Option<f64>| ms.map(|t| counts * t / 1000.0);
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
    o.raw(",\"n_scans\":").int(n as i64);
    o.raw(",\"counts_cps\":").num(round(total, 1));
    o.raw(",\"fill_time\":{\"ms\":");
    match fill {
        Some(t) => o.num(t).raw(",\"source\":\"user\",\"note\":null"),
        None => o.raw("null,\"source\":\"none\",\"note\":").str(NO_FILL),
    };
    o.raw("},\"ions\":");
    opt(&mut o, ions(total, fill));
    o.raw(",\"by_scan_type\":[");
    for (i, (k, c, s)) in types.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).raw("{\"scan_type\":");
        o.str(k).raw(",\"n_scans\":").int(*c as i64);
        o.raw(",\"counts_cps\":")
            .num(round(*s, 1))
            .raw(",\"ions\":");
        opt(&mut o, ions(*s, fill));
        o.raw("}");
    }
    o.raw("],\"ion_quantum\":")
        .raw(&crate::rawinfo::quantum_json(Some(probe)));
    o.raw(",\"ions_by_quantum\":");
    opt(&mut o, quant.map(|q| total / q.q));
    o.raw(",\"ions_by_quantum_note\":");
    match quant {
        Some(_) => o.raw("null"),
        None => o.str(NO_QUANTUM),
    };
    o.raw(",\"background\":{\"applied\":\"none\",\"method\":null}}");
    o.0
}
