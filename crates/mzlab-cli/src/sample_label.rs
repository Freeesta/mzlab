//! The label of a sample of a multi-sample file: the rule lives in `mzlab_core::readers::label` (one function for the CLI, the app and the site); this keeps the
//! names the CLI modules and their tests use.

pub(crate) use mzlab_core::readers::label::{labels, Kind, Member};

#[cfg(test)]
#[path = "sample_label_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "features_label_tests.rs"]
mod cli_tests;
