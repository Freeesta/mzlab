Read docs/coda/BRIEF.md.

# PBAR-MOVE-COMMON: The buttons common to all kinds (fit, download, ruler, log, smoothing, zoom link, expand) work from `#pbar` and leave the header when the setting is on

**Goal.** The buttons common to all kinds (fit, download, ruler, log, smoothing, zoom link, expand) work from `#pbar` and leave the header when the setting is on.

**Why.** First real move of the buttons; the panel-specific ones stay in the header until the next slice. Study: `docs/studies/PANEL_TOOLBAR_STUDY.md` (§3 design, §4 code facts, §5 slices).

**Read first.** Study §4 table (shared vs specific), `pannelli.js` `fitHd` and `hdRestore`, `zoom-reset.js`, `e2e_intestazioni*.py`.

**Already on main (re-check before coding).** The `actions/1` registry, the right-click menu, Shift+F10, long press and the active panel (`E.active`, `.act`) exist; no shared panel bar exists. Check `origin/main` first: another PR may have moved part of this.

**Done means.** 1. With `qqq.pbar` on: fit, download, expand, ruler, log, smoothing, zoom link are only in the bar; close and expand also stay inline (owner idea: always inline). 2. Per-panel state (`p.smooth`, `p.log`, `p.tl`) shows in the bar for the selected panel and as a small dot chip in the header of any panel with a non-default state. 3. `fitHd` no longer moves these into `⋯`. 4. The e2e files that click these buttons use the helper `bar(pg, panel, id)` of `tests_e2e/lib.py` (select the panel, click in `#pbar`); with the setting off they still pass.

**Out of scope.** Any engine, CLI or `.wasm` change (interface only: parity site = app = CLI is unchanged, say so in the PR body). Not in this slice: Specific buttons; removal of the old rows.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
