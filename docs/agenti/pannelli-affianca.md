# Tiling and moving panels (`affianca.js`, `AFFIANCA`)

- `.aff-grip` handle in the panel header; dragging it shows a preview of the final rectangle (`.aff-prev`, z-index above the panels, which rises on every activation) without redrawing; Esc cancels. Without dragging: the handle menu and Alt+arrows. `rifai()` applies a move to a stack (real or copied). Closing a panel (`closePanel`) leaves the last companion in the row at full width (`quota` null, `x` 0); a panel resized on its own does not change.
- The same handle moves are in the right-click menu and in the palette (`disp.*` actions, `docs/agenti/actions.md`); `disp.ripristina` puts every panel back at full width (undone with Ctrl+Z); `layout.js` never names an experiment (guard in `controlla.py`).
- The spectrum opened with a double click (`newSpec`, `eventi.js`) always goes below the chromatogram, in its own row; to tile it use the handle or the `disp.*` actions. An old `beside` in saved preferences is ignored.
- Cross-shaped drop zones on the target panel (`zona()`): top 30% = "Above" (own row before the target), bottom 30% = "Below", central band left half / right half = "Left" / "Right"; "Not enough space" if it does not fit. Each zone has its own label in the preview.
- A chromatogram drags its child spectra with it; dropped on its own spectrum (or with Alt+arrow towards it) it moves by itself past it (`solo` in the move).
- `.aff-sep` dividers (`role=separator`, arrows +-16, Shift +-48, Enter = equal) rebuilt by `sync()` in `fitHost`. Undone with `HSEQ` (Ctrl/Cmd+Z or the Undo button of the bar); no visible notice, only the announcement in `#aff-live`.
- Tests: `e2e_affianca`, `e2e_affianca_zone`.
