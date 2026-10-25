//! Q1 of each SRM event from the v66 method/transition table in the pre-data window, in one pass over the bytes.
//! Used by `RawFileReader::open_with`; equivalent to scanning the window once per event.
use std::collections::HashMap;

use crate::bytes::read_f64_le;
use crate::error::Result;

/// For each `(event, q3_hi)` target, the Q1 of the first byte position `i` (from 16) where the f64 at `i` is within 0.002 of `q3_hi`, the f64 at `i - 8` is
/// below it by less than 0.1, and the f64 at `i - 16` is in (50, 3000). Targets without a match are left out.
pub(crate) fn v66_q1_by_event(
    data: &[u8],
    targets: &HashMap<u16, f64>,
) -> Result<HashMap<u16, f64>> {
    let mut sorted: Vec<(f64, u16)> = targets.iter().map(|(&e, &t)| (t, e)).collect();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = HashMap::new();
    let (Some(first), Some(last)) = (sorted.first(), sorted.last()) else {
        return Ok(out);
    };
    let (lo_bound, hi_bound) = (first.0 - 0.01, last.0 + 0.01);
    let end = data.len().saturating_sub(8);
    for i in 16..end {
        let hi = read_f64_le(data, i)?;
        if !(hi >= lo_bound && hi <= hi_bound) {
            continue;
        }
        let lo = read_f64_le(data, i - 8)?;
        if !(hi > lo && (hi - lo) < 0.1) {
            continue;
        }
        let q1 = read_f64_le(data, i - 16)?;
        if !(q1 > 50.0 && q1 < 3000.0) {
            continue;
        }
        let from = sorted.partition_point(|&(t, _)| t < hi - 0.01);
        for &(t, event) in &sorted[from..] {
            if t > hi + 0.01 {
                break;
            }
            if (hi - t).abs() < 0.002 {
                out.entry(event).or_insert(q1);
            }
        }
        if out.len() == sorted.len() {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The per-event scan this pass replaced, kept as the reference.
    fn slow(data: &[u8], targets: &HashMap<u16, f64>) -> HashMap<u16, f64> {
        let mut m = HashMap::new();
        'outer: for (&event, &t) in targets {
            for i in 16..data.len().saturating_sub(8) {
                let hi = read_f64_le(data, i).unwrap();
                if (hi - t).abs() < 0.002 {
                    let lo = read_f64_le(data, i - 8).unwrap();
                    if hi > lo && (hi - lo) < 0.1 {
                        let q1 = read_f64_le(data, i - 16).unwrap();
                        if q1 > 50.0 && q1 < 3000.0 {
                            m.insert(event, q1);
                            continue 'outer;
                        }
                    }
                }
            }
        }
        m
    }

    #[test]
    fn one_pass_matches_per_event_scan() {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..200 {
            let mut data: Vec<u8> = (0..(next() % 400)).map(|_| next() as u8).collect();
            let mut targets = HashMap::new();
            for e in 0..(1 + next() % 5) as u16 {
                let t = 100.0 + (next() % 50) as f64 * 0.001 + (next() % 3) as f64 * 100.0;
                targets.insert(e, t);
                // Plant [q1][lo][hi] at a random (often unaligned) offset, sometimes twice and sometimes shifted by 0.0015.
                for _ in 0..(next() % 3) {
                    let hi = t + (next() % 3) as f64 * 0.0015 - 0.0015;
                    let mut rec = Vec::new();
                    rec.extend((50.0 + (next() % 100) as f64).to_le_bytes());
                    rec.extend((hi - 0.02).to_le_bytes());
                    rec.extend(hi.to_le_bytes());
                    let at = (next() as usize) % (data.len() + 1);
                    data.splice(at..at, rec);
                }
            }
            assert_eq!(
                v66_q1_by_event(&data, &targets).unwrap(),
                slow(&data, &targets)
            );
        }
    }
}
