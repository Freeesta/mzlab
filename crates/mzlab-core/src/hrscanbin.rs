//! `/api/scanbin` of a CENTROID file (`App.scanbin`): the scans of `/api/spectra` as one binary block, without JSON numbers: 4 bytes (little endian) =
//! length of the JSON header `{n, scans: [{i, sid, rt, no, filter, mode, n, np, nl, tic}]}`, then for every scan its peaks (m/z float64, intensity
//! float32). `Ok(None)` = Python's (profile scans, a request it must refuse).

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::json::{round, Out};
use crate::routes::{Query, Routes};

impl<R: ReadAt> Routes<R> {
    /// The block of the scans asked for, from the mzML by offset.
    pub fn scanbin_route<M: ReadAt>(&mut self, q: &Query, mzml: &mut M) -> Result<Option<Vec<u8>>> {
        if !self.has_scans() {
            return Ok(None);
        }
        let Some(p) = self.picked_scans(q, mzml, 0.1, true)? else { return Ok(None) };
        let mut o = Out::default();
        o.raw("{\"n\":").int(p.n as i64).raw(",\"scans\":[");
        let mut body: Vec<u8> = Vec::new();
        for (k, r) in p.rows.iter().enumerate() {
            let sc = &self.idx.scans[r.sid as usize];
            o.raw(if k > 0 { "," } else { "" }).raw("{\"i\":").int(r.j as i64).raw(",\"sid\":").int(r.sid).raw(",\"rt\":").num(round(r.rt, 4));
            o.raw(",\"no\":").int(sc.number(r.sid as usize)).raw(",\"filter\":").str(&sc.filter).raw(",\"mode\":\"centroid\",\"n\":").int(r.mz.len() as i64);
            o.raw(",\"np\":0,\"nl\":").num(r.y.iter().copied().fold(0.0, f64::max)).raw(",\"tic\":").num(r.y.iter().sum()).raw("}");
            r.mz.iter().for_each(|v| body.extend_from_slice(&v.to_le_bytes()));
            r.y.iter().for_each(|v| body.extend_from_slice(&(*v as f32).to_le_bytes()));
        }
        o.raw("]}");
        let mut out = (o.0.len() as u32).to_le_bytes().to_vec();
        out.extend_from_slice(o.0.as_bytes());
        out.extend(body);
        Ok(Some(out))
    }
}
