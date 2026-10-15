//! The family `aperti`: formats with a public specification that the engine does not read (docs/agenti/lettori-aperti.md). One module per format, each with its `VOCE`.

pub mod mzml_numpress;
mod mzml_testata;
pub mod mzxml;
mod mzxml_indice;
mod mzxml_scan;
mod mzxml_testata;
mod np_riscrivi;
