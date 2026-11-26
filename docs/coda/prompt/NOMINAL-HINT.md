Read docs/coda/BRIEF.md.

# NOMINAL-HINT: Name the nominal-resolution signature in `features.estimate` and the report

**Goal.** When the data of a unit-resolution series show the signature that the `nominal` profile addresses, `features.estimate` and the `choices[]` of `features --report` say so with the numbers, and point at `--set detect.preset=nominal`. Nothing is applied by itself.

**Why.** The profile is opt-in; a user who does not know it keeps losing low-mass products and keeps one ion split in two rows without any sign of why.

**Read first.** `docs/agenti/mzfeat-lr.md` (preset `nominal`), `crates/mzlab-cli/src/features_report.rs` (`choices_json`, the `profile` choice), `crates/mzlab-feat/src/params.rs` (estimates), `crates/mzlab-feat/tests/lr_nominal.rs`. Start with `grep -n 'estimate' docs/agenti/INDICE.md`.

**Already on main (checked 26 Nov, re-check before coding).** The `profile` choice names the profile in force and the hint to `{"preset":"nominal"}`; no estimate looks at the data for the signature.

**Done means.** 1. An estimate with its proof: the share of candidate rows between the nominal and the `lr` height, and of ions seen in both grids farther apart than the grid reach, on a synthetic series. 2. The hint in the report and in `flow defaults`/estimate output, a `--json` key and a schema entry. 3. Site, app and CLI print the same hint (parity test).

**Out of scope.** Applying the profile automatically; changing the HR or `lr` defaults.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
