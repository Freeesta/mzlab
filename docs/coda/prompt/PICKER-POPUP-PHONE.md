Read docs/coda/BRIEF.md.

# PICKER-POPUP-PHONE: Channel picker popup fits a 390 px screen

**Goal.** At 390 px width the channel picker popup (`.can-pop`, opened from the `Ions (n/m)` select of a SIM/MRM panel) is wider than the screen: its right part (the `None` button, the right of the rows) is off screen. The colour-key checkbox and the 24 px targets are fine; the width is the problem.

**Why.** On the first phone-width session the popup overflowed; found while reviewing the checkbox colour key (PR #3759). Not caused by it.

**Read first.** `docs/agenti/INDICE.md` (`grep -n 'picker\|canali' docs/agenti/INDICE.md`), `mzlab/web/canali.js` (`.can-pop` markup, `open`), `mzlab/web/canali.css` (`.can-pop`, `.can-sim`: `min-width:280px;max-width:min(360px,92vw)`), `tests_e2e/e2e_trace_key.py` (the 390 px step).

**Done means.** 1. At 390 px the popup lies inside the viewport (left and right edges within it), scrolls vertically when taller, and all its buttons and rows stay reachable by touch and keyboard. 2. Desktop placement unchanged. 3. A 390 px e2e step that measures the popup rectangle against the viewport, run in the browser. 4. Screenshots at deviceScaleFactor 2, desktop and 390 px, in the PR body.

**Out of scope.** The colour key itself (merged in the PR above) and any other panel layout.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
