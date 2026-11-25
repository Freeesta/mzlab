Read docs/coda/BRIEF.md.

# SHAPE-GATES-OTHER: peak-shape gates and decision evidence beyond quant targets

**Goal.** Give the other integrators the optional peak-shape gates and the evidence log that `quant targets` has: `peak auto-lr`, `peak table --auto`, `chrom.transitions`, the Dig detail and `raw xic`.

**Why.** A weak noise peak (S/N 3 to 5) integrated in a blank or matrix control is reported as a value by every one of these routes; only `quant targets` can keep it `forced` with a stated reason. The same lab question comes back on each surface.

**Read first.** `docs/agenti/quant-targets.md`, `docs/agenti/value-states.md`, `crates/mzlab-feat/src/quant_targets/gates.rs` (the gates and `checks()`), `crates/mzlab-feat/src/value_state.rs`. Start with `grep -n 'value states\|quant targets' docs/agenti/INDICE.md`, then `grep -rn 'min_half\|max_asym' crates --include=*.rs -l`.

**Already on main (checked 25 Nov, re-check before coding).** `quant targets` has `--set min_half=N`, `max_fwhm=S`, `max_asym=R` (all 0 = off, byte-identical output when off), a `bad_shape` flag and the log `quant-targets-log/1` (`decisions[]` with six checks, `files[].detected_min`). `raw xic` carries the `state` word (PR #3720) but no gates. The other four routes have the `state` word and no gates.

**Done means.** 1. One shared gate implementation, reused from `quant_targets::gates` (no second copy); each route gets the three parameters with defaults off and the output unchanged by default. 2. A route that applies a gate keeps the peak as `forced` with its numbers and the failed check in its log. 3. Parity site=app=CLI per route, `--json` keys and the exit code of a bad value pinned, schemas, catalogs, a `find_tests.rs` task. 4. Module sizes in the PR body (`rawxic` has 80 B of cap margin: new site code goes to a module with room).

**Out of scope.** The aligned CSV and `export.mztab` (rows NA-CELL-STATE, MZTAB-CELL-STATE); changing the default behaviour of any route.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
