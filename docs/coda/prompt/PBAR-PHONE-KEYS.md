Read docs/coda/BRIEF.md.

# PBAR-PHONE-KEYS: The bar at 390 px and its keyboard use

**Goal.** The bar at 390 px and its keyboard use.

**Why.** Study §3.5-3.6: on a phone the bar is one scrollable row with a More sheet; with the keyboard it is a toolbar reached by one key. Study: `docs/studies/PANEL_TOOLBAR_STUDY.md` (§3 design, §4.1-4.3 code facts and risks, §4.4 slices).

**Read first.** Study §3.5-3.6; `barra.js` (`SHEET`, `PHONE_W`), `accessibility.md`, `azioni-tasti.js` (`KEYS`, no duplicate key), `e2e_scorciatoie`.

**Already on main (re-check before coding).** The `actions/1` registry, the right-click menu, Shift+F10, long press and the active panel (`E.active`, `.act`) exist; no shared panel bar exists. Check `origin/main` first: another PR may have moved part of this.

**Done means.** 1. At 390 px: targets 44 px, priority order per kind, the rest in a bottom sheet; no horizontal page scroll (axe + `e2e_intestazioni` at 390). 2. Keyboard: a registered key moves focus to `#pbar`, arrows move, Enter/Space act, Esc returns to the chart (`ESC.registra`). 3. Shortcuts panel lists the new key. 4. Screen reader: the panel name is announced when the selection changes (`#sb-live`).

**Out of scope.** Any engine, CLI or `.wasm` change (interface only: parity site = app = CLI is unchanged, say so in the PR body). Not in this slice: Multi-select.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
