//! The family `aperti`: formats with a public specification that the engine does not read (docs/agenti/lettori-aperti.md). One module per format, each with its `ENTRY`.

pub mod mzml_gz;
mod mzml_header;
pub mod mzml_numpress;
pub mod mzxml;
mod mzxml_entry;
mod mzxml_header;
mod mzxml_index;
mod mzxml_precursors;
mod mzxml_scan;
mod mzxml_window;
mod np_rewrite;
