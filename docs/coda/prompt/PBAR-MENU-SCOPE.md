Read docs/coda/BRIEF.md.

# PBAR-MENU-SCOPE: One right-click menu per panel kind built from the registry with `scope` (point or panel), groups, keys and the same commands as the bar

**Goal.** One right-click menu per panel kind built from the registry with `scope` (point or panel), groups, keys and the same commands as the bar.

**Why.** Study §3.7: the menu holds what is under the pointer (point scope) then a «Panel» tail repeating the bar's actions, so touch and keyboard users never need the bar. Study: `docs/studies/PANEL_TOOLBAR_STUDY.md` (§3 design, §4.1-4.3 code facts and risks, §4.4 slices).

**Read first.** `docs/agenti/actions.md` (menu, Shift+F10, long press 550 ms), `explore.js` `menu()`, `eventi.js` `ctxFor`, `azioni.js` `GROUPS`, study §3.7 table.

**Already on main (re-check before coding).** The `actions/1` registry, the right-click menu, Shift+F10, long press and the active panel (`E.active`, `.act`) exist; no shared panel bar exists. Check `origin/main` first: another PR may have moved part of this.

**Done means.** 1. Field `scope: "point"|"panel"` in the registry; menu order: point actions, separator, panel actions with their key (`AZIONI.keyText`). 2. No second list of commands: the menu and `#pbar` read the same entries. 3. Long press and Shift+F10 open the same menu (`e2e_menu_coerenti` extended per kind: spectrum, chromatogram, XIC, SIM/MRM, map). 4. A menu never has more than 12 rows at the top level; the rest in one submenu.

**Out of scope.** Any engine, CLI or `.wasm` change (interface only: parity site = app = CLI is unchanged, say so in the PR body). Not in this slice: The bar itself.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
