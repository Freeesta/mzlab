//! The value state of a reported quantity (AU-3): one word per number, the same in every output that gives an agent a quantity.
//! `detected`, `forced`, `filled` and `absent` are the evidence words of `aligned::EVIDENCE`; `na` (the file does not cover the
//! window, nothing was measured) and `below_loq` (a number under the lower end of the range, still shown) complete the set.

use crate::aligned::{evidence_word, CELL_NA};

/// Every value state, in the order of the documentation.
pub const VALUE_STATES: [&str; 6] = ["detected", "forced", "filled", "below_loq", "absent", "na"];

/// The state of an aligned-table cell: `na` where the file does not cover the row, else the evidence word (`absent` for a cell without a peak).
pub fn cell_state(state: u8, rejected: bool) -> &'static str {
    if state == CELL_NA {
        VALUE_STATES[5]
    } else {
        evidence_word(state, rejected)
    }
}

/// `below_loq` replaces the state of a number that was measured (`detected`, `forced`, `filled`) when it is under the lower end of the range
/// (the lowest calibrator, the LLOQ); a cell without a number keeps `absent` or `na`.
pub fn under_loq(state: &'static str, below: bool) -> &'static str {
    match state {
        "detected" | "forced" | "filled" if below => VALUE_STATES[3],
        s => s,
    }
}

/// The state of a `chrom.transitions` cell from its `peak` flag: `ok`/`edge` are a peak found and checked (`detected`), `no_peak` is a covered window
/// without one (`absent`), `no_trace` (no channel) and `no_rt` (a row without a window) measured nothing (`na`). An unknown flag is `na`, never a number's word.
pub fn flag_state(flag: &str) -> &'static str {
    match flag {
        "ok" | "edge" => VALUE_STATES[0],
        "no_peak" => VALUE_STATES[4],
        _ => VALUE_STATES[5],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aligned::{CELL_ABSENT, CELL_DETECTED, CELL_FILLED, CELL_TARGETED};

    #[test]
    fn cells_map_to_the_six_words() {
        let got: Vec<&str> = [
            CELL_DETECTED,
            CELL_TARGETED,
            CELL_FILLED,
            CELL_ABSENT,
            CELL_NA,
        ]
        .iter()
        .map(|s| cell_state(*s, false))
        .collect();
        assert_eq!(got, ["detected", "forced", "filled", "absent", "na"]);
        assert_eq!(cell_state(CELL_FILLED, true), "forced");
        assert!(VALUE_STATES.contains(&under_loq("detected", true)));
    }

    #[test]
    fn below_loq_only_touches_measured_numbers() {
        assert_eq!(under_loq("detected", true), "below_loq");
        assert_eq!(under_loq("forced", true), "below_loq");
        assert_eq!(under_loq("detected", false), "detected");
        assert_eq!(under_loq("absent", true), "absent");
        assert_eq!(under_loq("na", true), "na");
    }

    #[test]
    fn flags_and_origins_map_to_states() {
        let f: Vec<&str> = ["ok", "edge", "no_peak", "no_trace", "no_rt", "x"]
            .iter()
            .map(|f| flag_state(f))
            .collect();
        assert_eq!(f, ["detected", "detected", "absent", "na", "na", "na"]);
        for w in mzlab_core::value_word::WORDS {
            assert!(VALUE_STATES.contains(&w), "{w}");
        }
    }
}
