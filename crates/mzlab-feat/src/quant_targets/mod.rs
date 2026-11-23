//! `quant.targets@1`: a list of (m/z, RT window) targets integrated over many files in one pass per file. Each file's index is swept once per polarity
//! (`Routes::sweep_matrix`, every target's XIC at once, the traces `trace xic` prints), each trace goes through the peak engine of `peak auto-lr`
//! (`peakq_lrauto::lr_auto_integrate`) and its shape through `peakq::quality`. `prepare` checks the run, `Job::file` is the pass over one file
//! (independent, so callers may run files on threads) and `Job::finish` joins them in file order: the bytes do not depend on the schedule.

mod params;
mod run;
mod targets;
mod text;

pub use params::{OffsetMode, Params};
pub use run::{plan, prepare, run, FilePass, Job, Outcome, MAX_WORKERS};
pub use targets::{parse, Role, Target};
pub use text::HEADER;

#[cfg(test)]
mod tests;
