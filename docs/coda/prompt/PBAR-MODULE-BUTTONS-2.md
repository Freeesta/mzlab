Read docs/coda/BRIEF.md.

# PBAR-MODULE-BUTTONS-2: The DDA buttons and `ticbg` become registry actions with a `run`

**Goal.** Every header button that `dda-elenco.js` and `tic-fondo-ui.js` append by hand is an action drawn by `AZIONI.bar` with a working `run`, as `xicp`, `tabp` and the XWC pair are after PBAR-MODULE-BUTTONS.

**Why.** `ddaw` and `ms2tri` (`dda-elenco.js`) and `ticbg` (`tic-fondo-ui.js`) are created with `createElement` and inserted next to the `xicb` and `dl` groups. A bar that lists `AZIONI.list(c, "barra")` (later slices of `docs/studies/PANEL_TOOLBAR_STUDY.md`) would miss them.

**Read first.** `docs/agenti/actions.md` (Header buttons), `azioni-barra.js` (rows with `visible`, `hid`, `bid`), `dda-elenco.js`, `tic-fondo-ui.js`, the e2e that name `ddaw`, `ms2tri`, `ticbg`.

**Already on main (re-check before coding).** `xicb`, `xicl`, `xicp`, `tabp`, `xwc`, `xwcl` are registry rows; check `origin/main` first.

**Done means.** 1. The three buttons are rows (`barra: true`, `scope: "panel"`, `bar` group) with a `run`; no node is appended by hand to `.tbs`. 2. DOM, `data-a`, `aria-*`, classes and order unchanged; the e2e that name them pass untouched. 3. `tests/test_azioni.py` lists the new ids per kind. 4. The start-up scripts cap (`tests/test_peso_avvio.py`) is not raised.

**Out of scope.** Any engine, CLI or `.wasm` change (interface only; parity unchanged). The shared bar `#pbar` itself.

**Follow-ups.** Closing step of the brief: delete your row and this file.
