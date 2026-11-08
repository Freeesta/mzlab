# Build prompts of the negative / unit-resolution audit

The prompts that close the gaps of `docs/studi/MATRICE_NEG_LR.md`, in waves of at most three chats on separate files. Each prompt is self-contained (goal, owned files, do-not-touch, tests, PR text, common rules). Start a wave when the previous one is merged; every chat starts from `origin/main` after the audit PR (branch `claude/neg-lr-audit`) is merged, because the matrix and the `gap_*` tests of `crates/mzlab-cli/src/flow_neglr_tests.rs` live there.

| Wave | Prompt | Terminal | Gaps | Owned files |
|---|---|---|---|---|
| 1 | `01_ORDER.md` | `neg-lr-order` | NL-8 | `mzlab_core::campioni` (new `order_like`), `crates/mzlab-feat-table-wasm/src/core.rs`, `pol.rs::samples_v1`, the native readers of the roles, the `ORDER` line of `tests_e2e/e2e_flow_neg_lr.py` |
| 1 | `02_NEG_WEB.md` | `neg-lr-web` | NL-7 | `flow/lazy/flow-live-jobs.js`, `flow-live.js`, the shared nature rule (`crates/mzlab-cli/src/flow_nature.rs` or its new home), the `plan()` lines of `tests_e2e/e2e_flow_neg_lr.py` |
| 1 | `03_NEG_MGF.md` | `neg-lr-mgf` | NL-3 | `crates/mzlab-feat-table-wasm/src/rows_mgf.rs`, the other `recipe_polarity` call sites of spectrum writers |
| 2 | `04_NEG_DEFAULTS.md` | `neg-lr-defaults` | NL-1, NL-2 | `crates/mzlab-cli/src/flow_estimate.rs`, `crates/mzlab-feat/src/flow_formula_ms1.rs`, the formula.ms1 part of `flow/lazy/flow-web-ident.js` |
| 2 | `05_MIX_GUARD.md` | `neg-lr-mix` | NL-4, NL-5 | `crates/mzlab-cli/src/flow_base.rs`, the browser recipe (`run_meta.rs`), the align error path of `flow-live-jobs.js`; coordinated with LR-TOL |
| 2 | `06_FACETS.md` | `neg-lr-facets` | NL-6 | `crates/mzlab-feat/src/steps/` (descriptors), `crates/mzlab-core/src/flow/check.rs`, `flow/lazy/flow-check.js`, `flow-page.js` |
| 3 | `07_LR_MS2_TOL.md` | `neg-lr-ms2tol` | NL-11, NL-12, NL-13 | MS2 match and QC descriptors and executors, `filterspec.rs`; after LR-TOL |
| 3 | `08_INCLUSION.md` | `neg-lr-inclusion` | NL-10 | `crates/mzlab-cli/src/flow_inclusion.rs`, its descriptor, a browser executor if added |

Wave 1 shares one file: `tests_e2e/e2e_flow_neg_lr.py`, where 01 changes only the `ORDER` line and 02 only `plan()` and the `UNWRITTEN` lines (separate hunks). 05 also touches `flow-live-jobs.js` after 02 is merged. Gaps owned by other chats (LR-TOL, LR-NOISE, P3, P4) are not in these prompts.
