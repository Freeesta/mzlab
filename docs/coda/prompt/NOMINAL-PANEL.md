Read docs/coda/BRIEF.md.

# NOMINAL-PANEL: The `nominal` profile in the Pipeline panel of the site

**Goal.** The Pipeline panel of the site lets the user choose the `nominal` profile of `features.detect`, so a run from the page gives the same bytes as `flow run --set detect.preset=nominal` and `features --params '{"preset":"nominal"}'`.

**Why.** The profile exists in the engine, the Flow, the CLI and `flow defaults`, but the panel has only the numbers (`floor`, `min_height`, `ppm`, `rt_tol`) and the unit-resolution switches, so a user of the page cannot reach it.

**Read first.** `docs/agenti/mzfeat-lr.md` (profile choice, preset `nominal`), `crates/mzlab-ops/src/run/feat/run_preset.rs` (the `preset` key of the recipe text), the Pipeline panel code (`grep -rln 'recenter' mzlab/web --include=*.js`). Start with `grep -n 'pipeline panel\|recipe text' docs/agenti/INDICE.md`.

**Already on main (checked 26 Nov, re-check before coding).** `recipe_on` accepts `preset: "nominal"`; the panel does not write it. The label goes through `lang/en` keys.

**Done means.** 1. A control in the panel (a `lang/` label and hint), shown for unit-resolution files only, that writes the `preset` key. 2. An e2e test in `tests_e2e/` and a byte test that the page recipe equals the CLI run (extend `tests/test_feat_nominal_wasm.py`). 3. The text says which values the profile writes (from `flow defaults features.detect`).

**Out of scope.** Choosing the profile from the data (`features.estimate` names it with its numbers and never applies it); changing the HR defaults.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
