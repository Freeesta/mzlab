//! The `raw_records` extra of a scan row of the dump: the raw reaction records of the scan event, for the energy rule of `reaction.rs`.
//! Used by `main.rs` (`scan_row`).

use serde_json::{json, Value};

/// Every record of the raw event, in order, precursor <= 0 included (`reactions_of` drops those): an extra of the bench for the energy rule of `reaction.rs`.
/// Written only when the event has two or more records.
pub(crate) fn raw_records(ev: &opentfraw::scan_event::ScanEvent) -> Option<Value> {
    if ev.reactions.len() < 2 {
        return None;
    }
    let list = ev.reactions.iter().map(|r| {
        json!({
            "precursor": r.precursor_mz,
            "energy": r.energy,
            "flags": r.unknown_long1,
            "double": r.unknown_double,
            "long2": r.unknown_long2,
        })
    });
    Some(Value::Array(list.collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentfraw::scan_event::{Reaction, ScanEvent, ScanEventPreamble};

    fn event(reactions: Vec<Reaction>) -> ScanEvent {
        ScanEvent {
            preamble: ScanEventPreamble { bytes: Vec::new() },
            reactions,
            fraction_collectors: Vec::new(),
            coefficients: Vec::new(),
        }
    }

    fn rec(precursor_mz: f64, energy: f64, long1: u32, double: f64, long2: u32) -> Reaction {
        Reaction {
            precursor_mz,
            unknown_double: double,
            energy,
            unknown_long1: long1,
            unknown_long2: long2,
        }
    }

    #[test]
    fn raw_records_needs_two_records() {
        assert!(raw_records(&event(vec![])).is_none());
        assert!(raw_records(&event(vec![rec(442.036, 27.0, 1, 2.0, 3)])).is_none());
    }

    #[test]
    fn raw_records_keeps_every_record_in_order() {
        let ev = event(vec![
            rec(442.036, 27.0, 1, 617.272, 5),
            rec(0.0, 33.0, 2, 0.5, 7),
        ]);
        let v = raw_records(&ev).unwrap();
        assert_eq!(
            v,
            json!([
                {"precursor": 442.036, "energy": 27.0, "flags": 1, "double": 617.272, "long2": 5},
                {"precursor": 0.0, "energy": 33.0, "flags": 2, "double": 0.5, "long2": 7},
            ])
        );
    }
}
