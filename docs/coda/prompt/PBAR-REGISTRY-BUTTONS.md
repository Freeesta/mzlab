Read docs/coda/BRIEF.md.

# PBAR-REGISTRY-BUTTONS: Every panel header button becomes an action of the `actions/1` registry, drawn by one renderer, the header looking and testing exactly as today

**Goal.** Every panel header button becomes an action of the `actions/1` registry, drawn by one renderer, the header looking and testing exactly as today.

**Why.** The header buttons are hand-written HTML in `addPanel` (`mzlab/web/pannelli.js`, near line 301) and the right-click menu comes from `AZIONI` (`azioni.js`): two lists kept by hand. One list is the base of every later slice, whatever the owner picks. Study: `docs/studies/PANEL_TOOLBAR_STUDY.md` (§3 design, §4.1-4.3 code facts and risks, §4.4 slices).

**Read first.** `docs/agenti/actions.md`, `docs/agenti/data-tools.md` (Panel header), `pannelli.js` `addPanel`, `ctl`, `syncTb`, `fitHd`; `azioni.js` (`barra`, `list(c, "barra")`).

**Already on main (re-check before coding).** The `actions/1` registry, the right-click menu, Shift+F10, long press and the active panel (`E.active`, `.act`) exist; no shared panel bar exists. Check `origin/main` first: another PR may have moved part of this.

**Done means.** 1. The 25 header `data-a` ids (study §4.2) are registry actions with `barra: true`, `scope: "panel"` and `checked`/`available` read from the panel state `p`. 2. ONE renderer `AZIONI.bar(c, host)` writes the buttons; `addPanel` uses it; DOM, `data-a`, `aria-*`, classes and order are unchanged, so no e2e test changes. 3. `syncTb` is replaced by the renderer's refresh. 4. `tests/test_azioni.py` and `e2e_menu_coerenti` cover every new id (icon in `POR_ID`).

**Out of scope.** Any engine, CLI or `.wasm` change (interface only: parity site = app = CLI is unchanged, say so in the PR body). Not in this slice: Whether the object `{p}` or a new context type carries the panel state; whether `fitHd` can read the registry order.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
