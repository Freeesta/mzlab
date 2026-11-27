Read docs/coda/BRIEF.md.

# EMPTY-HINT-FILES: the empty-state arrow also points at the file list

**Goal.** A chromatogram, XIC or MRM panel that shows nothing because no file is ticked points its arrow at the file list in the side bar, and an XIC panel with no trace points at the control that adds one.

**Why.** `mzlab/web/empty-hint.js` draws the prompt and the arrow only to a control inside the panel (the channel picker, the `fpop` file counter of an XIC panel). The file ticks live in the side bar, so those panels show a centred line without an arrow, and the XIC line «Type an m/z above» has no m/z field to point at.

**Read first.** `mzlab/web/empty-hint.js`, `tests_e2e/e2e_empty_hint.py`, `docs/agenti/data-channels.md` (the channel picker).

**Done means.** `empty()` accepts a target outside the panel (the live rectangle of the file list or the XIC input, still redrawn on resize and layout changes, plain «above»-style line when it is not on screen); e2e at 1400 and 390 px like the channel case; no change to the canvas drawing.

**Out of scope.** Spectrum and map panels (EMPTY-HINT-SPECTRUM).

**Follow-ups.** Closing step of the brief.
