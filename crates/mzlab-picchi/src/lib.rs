// Peak picking of one trace (baselines, Savitzky-Golay, ridge lines) and the compact sorts it uses.
// A crate of its own so Cargo.toml builds it at opt-level 3 while the rest of mzlab-core stays at "z";
// mzlab-core re-exports the modules, so every public path is unchanged.
pub mod peaks;
pub mod signal;
pub mod sortc;
