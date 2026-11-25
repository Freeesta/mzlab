Read docs/coda/BRIEF.md.

# REPEAT-HINT-FLOW: Repeat hint for other repeated `flow run` plans

**Goal.** The repeat hint covers `peak auto-lr` and `flow run` of a plan with `tp.dig_lr`. Extend `rule_of` to other plans that are run one by one where a one-call route exists, and make the per-parent state work outside Unix (it is Unix-only today: no parent tag elsewhere, so no hint).

**Read first.** `crates/mzlab-cli/src/repeat_hint.rs`, `repeat_hint_tests.rs`, `tests/repeat_hint.rs`, `docs/agenti/for-agents.md` (Loop hint), `docs/agenti/cli-agenti.md`.

**Done means.** (1) List the plans that are looped in practice (ask `commands --find`, read `flow steps --compact`) and name the one-call route of each; add a `Rule` per plan family with its lang keys. (2) A Windows parent tag (process id and creation time) behind `cfg(windows)` with a test, or a stated technical limit. (3) Tests: hint appears once at the tenth call per rule; stdout and exit code unchanged.

**Out of scope.** Changing bytes of existing outputs (a hint is a new key or a stderr line, never a changed value); new aliases.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
