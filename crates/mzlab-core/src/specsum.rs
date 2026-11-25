//! `/api/specsum` (`spectrum get --sum`): what a summed spectrum is made of, so that a reader can judge whether it is credible: the scans added, their
//! total counts (cps summed over scans) per scan type, the ions (counts x fill time, only when the fill time is known) and the ion quantum of the level
//! (`ionstat::quantum`, the one of `raw info`). Read from the index only (the arrays of the section), so the site (the lazy `rawxic` `.wasm`), the app and
//! the CLI answer the same bytes. Nothing is subtracted: the answer says so (`background.applied`).

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::iteminfo::hr_profile;
use crate::json::{round, Out};
use crate::routes::{Query, Routes};

/// The key of the note that says why `ions` is null.
pub const NO_FILL: &str = "note.spec.sum.noFill";
/// The key of the note that says why `ions_by_quantum` is null.
pub const NO_QUANTUM: &str = "note.spec.sum.noQuantum";

impl<R: ReadAt> Routes<R> {
    /// `/api/specsum?rt0=A&rt1=B[&level=N][&precursor=P][&filt=F|channel=ID][&fill_ms=T]`: the evidence of the summed spectrum of the window, `None` when
    /// the request has no window.
    pub fn spec_sum_route(&mut self, q: &Query) -> Result<Option<String>> {
        let (Some(rt0), Some(rt1)) = (q.num("rt0")?, q.num("rt1")?) else {
            return Ok(None);
        };
        let mut level = q.level();
        let mut filt = q.some("filt").map(str::to_string);
        if let Some(id) = q.some("channel") {
            let ch = self.canali();
            let i = ch
                .find(id)
                .ok_or_else(|| Error::new("err.canali.unknown").with("id", id))?;
            // as `/api/spectrum`: the level of the request wins, the channel's is the default
            if q.get("level").is_none() {
                level = ch.v[i].livello.max(1);
            }
            filt = Some(format!("channel:{id}"));
        }
        let fill = match q.num("fill_ms")? {
            Some(t) if t > 0.0 && t.is_finite() => Some(t),
            Some(t) => {
                return Err(Error::new("err.spec.fill").with("value", t));
            }
            None => None,
        };
        let precursor = q.num("precursor")?;
        // the same scans as `/api/spectrum`: the tolerance of the precursor is the one the spectrum route uses for the kind of file
        let profile = self.idx.scans.iter().any(|s| s.level == level && s.profile);
        let hr = hr_profile(&self.idx.scans, level == 1, q.get("hr") != Some("0"));
        let prec = if !profile && hr.on {
            Self::hr_cells(level, precursor, &hr).0
        } else {
            (precursor, 0.6)
        };
        let (lo, hi) = (rt0.min(rt1), rt0.max(rt1));
        let sel = self.select_scans(level, lo, hi, prec, filt.as_deref())?;
        let probe = self.probe(level, crate::rawxic::PROBE)?;
        let quant = crate::rawinfo::quantum_of(Some(&probe)).filter(|q| q.accepted);
        let (n, total, types) = self.tally(sel.as_ref())?;
        Ok(Some(answer(level, fill, n, total, &types, &probe, quant)))
    }

    /// The scans of the selection: how many, their counts, and the same per scan type (in order of first appearance).
    fn tally(
        &mut self,
        sel: Option<&crate::spectrum::Selection>,
    ) -> Result<(usize, f64, Vec<(String, usize, f64)>)> {
        let mut types: Vec<(String, usize, f64)> = Vec::new();
        let (mut n, mut total) = (0usize, 0.0f64);
        if let Some(sl) = sel {
            let ar = self.idx.arrays(sl.si)?.clone();
            for j in (0..ar.rt.len()).filter(|&j| sl.ok[j]) {
                let key = self.idx.scans[ar.scan_ids[j] as usize].key();
                let c = ar.sumint[j];
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
        }
        Ok((n, total, types))
    }
}

/// The answer text (see the module header for the keys).
fn answer(
    level: u8,
    fill: Option<f64>,
    n: usize,
    total: f64,
    types: &[(String, usize, f64)],
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
