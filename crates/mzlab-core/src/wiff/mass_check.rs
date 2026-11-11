//! The method tables of a `.wiff` sample checked before its data are read: each `MassRangeEx` must hold exactly the entries its `ExperimentHeader`
//! announces (counted grammar, `methodinfo::mass_range`). A mismatch refuses the sample with `err.file.wiffMassRange`: transitions found by a guess
//! could be paired with the wrong values, or some left out, unsaid.

use super::Experiment;
use crate::error::{Error, Result};
use crate::methodinfo::mass_range;
use crate::ole::Ole;

/// Checks the tables of the experiments of method `no` that announce a count (a header without one is read as before).
pub(super) fn check(ole: &Ole, no: u32, experiments: &[Experiment]) -> Result<()> {
    for i in 0..experiments.len() {
        let base = format!("MethodSubtree/Method{no}/DeviceMethod0/Period0/Experiment{i}");
        let Some(n) = ole
            .read(&format!("{base}/ExperimentHeader"))
            .and_then(|h| mass_range::header_count(&h))
            .filter(|n| *n > 0)
        else {
            continue;
        };
        if let Some(d) = ole.read(&format!("{base}/MassRangeEx/MassRangeEx")) {
            mass_range::parse(&d, n).map_err(|why| {
                Error::new("err.file.wiffMassRange")
                    .with("experiment", i + 1)
                    .with("expected", n)
                    .with("detail", why)
            })?;
        }
    }
    Ok(())
}
