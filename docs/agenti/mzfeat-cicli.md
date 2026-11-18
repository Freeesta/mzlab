# Cycles between `mzlab-feat` modules: map and cut plan
Graph of the top-level modules of `crates/mzlab-feat/src` (those in `lib.rs`, with all features on) and a plan to cut its big cycle without touching the body of any algorithm: simple types and small functions that create the back edges move down, and the old public paths `mzlab_feat::…` stay through a `pub use`. The result bytes do not change; the proof is the parity tests.

## How it is measured
- Node = top-level module (`X.rs` plus `X/**`); edge X → Y = a path `crate::Y` or `super::Y` (also inside `use crate::{…}`) in a file of X, excluding comments, strings, files and blocks under `#[cfg(test)]` and the `*_tests.rs`. Lines = lines of the module's code files, without the tests.
- Cycle = strongly connected component (Tarjan). A `pub use` that only keeps the old public path of a moved item does not count as an edge (no code depends on the module above); these `pub use` are listed in the PR of the cut and stay as long as callers use the old path.
- The script is throwaway (not a repository tool): its result is pasted in the body of each PR, before → after.

## The cycle today
- **50 modules, 43 369 lines** (out of 115 modules and 108 958 lines of code). `attributio` (27 772 lines) is outside it: it depends on `aligned`, `rowident`, `untargeted` and `interp`, no module of the cycle depends on it. Other cycles: 4 modules (`weakfile`, `slabs`, `spill`, `series`, 1 115 lines), 2 (`suspects`, `suspects_listings`). Outside the cycle: `chem`, `steps`, `hypo`, `axis`, `aligned_meta`, `kinetic` and the others.
- Members, lines: `legacy` 15 177 · `untargeted` 5 476 · `interp` 3 421 · `evid` 1 932 · `tp` 1 725 · `family` 1 503 · `groups` 1 396 · `transitions` 746 · `network` 692 · `lists` 673 · `broad` 554 · `rtwarp` 550 · `params` 493 · `quality` 487 · `store_read` 459 · `store` 457 · `filters` 447
- (continued) `filters_blank` 445 · `centwave` 421 · `resolve` 412 · `ms2spec` 405 · `align` 389 · `trace` 385 · `sweep` 362 · `broad_gt` 334 · `gapfill` 320 · `famalign` 315 · `channels` 300 · `ms2` 285 · `rtmap` 249 · `isotopes` 242 · `norm` 234 · `export` 231 · `post` 208 · `recipe` 196 · `qc` 159 · `pipeline` 148
- (continued) `table` 140 · `aligned` 132 · `rowmeta` 130 · `broad_blank` 130 · `view` 124 · `filters_nts` 81 · `rowident` 78 · `export_ms` 74 · `lrnoise` 59 · `broad_front` 58 · `params_unit` 47 · `params_height` 45 · `report` 43.
- Hubs: `trace` and `table` have 24 incoming modules each, `aligned` 14; `legacy` 7 incoming and 9 outgoing. `table` re-exports `aligned` (`pub use crate::aligned::*`), so every edge that climbs from `aligned` ties the 24 users of `table` to the cycle.

## What a light build compiles today
- `attributio` already sits behind the `attributio` feature: `mzlab-feat-wasm --no-default-features --features untargeted` (wasm32, release) reads 305 files and 62 883 lines of `mzlab-feat`, none of `attributio/` (dep-info); `flow_tab` and `flow_stat` (same features as `mzlab-feat`, a single unit) 280 files and 57 176 lines.
- After a change to `attributio/mod.rs` the three builds do not recompile `mzlab-feat` (0.1-1.6 s); after a change to `legacy.rs` they recompile it in 40 s (4 CPU). The weight of the light builds is the rest of the node: `legacy` 12 460 lines read, `untargeted` 5 475, `steps` 4 039, `evid` 1 932, `tp` 1 725.
- Closure of the modules that use `run_tab.rs` and `run_stat.rs` (`flow_drift`, `flow_export_annot`, `flow_stat`, `ionnet`, `methods`, `plausible`, `runorder`, `transitions`): 80 modules, 86 938 lines in the graph with all features; after cuts 1-8 below: 68 modules, 46 267 lines, still pulled in by `transitions` → `legacy` and `groups` → `legacy`.

## Back edges, by how many modules they keep in the cycle
Removing a single edge from the graph before cut 1 (51 modules, 71 387 lines), the biggest cycle becomes (modules, lines):

| Edge | Items used | After |
|---|---|---|
| `recipe` → `pipeline` | `Steps` | 44, 69 941 |
| `pipeline` → `lrnoise` | `reject` | 47, 70 743 |
| `lrnoise` → `params` | `median`, `strongest` | 48, 70 802 |
| `legacy` → `network` | `arcs` (`best_pairs`, `counts`, `neutral_mass`) | 49, 67 222 |
| `legacy` → `lists` | `from_legacy_text` | 49, 69 968 |
| `table` → `aligned` (that is `aligned` → `tp`, `rowident`) | `*` | 49, 70 998 |
| `network` → `interp` | `RowInterp`, `store` | 50, 67 914 |
| `untargeted` → `attributio` and `interp` → `attributio` (together) | `to_attributio`, `ms1_window_metrics` | 50, 43 615 |
| `sweep`, `channels` … → `trace` | `filled`, `reserve`, constants | 50, ~71 000 each |

Removing the edges one after another (the best at each step): `recipe` → `pipeline` (44), `legacy` → `network` (42), `trace` → `untargeted` (40 modules, 32 334 lines), `table` → `aligned` (22), `store` → `recipe` (11), `legacy` → `lists` (10), then local cycles ≤ 9.

## Cut plan (one PR per cut, ≤ ~600 changed lines)
Numbers = biggest cycle after the cut, cumulative in the order below (simulated on the graph; the PR measures on the code).

1. **`attributio` out of the cycle** (done: 51 → 50 modules, 71 387 → 43 369 lines). The `untargeted` table as entry of the cascade lives in `attributio/from_untargeted.rs` (`#[cfg(feature = "untargeted")]`), `ms1_window_metrics` in `attributio/review/window.rs` (`#[cfg(feature = "interp")]`); the old paths `untargeted::to_attributio` and `interp::ms2link::ms1_window_metrics` stay through a `pub use` (comment "The old path of"), used by `tests/untargeted_to_attributio.rs` and `mzlab-cli/src/flow_chimeric.rs`.
2. **`aligned` becomes a base** (→ 46, 42 750). The struct `TpBlock` with `TpParams`, `RoleCounts`, the constants `NO_PARENT`, `ST_*`, `NOTE_*`, `VIA_*`, `KIN_*` (from `tp/mod.rs`) and `FitCandidate` (from `tp/candidates.rs`) go into the new module `tp_block.rs`, with no dependencies; the struct `RowIdent` and `SRC_DETECTED` go into `aligned_meta.rs` (`rowident.rs` keeps `impl RowIdent { compute… }`, `adduct_names`, `code_of`). Changed: `tp/mod.rs`, `tp/candidates.rs`, `tp_block.rs`, `aligned.rs`, `aligned_meta.rs`, `rowident.rs`, `lib.rs`.
3. **`pipeline::Steps` down** (→ 42, 42 008). `Steps` with its `Debug` and `Default` goes into `pipeline_steps.rs` (depends only on `quality`, `isotopes`, `ms2`, `resolve`, `align_mzmine`); `pipeline.rs` re-exports it. Changed: `pipeline.rs`, `pipeline_steps.rs`, `recipe.rs`, `lib.rs`.
4. **`network::arcs` down** (→ 39, 36 228). `git mv crates/mzlab-feat/src/network/arcs.rs crates/mzlab-feat/src/netarcs.rs` (82 lines, uses only `hypo`); `network/mod.rs` has `pub use crate::netarcs as arcs;`. Changed: `network/mod.rs`, `legacy/network.rs`, `lib.rs`.
5. **Base of `trace`** (→ 33, 34 210). `reserve`, `filled` and the constants `SLAB_PEAKS`, `BYTES_PER_PEAK`, `FLOOR`, `MIN_HEIGHT`, `TOUCH_HEIGHT` go into `trace_base.rs` (only `mzlab_core::error`); `trace.rs` re-exports them; `Params` stays in `trace` (it carries `centwave::Peak`). Changed: `trace.rs`, `trace_base.rs`, `lib.rs` and the ~30 files that import only those items from `crate::trace` (`sweep`, `isotopes`, `broad*`, `post`, `rowmeta`, `align`, `famalign`, `slabs`, `tp`, `untargeted/*`, `family/roles.rs`, …).
6. **`trace` → `untargeted`** (→ 13, 23 327): the ADAP route of `trace::detect_with` calls `untargeted::flow::detect`, which uses `untargeted::pipeline`. Ways out, to be chosen by reading `untargeted::pipeline`: the part of `untargeted` that `flow` needs moves down into a module under `trace`, or the choice of route passes to the caller (`pipeline`, `series`) as a parameter.
7. **`store` → `recipe`** (→ 9, 22 421): `recipe::fingerprint` (a pure function) moves down into `store`/base and the function of `store.rs` that reads a `&Recipe` receives the texts already prepared.
8. **`family` → `legacy`** (→ 9, 2 847): `legacy::edge::{Iimn, ISOTOPES}` used by `family/rel.rs` move down into a base module.

After 8, local cycles of ≤ 9 modules remain: `trace`, `centwave`, `channels`, `broad*`, `post`, `quality` (2 847 lines); `legacy`, `groups`, `lists`, `transitions` (17 992); `store`, `store_read`, `evid`, `rowident` (2 926); `weakfile`, `series`, `spill`, `slabs`; `filters*`; `params*`.

## After the cuts: `legacy` behind a feature
- No new crate for the attribution engine: `attributio` already sits behind its feature and the light builds read none of its files, so a separate crate takes nothing away from them.
- After the cuts inside the crate, `legacy` (12 460 lines read by `untargeted`) goes behind a cargo feature, turned on only where low resolution is needed: first the edges `transitions` → `legacy::lr` and `groups` → `legacy::lr` are cut, through which `flowtab` and `flowstat` reach `legacy`. A split into crates is done only if a measurement shows a saving that the feature does not give.
- Without real LTO the margins under the cap are a few KB (`hr`, `table`): each PR measures every `.wasm` module before and after.
