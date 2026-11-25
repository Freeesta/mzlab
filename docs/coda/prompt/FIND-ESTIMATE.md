Read docs/coda/BRIEF.md.

# FIND-ESTIMATE: Find estimate, features.union and raw xic as entry points

**Goal.** Make an already implemented feature findable: `estimate` (measured parameters), `features.union`/`quant union` (one target list from many series), `raw xic` (a verdict at a mass and time). This is a DISCOVERABILITY row: the feature exists, so the task is to find why a user or agent with the original problem did not reach it, and fix that cause.

**Why.** Standing rule of the owner: if something exists but was not used, it is poorly signalled or not intuitive. The request came from a lab report; the feature was already on main.

**Read first.** `mzlab-cli commands --find WORDS --json` (`crates/mzlab-cli/src/catalog/find.rs`; task words, `see_also` and `bulk` in `catalog/related.rs`; tests `catalog/find_tests.rs`), the task table of `docs/agenti/for-agents.md`, the `hint` of `error/1` (`docs/agenti/cli-agenti.md`), help examples. The AGENT-DISCOVERY work is merged: reuse this mechanism, never build a second one. `docs/agenti/estimate.md`, `docs/agenti/quant-targets.md`, `docs/agenti/raw-access.md`. Start with `grep -n 'estimate\|quant targets' docs/agenti/INDICE.md`, then `grep -n 'estimate\|union' crates/mzlab-cli/src/catalog/related.rs`.

**Already on main (checked 25 Nov, re-check before coding).** All three are merged. Check that the error of a step with a missing or failing parameter (`flow check`) points to `estimate`, and that `llm params` and the recipe in `docs/agenti/recipes.md` name `features.union` and `raw xic`.

**Done means.** Steps: (1) write the question the user or agent had, in their own task words (no lab names), and run `commands --find` with those words, then read the output of the nearest wrong call: record where the trail goes cold. (2) Fix the cause, in this order of preference: a hint in the error or output that names the command to run next; task words and `see_also`/`bulk` in `related.rs`; a task-table line; a help example; the default made visible in the output. A rename only if the repo rules allow it (rename, never a backward-compat alias). (3) A test that the hint appears (`find_tests.rs` pattern: the task words find the command in the top results; an error or output carries the hint). No new command, no behaviour change. Task words to start from: 'which parameters', 'noise floor', 'what threshold', 'ppm', 'targets from many series', 'is there a peak at this mass', 'how many ions'.

**Out of scope.** Changing the feature itself; new aliases; bytes of existing outputs (a hint is a new key or a stderr line, never a changed value).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
