Read docs/coda/BRIEF.md.

# UV-PANEL-LEG-NARROW: UV spectrum legends clear of the axis at 390 px

**Goal.** In the UV spectrum panel (`uv-spettro.js`, CSS `.pnl.spec .leg`, `.leg2`) the one-line legend at the bottom ("file · RT x min") must not overlap the "Wavelength (nm)" axis title at 390 px, and the boxed source legend (`provenance`, `.leg2`) must not sit over the curve.

**Why.** Found in the screenshots of the linked-spectrum PR (#3776, branch `claude/linked-spectrum-shots`, `shots/linked-spectrum/`): at 390 px both overlaps are visible. The bottom-legend overlap is already on main.

**Read first.** `grep -n 'leg' docs/agenti/dad.md`, `docs/agenti/INDICE.md` (panels, narrow screens), `tests_e2e/e2e_linked_spectrum.py` (fixture and 390 px pattern: load wide, then `set_viewport_size`).

**Done means.** 1. At 390 and 1400 px no legend box overlaps the axis labels or the plotted curve (a bounding-box check in an e2e). 2. Screenshots at deviceScaleFactor 2 on a side branch. 3. Text through `lang/` keys only.

**Out of scope.** Other panel types; the wording of the legends.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
