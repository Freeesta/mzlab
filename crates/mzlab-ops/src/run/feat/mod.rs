//! Plain modules moved from `mzlab-feat-wasm`, bodies unchanged: the steps of the pipeline over any `ReadAt` (`run`, with the Flow meta as `run::meta`), the proofs
//! evidence, the state of the proofs, the estimate of a file, the hypotheses (`run_tp`, with its Dig inputs), the cache key, the family record, the interpretation of the rows (`interp`) and the untargeted pipeline. The JS exports of the bridge stay there.

#[cfg(any(feature = "detect", feature = "untargeted"))]
pub mod cache;
pub mod estimate;
#[cfg(feature = "detect")]
pub mod family;
#[cfg(feature = "table")]
pub mod interp;
pub mod run;
#[cfg(feature = "detect")]
pub mod run_estimate;
pub mod run_evid;
pub mod run_tp;
#[cfg(feature = "proofs")]
pub mod state;
#[cfg(feature = "untargeted")]
pub mod untargeted;
