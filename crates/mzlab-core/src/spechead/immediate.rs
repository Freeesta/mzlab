//! The rule that picks the immediate precursor of an MSn spectrum, shared by the scan table and the engine reader.

/// Which precursor of a level-`level` spectrum is the immediate one (the stage just before it): the one rule of every mzML path. `stages` gives the
/// target, selected m/z and `spectrumRef` of each precursor in file order (first stage to last). The one whose target or selected m/z is `path_last`
/// (the last stage of a filter path) within 0.01, else the last whose `spectrumRef` is a scan of level `level - 1`, else the last listed.
pub fn immediate(
    stages: &[(Option<f64>, Option<f64>, Option<&str>)],
    level: u8,
    path_last: Option<f64>,
    level_of: &dyn Fn(&str) -> Option<u8>,
) -> Option<usize> {
    if stages.is_empty() {
        return None;
    }
    let by_path = path_last.and_then(|last| {
        stages.iter().position(|(t, s, _)| {
            [t, s]
                .iter()
                .any(|v| v.is_some_and(|v| (v - last).abs() <= 0.01))
        })
    });
    let by_ref = || {
        stages.iter().rposition(|(_, _, r)| {
            r.is_some_and(|r| !r.is_empty() && level_of(r) == Some(level.wrapping_sub(1)))
        })
    };
    by_path.or_else(by_ref).or(Some(stages.len() - 1))
}
