Read docs/coda/BRIEF.md.

# FIND-LOOP-XIC: Hint for one `trace xic` or `raw xic` call per target

**Goal.** An agent that calls `trace xic` or `raw xic` once per target in a loop is led to the one-call route. DISCOVERABILITY row: the one-call routes exist (`trace xic --mz LIST`, `raw xic`, `quant targets`).

**Read first.** `crates/mzlab-cli/src/repeat_hint.rs` (rules, `rule_of`, per-parent state, tests `repeat_hint_tests.rs`, `tests/repeat_hint.rs`), the `bulk` and `related` rows of `catalog/related.rs`, `docs/agenti/for-agents.md` (Loop hint). Start with `grep -n 'repeat' docs/agenti/INDICE.md`.

**Done means.** (1) A `Rule` for `trace xic` and one for `raw xic` with their own lang keys (`note.cli.repeat.*`, en + contesto), naming the one call. (2) Tests: the hint appears at the tenth call from one parent, never on stdout or in the exit code, and `MZLAB_NO_HINTS` silences it. (3) The doc line of the Loop hint lists the new commands. Reuse the mechanism, never build a second one.

**Out of scope.** Changing bytes of existing outputs (a hint is a new key or a stderr line, never a changed value); new aliases.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
