Read docs/coda/BRIEF.md.

# PBAR-MOVE-SPECIFIC: The kind-specific buttons (integration set, XIC, peak picking, axis lock, % and the SIM area) move to `#pbar`, grouped per kind

**Goal.** The kind-specific buttons (integration set, XIC, peak picking, axis lock, % and the SIM area) move to `#pbar`, grouped per kind.

**Why.** Second move: what only some kinds have. The bar shows a group only when the selected panel has it. Study: `docs/studies/PANEL_TOOLBAR_STUDY.md` (§3 design, §4.1-4.3 code facts and risks, §4.4 slices).

**Read first.** Study §3.2 mockups and §4.2 counts; `picchi.js`, `xic-spettro.js`, `sim-ui.js` (`panel.area`), `xwc.js`, `dad-mrm.js`, `mappa.js` (`CMDS`).

**Already on main (re-check before coding).** The `actions/1` registry, the right-click menu, Shift+F10, long press and the active panel (`E.active`, `.act`) exist; no shared panel bar exists. Check `origin/main` first: another PR may have moved part of this.

**Done means.** 1. Groups: View, Analyse (integration), Extract (XIC, spectrum at RT), Output; labels from `lang/en`. 2. A kind that lacks an action shows nothing, never a greyed gap; an unavailable action is hidden as in the menu (`AZIONI.clean` rule). 3. The map keeps its own `.mznav` bar logic through the same renderer. 4. SIM/MRM channel picker, colour keys on traces and the empty-panel hint stay in the header or on the plot (study §3.6). 5. e2e helper from the previous slice.

**Out of scope.** Any engine, CLI or `.wasm` change (interface only: parity site = app = CLI is unchanged, say so in the PR body). Not in this slice: Removal of the old rows; phone and keyboard polish.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
