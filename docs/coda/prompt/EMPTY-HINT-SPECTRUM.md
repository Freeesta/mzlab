Read docs/coda/BRIEF.md.

# EMPTY-HINT-SPECTRUM: spectrum and map panels use the same empty state

**Goal.** A spectrum panel with no time range chosen («Click or drag on the chromatogram…») and a map panel with no scans show the empty-state prompt of `empty-hint.js` in place of the line written on the canvas.

**Why.** The chromatogram, XIC and MRM panels already show a centred prompt; the spectrum (`picchi.js` and `uv-spettro.js` `say`) and map (`grafici.js` `drawMap`) still write a muted line at the top left of the canvas.

**Read first.** `mzlab/web/empty-hint.js`, `mzlab/web/picchi.js` (`say`), `mzlab/web/uv-spettro.js`, `tests_e2e/e2e_empty_hint.py`. Keep `p._say` (the data table and tests read it) and the legend reset.

**Done means.** The spectrum prompt names what to choose and points, when it can, at the chromatogram panel that holds the choice (the live rectangle, redrawn on resize); text-only otherwise; e2e at 1400 and 390 px; `grafici.js` and `picchi.js` do not grow (`python3 tools/controlla.py`).

**Out of scope.** The chromatogram, XIC and MRM panels (done).

**Follow-ups.** Closing step of the brief.
