Read docs/coda/BRIEF.md.

# SPECTRUM-COMPARE-MIRROR: a mirror plot in the compare card

**Goal.** The card draws the two spectra as a mirror (A up, B down) with the matched bins marked, from the `matched`, `only_a` and `only_b` lists of the document.

**Why.** The card shows tables only; a mirror shows at once which ions agree and where the intensities differ. The mirror of the library window (`LIB.mirror`) draws a query against a library entry and can be reused or generalised.

**Read first.** `mzlab/web/spettro-confronto.js` (the card), `docs/agenti/spectral-match.md` (section 6), `crates/mzlab-cli/src/spectrum_compare.rs`. Start with `grep -n 'spectrum' docs/agenti/INDICE.md`.

**Already on main (check before coding).** The card of the MS2 spectrum menu ("Compare with another MS2 spectrum…") compares two selections (file, precursor, time window) through the `librerie` export on the site and the `speccompare` op in the app; the e2e `tests_e2e/e2e_spettro_confronto.py` covers it.

**Done means.** 1. A canvas in the card, colours from CSS variables, Okabe-Ito safe. 2. Its data table through `TDATI` (rule of `.claude/rules/frontend.md`). 3. All text through `lang/` keys. 4. An e2e step that checks the canvas holds the matched and unmatched bars of the known-cosine files.

**Out of scope.** Library search; MS2 of different precursors; changing the `spectrum-compare/1` document.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
