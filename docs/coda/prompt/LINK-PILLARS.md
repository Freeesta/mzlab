Read docs/coda/BRIEF.md.

# LINK-PILLARS: The three pillars of the parent-TP link

**Goal.** A step `tp.pillars@1` gives each row three pillars side by side: MS2 vs the parent (cosine, shared ions, neutral losses), RT shift direction vs the parent, kinetic coherence (rises while the parent decays, tmax vs parent t1/2). Each pillar is `supports`, `against` or `no_data` with its numbers; nothing is combined into a hidden score.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 10 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §D (bottom line).

**Read first.** `crates/mzlab-lib/src/speccmp.rs` (`spectrum compare`), `crates/mzlab-feat/src/kinetic/evidence.rs` (coupling), `tp.link`, Attributio RT-logP check (`docs/agenti/attributio-revisione.md`).

**Already on main (checked 26 Nov, re-check before coding).** Each pillar exists in pieces; neutral losses between parent and TP spectra are not computed.

**Done means.** 1. The step in Rust, LR and HR, thresholds as `--set` with defaults printed. 2. wasm + native + CLI byte test. 3. A pillar line in the feature view.

**Out of scope.** A combined score.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
