# mzlab-picchi

Peak picking of one trace: baselines, Savitzky-Golay smoothing, ridge lines and the compact sorts they use.
A crate of its own so the release profile builds it at opt-level 3 while the rest of `mzlab-core` stays size-optimised; `mzlab-core` re-exports its modules.
Used by the engine, the CLI and the desktop app.
