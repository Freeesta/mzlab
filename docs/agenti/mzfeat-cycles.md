# Layering of the `mzlab-feat` modules
Top-level modules of `crates/mzlab-feat/src` (those declared in `lib.rs`) and the low-level base modules that keep them from depending on each other in a cycle. Moved items keep their old public path `mzlab_feat::…` through a `pub use`, so callers and result bytes do not change; the parity tests are the proof.

## Base modules (no dependency on the modules above them)
- `trace_base.rs`: `reserve`, `filled` (allocations that refuse with `error.memoryBudget`) and the slab constants `SLAB_PEAKS`, `BYTES_PER_PEAK`, `FLOOR`, `MIN_HEIGHT`, `TOUCH_HEIGHT`. It depends only on `mzlab_core::error`; `trace.rs` re-exports it and keeps `Params`, which carries the `centwave::Peak`.
- `tp_block.rs`: `TpBlock` with `TpParams`, `RoleCounts`, the constants `NO_PARENT`, `ST_*`, `NOTE_*`, `VIA_*`, `KIN_*` and `FitCandidate`; `tp/` uses it.
- `aligned_meta.rs`: `RowIdent` and `SRC_DETECTED`; `rowident.rs` keeps `impl RowIdent { compute… }`, `adduct_names`, `code_of`. `table.rs` re-exports `aligned` (`pub use crate::aligned::*`).
- `pipeline_steps.rs`: `Steps` with its `Debug` and `Default`; `pipeline.rs` re-exports it and `recipe.rs` imports it from here.
- `netarcs.rs`: the arcs of the network (`best_pairs`, `counts`, `neutral_mass`), which uses only `hypo`; `network/mod.rs` has `pub use crate::netarcs as arcs;`.
- `store_base.rs`: magic, alignment, version, error, origin and the pieces of the `.mzfeatf` and `.mzfeat` containers; `store` (writer) re-exports it and `store_read` and `evid` depend on it, not on the writer.
- `finder_base/` (fragmentation and MSn trees, MS2, identity graph) and `transf_base.rs`: the shared base of `legacy` and `interp`.

## Cargo features (what a light build compiles)
- `attributio`, `untargeted`, `interp`, `legacy`, `flow_formula` and `chem` gate whole modules in `lib.rs`; `legacy` is on by default and the finder `.wasm` that need it turn it on, so the other `.wasm` do not compile `legacy/`.
- The table `untargeted` as entry of the attribution cascade lives in `attributio/from_untargeted.rs` and `ms1_window_metrics` in `attributio/review/window.rs`; the old paths `untargeted::to_attributio` and `interp::ms2link::ms1_window_metrics` stay through a `pub use`.
- No separate crate for the attribution engine: it already sits behind its feature, so a split would take nothing away from the light builds. A split into crates is warranted only if a measurement shows a saving that the feature does not give.
- Without real LTO the margins under the `.wasm` cap are small, so each change measures every `.wasm` module before and after (`tools/moduli/*.toml`).

## Rules for a new edge
- A simple type or small function that two layers need goes into a base module, not into the upper one; the upper module re-exports it.
- `trace`, `table`, `aligned` and `legacy` are hubs with many users: an edge from them to a module that depends on them back ties all those users into one cycle.
- Edges checked by `grep 'crate::<module>\|super::<module>'` on the module's files, excluding `#[cfg(test)]` code and `*_tests.rs`; a `pub use` that only keeps an old public path does not count as an edge.
