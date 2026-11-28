Read docs/coda/BRIEF.md.

# PBAR-TWINS-REMOVE: delete the hidden tool-button twins of the panel header

**Goal.** The panel bar (`#pbar`) holds the only tool buttons; the hidden copies in every panel header (`.tbs .tbg`) are gone, with the mirror code of `pbar.js` (`mirror`, the `MutationObserver`, the click on the header twin).

**Why.** PBAR-REMOVE-OLD made the bar permanent and removed the visible tool rows and their overflow code, but kept the buttons in the DOM hidden by CSS, because about 30 modules and many e2e read them by `p.el.querySelector('[data-a=...]')` (`xic-spettro.js`, `zoom-reset.js`, `spettro.js`, `scroll.js`, `tabella-picchi.js`, `xwc.js`, `dda-elenco.js`, `tic-fondo-ui.js`, `picchi.js`, `history.js`, `pannelli.js`, ...).

**Read first.** `docs/studies/PANEL_TOOLBAR_STUDY.md` §4.1-4.3; `docs/agenti/actions.md` (Panel tool buttons, Panel bar); `pbar.js`, `azioni-barra.js` (`AZIONI.bar`, `AZIONI.refresh`).

**Done means.** 1. `AZIONI.refresh` writes the state to the bar buttons of the selected panel; panel state lives in `p`, not in buttons. 2. Popovers anchor to the bar button (or the header) and need no hidden twin. 3. No `.tbs .tbg` in the header DOM; `hdkeep`, `twins()` and the mirror are deleted. 4. e2e that read a header twin use the `bar` helper. 5. `python3 tools/verifica.py` full round once.

**Out of scope.** Any engine, CLI or `.wasm` change (interface only: parity site = app = CLI is unchanged, say so in the PR body). A configurable bar.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
