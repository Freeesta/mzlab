Read docs/coda/BRIEF.md.

# SPECTRUM-COMPARE-OFFSET: an m/z offset per side in the compare card

**Goal.** The card takes, for each side, an m/z offset in Da or `auto:MZ[@RT]`, as `spectrum compare --a-mz-offset` and `--b-mz-offset`, and shows the offset each side used (`mz_offset_da`, `mz_offset`).

**Why.** A sample and a standard measured with different calibrations land in different nominal bins; the CLI has the flags, the card does not, so the site half of the feature is narrower than the CLI.

**Read first.** `mzlab/web/spettro-confronto.js` (the card), `docs/agenti/spectral-match.md` (section 6), `crates/mzlab-cli/src/spectrum_compare.rs`. Start with `grep -n 'spectrum' docs/agenti/INDICE.md`.

**Already on main (check before coding).** The card of the MS2 spectrum menu ("Compare with another MS2 spectrum…") compares two selections (file, precursor, time window) through the `librerie` export on the site and the `speccompare` op in the app; the e2e `tests_e2e/e2e_spettro_confronto.py` covers it.

**Done means.** 1. Two fields in the card, sent as `mzoff=` of each side's `/api/spectrum` query (check the route in `crates/mzlab-core/src/nominalbin.rs`). 2. The document's offset evidence shown per side, `auto` included. 3. All text through `lang/` keys. 4. An e2e step on a synthetic file shifted by a known offset, and the existing `mz_offset` bytes unchanged.

**Out of scope.** Library search; MS2 of different precursors; changing the `spectrum-compare/1` document.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
