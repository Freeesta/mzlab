Read docs/coda/BRIEF.md.

# UNION-POLARITY-NODE: A page node of features.union with a polarity split

**Goal.** The Flow page node of `features.union` splits by polarity so one plan serves both polarities.

**Why.** The step has a `polarity` parameter; the page has no way to run both (follow-up of #3710).

**Read first.** `flow/lazy/flow-web-union.js`, `docs/agenti/flow-web.md`, `docs/agenti/quant-targets.md`.

**Already on main (checked 25 Nov, re-check before coding).** The step and its web runner are merged; `polarity: any|+|-` is a parameter.

**Done means.** A node that emits one union per polarity from one table list, with `tests_e2e/` coverage by `python3 tools/verifica.py --changed`; text through `lang/` keys; no logic in JS.

**Out of scope.** Engine changes.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
