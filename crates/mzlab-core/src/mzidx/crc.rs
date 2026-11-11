//! The CRC-32 of every array of a section (`sections[i].crc32` of the meta, docs/agenti/rust-mzidx.md): an `Output` that hashes the bytes
//! written into each array while they arrive in order, and the JSON of the sections. An array written out of order gets no sum, so the
//! section is written without `crc32` (a reader checks only what it finds).

use super::{Info, Output, BLOCK};
use crate::error::Result;
use flate2::Crc;

struct Span {
    start: u64,
    end: u64,
    next: u64,
    crc: Crc,
    ok: bool,
}

/// `out`, plus the sums of the ten arrays of each section (`NAMES` order of idxread.rs).
pub(super) struct Summed<'a> {
    out: &'a mut dyn Output,
    spans: Vec<(usize, Span)>, // (section * 10 + array, span), by start then end
}

impl<'a> Summed<'a> {
    pub(super) fn new(out: &'a mut dyn Output, infos: &[Info]) -> Self {
        let mut spans = Vec::new();
        for (si, s) in infos.iter().enumerate() {
            let (n, np, w) = (s.ids.len() as u64, s.n_peaks, if s.f32 { 4 } else { 8 });
            let o = &s.off;
            let arrays = [
                (o.rt, 8 * n),
                (o.tic, 8 * n),
                (o.scan_ids, 8 * n),
                (o.sumint, 8 * n),
                (o.bp_mz, 8 * n),
                (o.bp_inten, 8 * n),
                (o.fence, w * np.div_ceil(BLOCK)),
                (o.mz, w * np),
                (o.inten, w * np),
                (o.pos, 4 * np),
            ];
            for (k, (start, len)) in arrays.into_iter().enumerate() {
                let crc = Crc::new();
                spans.push((
                    si * 10 + k,
                    Span {
                        start,
                        end: start + len,
                        next: start,
                        crc,
                        ok: true,
                    },
                ));
            }
        }
        crate::sortc::slice_by_key(&mut spans, |x| (x.1.start, x.1.end));
        Summed { out, spans }
    }

    /// The sums of section `si`, when each of its arrays was written whole and in order.
    fn sums(&self, si: usize) -> Option<[u32; 10]> {
        let mut c = [0u32; 10];
        for (id, s) in self.spans.iter().filter(|(id, _)| id / 10 == si) {
            if !s.ok || s.next != s.end {
                return None;
            }
            c[id % 10] = s.crc.sum();
        }
        Some(c)
    }
}

impl Output for Summed<'_> {
    fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<()> {
        // the last span starting at or before `offset` (among equal starts the longest: an empty array shares its start with the next one)
        let i = self.spans.partition_point(|(_, s)| s.start <= offset);
        if let Some((_, s)) = i.checked_sub(1).and_then(|i| self.spans.get_mut(i))
            && !data.is_empty()
            && offset < s.end
        {
            if offset == s.next && offset + data.len() as u64 <= s.end {
                s.crc.update(data);
                s.next += data.len() as u64;
            } else {
                s.ok = false;
            }
        }
        self.out.write_at(offset, data)
    }
}

/// The `sections` array of the meta.
pub(super) fn sections_json(infos: &[Info], sums: &Summed) -> String {
    let mut j = String::from("[");
    for (i, s) in infos.iter().enumerate() {
        if i > 0 {
            j.push(',');
        }
        let o = &s.off;
        j.push_str(&format!(
            "{{\"level\":{},\"dtype\":\"{}\",\"n_scans\":{},\"n_peaks\":{},\"n_fence\":{},\"off\":{{\"rt\":{},\"tic\":{},\"scan_ids\":{},\"sumint\":{},\"bp_mz\":{},\"bp_inten\":{},\"fence\":{},\"mz\":{},\"inten\":{},\"pos\":{}}}",
            s.level,
            if s.f32 { "<f4" } else { "<f8" },
            s.ids.len(),
            s.n_peaks,
            s.n_peaks.div_ceil(BLOCK),
            o.rt, o.tic, o.scan_ids, o.sumint, o.bp_mz, o.bp_inten, o.fence, o.mz, o.inten, o.pos
        ));
        if let Some(c) = sums.sums(i) {
            let names = crate::idxread::NAMES
                .iter()
                .zip(c)
                .map(|(n, v)| format!("\"{n}\":{v}"));
            j.push_str(&format!(
                ",\"crc32\":{{{}}}",
                names.collect::<Vec<_>>().join(",")
            ));
        }
        j.push('}');
    }
    j.push(']');
    j
}
