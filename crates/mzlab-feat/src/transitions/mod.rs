//! The transition table of a triple quadrupole acquisition (SIM: Q1 only; MRM/SRM: Q1 > Q3) and its integration, the step `chrom.transitions@1`.
//! The table is a CSV the user edits (`table.rs`), each row matched to a chromatogram of each mzML at unit resolution (`traces.rs`), integrated inside
//! its RT window with `legacy::lr::peak1d::find_peak`, or searched for candidate peaks when it has no RT yet (`text.rs`). Bytes in, text out: `flow run`
//! (`mzlab-cli`) and the browser module `flowtab` (`mzlab-feat-wasm`) both call `transitions_text`, so they write the same bytes.

mod route;
mod table;
mod text;
mod traces;

pub use route::route;
pub use table::{parse, to_csv, Row, HEADER};
pub use text::{channels_text, transitions_packed, transitions_text, Output, Settings};
pub use traces::{channels, Channel};

#[cfg(test)]
mod tests;
