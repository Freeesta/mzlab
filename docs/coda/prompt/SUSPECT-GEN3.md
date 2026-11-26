Read docs/coda/BRIEF.md.

# SUSPECT-GEN3: Suspects up to three generations

**Goal.** `tp.generate` builds up to 3 generations from an editable rule list (a user rule file), prints the rules used and the counts per generation, with a cap printed as a `--set` parameter.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 18 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md`.

**Read first.** `tp.generate` code and descriptor, `docs/agenti/mzfinder.md`, `docs/studies/TRANSFORMATION_RECORD.md`.

**Already on main (checked 26 Nov, re-check before coding).** 1-2 steps of photocatalytic rules and user lists exist.

**Done means.** 1. Generations and rule file, default bytes unchanged. 2. wasm + native + CLI byte test. 3. Agent docs.

**Out of scope.** BioTransformer.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
