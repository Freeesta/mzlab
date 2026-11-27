Read docs/coda/BRIEF.md.

# DASHED-HOVER-LABEL: say what the second dashed line is

**Goal.** A chromatogram shows two dashed vertical lines once a spectrum is linked: the cursor (`.cl`, the time the spectrum is read at) and the hover guide (`.vl`, follows the pointer). Make the hover guide self-explaining: a small time label on the guide while it is shown, and an accessible description, without making the line catch pointer events.

**Why.** The owner could not tell the lines apart (screenshot of 27 Nov). PR #3776 only explains it in the tooltip of the cursor dot and in the badges; the hover line itself still says nothing.

**Read first.** `grep -n '\.vl' mzlab/web/grafici.js` (`putHover`), `mzlab/web/scroll.js` (`cursorLine`), `docs/agenti/dad.md` (reading the UV spectrum back to its source), `tests_e2e/e2e_linked_spectrum.py`.

**Done means.** 1. The label with its `lang/` key, readable at 390 and 1400 px. 2. An e2e that hovers and reads it. 3. Screenshots at deviceScaleFactor 2 on a side branch.

**Out of scope.** The cursor line and the badges (done in #3776).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
