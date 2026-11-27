Read docs/coda/BRIEF.md.

# PBAR-MODULE-BUTTONS: The header buttons that other modules add or wire become registry actions with a `run`

**Goal.** Every header button is an action with a working `run`, including the ones the modules wire or add after `AZIONI.bar` has drawn the header.

**Why.** `azioni-barra.js` (`AZIONI.bar`, `AZIONI.refresh`) draws and wires the common buttons. Still outside the registry: `xicb` and `xicl` (drawn by the renderer, wired by `XICBTN.wire` in `xic-spettro.js`, so their actions have no `run`), the buttons appended later (`xicp` in `xic-spettro.js` `wire`, `tabp` in `tabella-picchi.js`, the XWC split button of `xwc.js` which wraps `onclick` of `xic`, the DDA trio and `tic-fondo-ui.js` groups). A bar that lists `AZIONI.list(c, "barra")` (later slices of `docs/studies/PANEL_TOOLBAR_STUDY.md`) would miss them or find actions that do nothing.

**Read first.** `docs/agenti/actions.md` (Header buttons), `azioni-barra.js`, `xic-spettro.js` (`wire`, `press`, `openPop`), `tabella-picchi.js`, `xwc.js` (`dress`), `dda-elenco.js`.

**Already on main (re-check before coding).** `AZIONI.bar` draws `panel.xicb` and `panel.xicl` without a `run`; `XICBTN.wire(p)` sets `onclick` on them. Check `origin/main` first.

**Done means.** 1. `panel.xicb` and `panel.xicl` have `run` (`press`, `openPop`) and the renderer wires them; `XICBTN.wire` keeps only what is not a click (Space key, refresh). 2. `xicp`, `tabp` and the XWC state are registry actions (`barra: true`, `scope: "panel"`, `bar` group), drawn by `AZIONI.bar`, with no node appended by hand to `.tbs`. 3. DOM, `data-a`, `aria-*`, classes and order unchanged: the e2e that name these buttons (`e2e_xic_spettro*`, `e2e_tabella_picchi`, `e2e_xwc*`) pass untouched. 4. `tests/test_azioni.py` lists the new ids per kind. 5. The start-up scripts cap (`tests/test_peso_avvio.py`) is not raised: it has almost no room, so shrink elsewhere or load lazily.

**Out of scope.** Any engine, CLI or `.wasm` change (interface only; site = app = CLI parity is unchanged: say so in the PR body). The shared bar `#pbar` itself (`PBAR-SKELETON`).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
