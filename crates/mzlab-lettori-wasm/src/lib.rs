//! The `.wasm` modules of the page that read files outside the engine, one per feature: the readers of the makers' formats (`lettura`, one module per family) and `valida`, the mzML
//! validator of `mzlab-cli validate` (its own module, loaded only when the user validates a file: the PSI-MS table keeps it out of `aperti`).
#[cfg(feature = "lettura")]
mod reading;
#[cfg(feature = "lettura")]
pub use reading::*;
#[cfg(feature = "valida")]
mod validate;
