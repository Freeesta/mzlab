Read docs/coda/BRIEF.md.

# FLOW-RECIPE-FIRST: One recipe estimate rule for `flow run` and the browser runner

**Goal.** `flow run`, the desktop `run_plan` and the browser runner give the same bytes for a unit-resolution series whose files have different estimated floors or mass defects: one rule, the per-file recipe, named in the record.

**Why.** Found in FEATURES-NOMINAL: `detect_job` (`crates/mzlab-cli/src/flow_base.rs`) estimates the recipe of each file, while the browser runner (`recipe_of_layers`) and `rebuild` estimate it once from the first file. On a series whose estimates differ the three routes write other bytes; the parity tests only cover series with one estimate.

**Read first.** `docs/agenti/mzfeat-lr.md` (profile choice), `crates/mzlab-ops/src/run/feat/run_meta.rs`, `crates/mzlab-cli/src/flow_base.rs`, `crates/mzlab-cli/src/flow_nominal_tests.rs` (the fixture that works around it with three files of one seed). Start with `grep -n 'recipe_for_estimated' docs/agenti/INDICE.md`, then `grep -rn 'recipe_for_estimated' crates --include=*.rs`.

**Already on main (checked 26 Nov, re-check before coding).** The difference exists; no test pins it.

**Owner decision (26 Nov: "fino alla 3 ok").** The per-file recipe everywhere: each file of a unit-resolution series gets its own estimate (as `detect_job` does today), in `flow run` (CLI), the desktop `run_plan`, the browser runner and `rebuild`. Bytes change only for batches whose files' estimates differ; a one-estimate series stays identical. Log every automatic choice: which recipe, estimated from which file. If main shows a route not as described above, apply the same rule there and say so in the PR body.

**Done means.** 1. A synthetic series of unit-resolution files with different noise floors (`tools/dati_sintetici.py`) whose estimates differ, that fails today in a parity test (site = app = CLI, byte for byte). 2. The per-file rule applied in every route, `rebuild` included, the estimate of each file listed in the record. 3. The fixture of `flow_nominal_tests.rs` no longer needs identical seeds. 4. The jobs that depend on the first index (`sched_graph.rs`) follow the new rule.

**Out of scope.** High-resolution estimates; the parameters of the profiles.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
