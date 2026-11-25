Read docs/coda/BRIEF.md.

# RT-ALIGN-REF: RT alignment on an internal reference between batches

**Goal.** Align retention times between batches on an internal reference compound (shifts of 0.16-0.41 min were seen), with the shift per file in the evidence.

**Why.** Without it the same compound sits in different RT windows in different batches, so targeted extraction and the union of tables miss or split it.

**Read first.** `crates/mzlab-feat/src/rtwarp.rs`, `rtmap.rs`, `crates/mzlab-cli/src/flow_drift.rs` (drift step), `docs/agenti/mzfeat.md`, `docs/agenti/quant-targets.md` (union, `rt_warp`).

**Already on main (checked 25 Nov, re-check before coding).** `features.union@1` has `rt_warp` off or piecewise; a drift step exists. Check whether an internal-reference anchor (one named ion list, one RT window) already feeds them; if so, the row is the reference-driven anchor and its logging only.

**Done means.** 1. `--set` parameters for the reference m/z, window and tolerance; the shift per file is logged and applied only where the user asked. 2. A synthetic two-batch test with a known shift. 3. Parity site=app=CLI. Do not duplicate `rt_warp`: extend it.

**Out of scope.** Mass offset (row MZ-OFFSET-POL); PDA delay (row DAD-DELAY-APPLY).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
