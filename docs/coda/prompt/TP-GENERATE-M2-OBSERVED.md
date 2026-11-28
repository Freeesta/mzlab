Read docs/coda/BRIEF.md.

# TP-GENERATE-M2-OBSERVED: observed parent M+2/M for the `tp.generate` formula filter

**Goal.** Give the `m2_ref` of the `tp.generate` formula filter from the measured parent: the M+2/M of `raw isotopes` for the parent in a given file, read by the plan or passed with one switch, with the value and its source written in the registry `filter` block.

**Why.** The filter takes `m2_ref` as a typed number or falls back to the parent's formula; an observed ratio is the evidence the user wants, and typing it by hand loses the trail.

**Read first.** `docs/agenti/mzfeat-tp.md` (formula filter of `tp.generate`), `docs/agenti/raw-access.md` (`raw isotopes`), `crates/mzlab-feat/src/hypo/filter.rs`, `crates/mzlab-cli/src/flow_generate.rs`.

**Already on main (check before coding).** `formula_filter`, `filter_steps`, `m2_tol` and `m2_ref` are `--set` parameters, off by default; `m2_source` says `observed` or `parent_formula`.

**Done means.** 1. A way to take `m2_ref` from a `raw-isotopes/1` record (input port or `--set`), with the file and the record's sigma logged in `filter`. 2. Without it the output is unchanged. 3. Synthetic test; parity site = app = CLI (the wasm leg fails, not skips, in CI). 4. Entry point findable with `commands --find`.

**Out of scope.** Choosing the filter thresholds for the user; a high-resolution isotope finder.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
