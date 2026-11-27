Read docs/coda/BRIEF.md.

# INSTRUMENT-PRESETS: Presets named after the instrument

**Goal.** Presets named after the instrument for the detection steps: unit resolution +-0.5 Da and Orbitrap +-5 ppm (owner's values), beside the existing `lr` and `hr`. `features.estimate` may propose other values; the choice is logged as `auto` or `user`.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 12 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md`.

**Read first.** `docs/schemi/steps-1.json` (`presets` of `features.detect`), `docs/agenti/estimate.md`, `docs/agenti/cli.md` (`--preset`).

**Already on main (checked 26 Nov, re-check before coding).** Presets `lr` and `hr` exist, chosen by resolution.

**Done means.** 1. The presets in the step descriptors (regenerate the schema with its tool). 2. Existing defaults unchanged. 3. `commands --find` test and agent docs.

**Out of scope.** Other instruments.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
