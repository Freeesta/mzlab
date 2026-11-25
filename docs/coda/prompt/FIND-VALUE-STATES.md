Read docs/coda/BRIEF.md.

# FIND-VALUE-STATES: Find the value states (detected, forced, filled, ...)

**Goal.** Make an already implemented feature findable: the AU-3 value states. This is a DISCOVERABILITY row: the feature exists, so the task is to find why a user or agent with the original problem did not reach it, and fix that cause.

**Why.** Standing rule of the owner: if something exists but was not used, it is poorly signalled or not intuitive. The request came from a lab report; the feature was already on main.

**Read first.** `mzlab-cli commands --find WORDS --json` (`crates/mzlab-cli/src/catalog/find.rs`; task words, `see_also` and `bulk` in `catalog/related.rs`; tests `catalog/find_tests.rs`), the task table of `docs/agenti/for-agents.md`, the `hint` of `error/1` (`docs/agenti/cli-agenti.md`), help examples. The AGENT-DISCOVERY work is merged: reuse this mechanism, never build a second one. `docs/agenti/value-states.md`.

**Already on main (checked 25 Nov, re-check before coding).** `state` words exist in the concentration outputs, `peak auto-lr`, `peak table --columns state`, `chrom.transitions` and the Dig detail. The running chat DETECT-VS-FORCED works on the feature-table side: start after its PR is merged, read its diff, and do not repeat it.

**Done means.** Steps: (1) write the question the user or agent had, in their own task words (no lab names), and run `commands --find` with those words, then read the output of the nearest wrong call: record where the trail goes cold. (2) Fix the cause, in this order of preference: a hint in the error or output that names the command to run next; task words and `see_also`/`bulk` in `related.rs`; a task-table line; a help example; the default made visible in the output. A rename only if the repo rules allow it (rename, never a backward-compat alias). (3) A test that the hint appears (`find_tests.rs` pattern: the task words find the command in the top results; an error or output carries the hint). No new command, no behaviour change. Task words to start from: 'forced integration', 'detected versus integrated', 'peak in blank', 'is this real', 'S/N'.

**Out of scope.** Changing the feature itself; new aliases; bytes of existing outputs (a hint is a new key or a stderr line, never a changed value).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
