//! The `.wasm` modules of the page that read files outside the engine, one per feature: the readers of the makers' formats (`lettura`, one module per family) and `valida`, the mzML
//! validator of `mzlab-cli validate` (its own module, loaded only when the user validates a file: the PSI-MS table keeps it out of `aperti`).
#[cfg(feature = "lettura")]
mod lettura;
#[cfg(feature = "lettura")]
pub use lettura::*;
#[cfg(feature = "valida")]
mod valida;
