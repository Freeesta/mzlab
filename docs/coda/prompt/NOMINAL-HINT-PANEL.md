Read docs/coda/BRIEF.md.

# NOMINAL-HINT-PANEL: Show the nominal-resolution hint beside the profile choice in the Pipeline panel

**Goal.** When the Pipeline panel of the site holds unit-resolution files that show the signature of the `nominal` profile, it says so beside the profile choice, with the numbers of the `nominal` member of `estimates/1` (candidate rows, share between the two heights, share of ions split across the grids, the cuts). Nothing is applied by itself.

**Why.** `features.estimate`, `estimate --json` and `features --report` name the signature, but a user of the page never runs them; the panel is where the profile is chosen.

**Read first.** `docs/agenti/estimate.md` (the nominal-resolution hint), `docs/agenti/mzfeat-lr.md` (preset `nominal`), `crates/mzlab-ops/src/run/feat/nominal_note.rs`, `docs/agenti/mzfeat-wasm-panel.md` (the `nominal` switch: `#pipe-nominal`, `PIPEFORM.nominalHtml`). Start with `grep -n 'pipeline panel' docs/agenti/INDICE.md`.

**Already on main (checked 26 Nov, re-check before coding).** `flow_estimate` of the `.wasm` writes the `nominal` member; the panel does not read it.

**Done means.** 1. A line under the profile choice (a `lang/` text with the numbers as parameters), only for unit-resolution files with `nominal.hint` true. 2. An e2e test in `tests_e2e/` on a synthetic series. 3. The numbers shown equal those of `estimate --json` for the same files (byte test, extend `tests/test_estimate_nominal_wasm.py`).

**Out of scope.** Applying the profile automatically; changing the thresholds' defaults.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
