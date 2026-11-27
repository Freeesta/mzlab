Read docs/coda/BRIEF.md.

# RAW-VERSIONS: More Thermo .raw file versions

**Goal.** Read the Thermo `.raw` file versions of the owner's instrument, beyond 57, 63, 64, 66.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 21 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md`.

**Read first.** `crates/mzlab-core/src/rawreader.rs`, `vendor/opentfraw/src/robust.rs` (`VERIFIED_VERSIONS`), `docs/agenti/readers-module.md`.

**Already on main (checked 26 Nov, re-check before coding).** Other versions are refused with `err.file.rawVersion`.

**Done means.** 1. The owner gives the version numbers (format facts only, never data). 2. A version is added only with a check on a real file in the private data repo and a synthetic test here.

**Out of scope.** Other vendors.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
