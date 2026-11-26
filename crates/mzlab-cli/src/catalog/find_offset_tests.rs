//! `commands --find` for a mass axis that is off by a fraction of a Da and a UV trace that lags the MS: the flag and the estimate that exist
//! (`--mz-offset`, `dad delay`) are among the first three answers.

use super::{search, words};

const TASKS: &[(&str, &str)] = &[
    ("mass shift", "trace xic"),
    ("mass calibration", "trace xic"),
    ("mass shift", "features"),
    ("calibrate m/z of the feature table", "features"),
    ("wrong mass in the xic, the peak is shifted", "trace xic"),
    ("mass shift in raw xic", "raw xic"),
    ("time offset between uv and ms", "dad delay"),
];

#[test]
fn a_mass_shift_or_a_uv_lag_finds_the_flag_and_the_estimate_among_the_first_three() {
    for (task, want) in TASKS {
        let got: Vec<String> = search(&words(task))
            .into_iter()
            .take(3)
            .map(|(it, _, _)| it.name)
            .collect();
        assert!(
            got.iter().any(|g| g == want),
            "{task:?}: {want} not in {got:?}"
        );
    }
}
