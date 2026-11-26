Read docs/coda/BRIEF.md.

# BIOTRANSFORMER-IO: BioTransformer by file exchange

**Goal.** Export the parent as a BioTransformer input file; import its output as hypotheses of `tp.generate` with provenance. mzLab bundles nothing and calls no network.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 19 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md`.

**Read first.** `tp.generate` user lists, `docs/studies/TRANSFORMATION_RECORD.md` (licence rule).

**Already on main (checked 26 Nov, re-check before coding).** User hypothesis lists exist.

**Done means.** 1. Export and import in Rust, tested on a synthetic output file. 2. Site, app, CLI the same bytes. 3. Agent docs.

**Out of scope.** Running BioTransformer.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
