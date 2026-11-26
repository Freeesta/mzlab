//! `commands --find` for the ion scale of a trace (`trace xic --ions`, `peak noise --ions`): the words of the task reach the command that has it.

use super::{search, words};

#[test]
fn the_ions_of_a_trace_and_its_noise_are_found_from_the_task() {
    for (q, want) in [
        (
            "the extracted ion chromatogram in ions per point with the dwell or fill time of the scan type",
            "trace xic",
        ),
        (
            "noise of an ion counting trace with a floor of one ion",
            "peak noise",
        ),
    ] {
        let got: Vec<String> = search(&words(q))
            .into_iter()
            .take(3)
            .map(|(it, _, _)| it.name)
            .collect();
        assert!(got.iter().any(|g| g == want), "{q:?}: {want} not in {got:?}");
    }
}
