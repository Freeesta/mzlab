//! The per-spectrum steps of `Builder::add`: quantising the peaks and serialising the record that is kept in the scratch store.

use super::{FLAG_FEW_PEAKS, FLAG_HAS_RI, FLAG_HAS_RT, INT_MAX};
use crate::clean::{quant_mz, MAX_MZ};
use crate::libinfo::inchikey14;
use crate::spec::{licence, Spectrum};
use crate::util::put_varint;

/// Peaks below `MAX_MZ` as (m/z in 1e-5 Da, intensity); peaks that land on the same m/z unit are merged.
pub(super) fn quantise_into(mz: &[f64], it: &[f64], q: &mut Vec<(u32, f64)>) {
    q.clear();
    for (m, i) in mz.iter().zip(it) {
        if *m >= MAX_MZ {
            continue;
        }
        let mq = quant_mz(*m);
        match q.last_mut() {
            Some(l) if l.0 == mq => l.1 += i,
            _ => q.push((mq, *i)),
        }
    }
}

/// record: prec f64 | pol | licence | flags u16 | orig u32 | rt f32 | ri f32 | inchikey14 u64 | n u16 | n x (mz u32, it u16) | 10 x (varint len, bytes)
pub(super) fn record_into(rec: &mut Vec<u8>, s: &Spectrum, q: &[(u32, f64)], mx: f64, orig: u32) {
    rec.clear();
    let rt = s.rt.filter(|r| r.is_finite() && *r >= 0.0 && *r < 1e5);
    let ri = s.ri.filter(|r| r.is_finite() && *r > 0.0 && *r < 1e5);
    let mut flags = if q.len() < 3 { FLAG_FEW_PEAKS } else { 0 };
    if rt.is_some() {
        flags |= FLAG_HAS_RT;
    }
    if ri.is_some() {
        flags |= FLAG_HAS_RI;
    }
    rec.extend_from_slice(&s.prec.to_le_bytes());
    rec.push(s.pol as u8);
    rec.push(licence::class_of(&s.meta.license));
    rec.extend_from_slice(&flags.to_le_bytes());
    rec.extend_from_slice(&orig.to_le_bytes());
    rec.extend_from_slice(&(rt.unwrap_or(0.0) as f32).to_le_bytes());
    rec.extend_from_slice(&(ri.unwrap_or(0.0) as f32).to_le_bytes());
    rec.extend_from_slice(&inchikey14(&s.meta.inchikey).to_le_bytes());
    rec.extend_from_slice(&(q.len() as u16).to_le_bytes());
    for (mq, i) in q {
        rec.extend_from_slice(&mq.to_le_bytes());
        let iq = ((i / mx) * INT_MAX).round().clamp(1.0, INT_MAX) as u16;
        rec.extend_from_slice(&iq.to_le_bytes());
    }
    for f in s.meta.fields() {
        put_varint(rec, f.len() as u64);
        rec.extend_from_slice(f.as_bytes());
    }
}
