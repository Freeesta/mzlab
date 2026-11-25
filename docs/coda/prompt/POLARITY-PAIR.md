Read docs/coda/BRIEF.md.

# POLARITY-PAIR: Pair [M+H]+ and [M-H]- features across the polarity methods

**Goal.** Pair the [M+H]+ and [M-H]- features of the two polarity methods with an RT tolerance, to see which species behave as acids; log the pairing rule and tolerance.

**Why.** The two polarities are separate tables; the neutral mass and the match are done by hand.

**Read first.** `docs/agenti/ion-groups.md`, `docs/agenti/mzfeat.md`, `crates/mzlab-core/src/adduct_table.rs`, the `--polarity` flags of `features` and `quant union`. Start with `grep -n 'union\|adduct' docs/agenti/INDICE.md`, then `grep -rn 'adduct' crates/mzlab-core/src/adduct_table.rs`.

**Already on main (checked 25 Nov, re-check before coding).** Adduct tables exist. `features.union` has a `polarity` parameter. There is no cross-polarity pairing step on main (check `grep -rn 'polarity' crates/mzlab-feat/src/steps --include=*.rs -l`).

**Done means.** 1. A step or command `pair` (name at most 10 characters) taking two feature tables: neutral mass from each adduct (list in the table, user-editable), RT tolerance and mass tolerance as `--set`; every pair with the mass error, RT difference and the rule; unmatched rows listed. 2. Synthetic tables with known pairs and a decoy. Parity site=app=CLI.

**Out of scope.** Identification of the compounds.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
