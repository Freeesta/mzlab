Read docs/coda/BRIEF.md.

# MULTI-TARGET: One call for many targets

**Goal.** Tens of thousands of separate `peak auto-lr` calls and 23 Dig runs one by one become ONE multi-target call: extract many m/z targets from many files and return the peak table in one answer.

**Why.** Measured in the lab (the numbers live in the private repo: ask the coordinator for them, never paste private data): the per-call overhead dominates the run.

**Read first.** `docs/agenti/quant-targets.md` (`quant targets`, `quant union`), `docs/agenti/raw-access.md` (`raw xic --mz LIST`, at most 1000 masses), `docs/agenti/for-agents.md` (the kinetic chain, the 9-call recipe in `docs/agenti/recipes.md`), `crates/mzlab-cli/src/trace_xic.rs` (`trace xic --mz LIST`, `--sweep`).

**Already on main (checked 25 Nov, re-check before coding).** `trace xic --mz LIST`, `raw xic`, `quant targets` and `features.union` are merged. First MEASURE on a synthetic campaign (many targets x many files): which existing route already does it in one call and where the per-call cost remains (Dig runs, `peak auto-lr` per trace). Reuse; do not add a fourth way to extract targets.

**Done means.** 1. A written measurement (calls, seconds, on synthetic data) before and after, in the PR body. 2. The missing one-call path: for Dig, a plan or step that runs all hypotheses in one pass; for traces, `peak table --traces ... --auto` over a trace set. 3. Output equals the many-call output byte for byte (test). Parity site=app=CLI.

**Out of scope.** Caching of raw indexes (exists: `file index --cache`).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
