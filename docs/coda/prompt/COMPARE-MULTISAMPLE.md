Read docs/coda/BRIEF.md.

# COMPARE-MULTISAMPLE: table compare with multi-sample .wiff files

**Goal.** `table compare` on tables from multi-sample `.wiff` files gives one column per sample, named by the sample, not one name repeated from the file.

**Why.** Columns take the file name instead of the sample name, so every sample of a file collides and the result is unusable.

**Read first.** `crates/mzlab-cli/src/table_compare_cmd.rs`, `crates/mzlab-feat/src/flow_compare.rs` (reads `datafile:<name>:area` columns; roles by name with or without `.mzML`, else SHA-256), `docs/agenti/wiff-sample-choice.md`, `docs/agenti/samples.md`.

**Already on main (checked 25 Nov, re-check before coding).** `table compare` takes `--samples samples/1` and selectors `role:` and `time:`. Reproduce first on a synthetic two-sample `.wiff` (`tools/wiff_riferimento.py`) with `features` then `table compare`; find where the column name is made (alignment or feature export).

**Done means.** 1. A failing test, then the fix at the source of the names (the feature table column for a sample of a multi-sample file carries file and sample, unambiguous and stable). 2. `table compare --dry-run` shows the columns it found. 3. Existing single-sample bytes unchanged. Parity site=app=CLI.

**Out of scope.** Selecting samples (exists: `sample select`).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
