Read docs/coda/BRIEF.md.

# LEGEND-CHIP-CLIP: Legend chip list stays inside the panel

**Goal.** A legend with more than 4 entries collapses into a chip (`N traces ▾`) whose list (`.leg .lgl`, absolute, `top:100%`) opens below it. When the legend sits near the bottom of the panel the list is clipped by the panel edge and its entries (and their colour keys) are covered.

**Why.** Seen while testing the colour key of the XIC legend (PR #3759), an existing behaviour that the key did not change.

**Read first.** `docs/agenti/INDICE.md` (`grep -n 'legend' docs/agenti/INDICE.md`), `mzlab/web/grafici.js` (`legend`, the `lgc` chip, about line 292), `mzlab/web/app.css` (`.leg .lgl`), `tests_e2e/e2e_trace_key_panels.py`.

**Done means.** 1. The list opens upward (or inside the panel) when there is no room below, never clipped, scrollable when it is taller than the panel. 2. Same behaviour in SIM, MRM, XIC and DAD panels, at 390 px too. 3. An e2e step that places the legend at the bottom and checks the list rectangle is inside the panel, run in the browser. 4. Screenshots at deviceScaleFactor 2 in the PR body.

**Out of scope.** The colour key itself (merged in the PR above) and any other panel layout.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
