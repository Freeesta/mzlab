//! The value-state words that the core outputs (`peak auto-lr`, `peak table`) print: the same strings as `mzlab_feat::value_state`, which cannot be
//! imported here (feat depends on core); a test there checks that every word below is in its vocabulary.

pub const DETECTED: &str = "detected";
pub const FORCED: &str = "forced";
pub const ABSENT: &str = "absent";
pub const NA: &str = "na";

/// Every word used here, for the vocabulary test of `mzlab-feat`.
pub const WORDS: [&str; 4] = [DETECTED, FORCED, ABSENT, NA];

/// The state of a hand-drawn or applied integration window of `peak table` from its `origin`: a window that was given or moved (`manual`, `applied`,
/// `shifted`, `too_far`) is an area integrated where the user said (`forced`); `no_peak` (a followed window with no apex) is `absent`; the engine's own
/// peaks (`auto`, `unmatched`) were found and checked (`detected`). Anything else is `na`.
pub fn origin_state(origin: &str) -> &'static str {
    match origin {
        "manual" | "applied" | "shifted" | "too_far" => FORCED,
        "no_peak" => ABSENT,
        "auto" | "unmatched" => DETECTED,
        _ => NA,
    }
}

/// The state of a `raw xic` window from its verdict: `peak` was found and checked (`detected`), `trace` has signal under the thresholds with its numbers
/// kept (`forced`), `nothing` is a covered window without a peak (`absent`), `no_data` measured nothing (`na`). An unknown verdict is `na`.
pub fn verdict_state(verdict: &str) -> &'static str {
    match verdict {
        "peak" => DETECTED,
        "trace" => FORCED,
        "nothing" => ABSENT,
        _ => NA,
    }
}

/// The state of a whole trace given to `peak auto-lr`: `na` when it is too short to be searched (fewer than 7 points, nothing was measured), `detected`
/// when it has peaks, `absent` when it was searched and has none.
pub fn trace_state(points: usize, peaks: usize) -> &'static str {
    if points < 7 {
        NA
    } else if peaks > 0 {
        DETECTED
    } else {
        ABSENT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdicts_map_to_states() {
        let v: Vec<&str> = ["peak", "trace", "nothing", "no_data", "x"]
            .iter()
            .map(|v| verdict_state(v))
            .collect();
        assert_eq!(v, ["detected", "forced", "absent", "na", "na"]);
    }

    #[test]
    fn origins_and_traces_map_to_states() {
        let o: Vec<&str> = [
            "manual",
            "applied",
            "shifted",
            "too_far",
            "no_peak",
            "auto",
            "unmatched",
            "x",
        ]
        .iter()
        .map(|o| origin_state(o))
        .collect();
        assert_eq!(
            o,
            ["forced", "forced", "forced", "forced", "absent", "detected", "detected", "na"]
        );
        assert_eq!(
            (trace_state(6, 0), trace_state(100, 0), trace_state(100, 2)),
            ("na", "absent", "detected")
        );
    }
}
