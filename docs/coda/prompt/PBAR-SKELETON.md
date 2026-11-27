Read docs/coda/BRIEF.md.

# PBAR-SKELETON: A shared panel bar `#pbar` and the selection model, behind an off-by-default setting `qqq.pbar`

**Goal.** A shared panel bar `#pbar` and the selection model, behind an off-by-default setting `qqq.pbar`.

**Why.** Study §3: one bar above the panel stack shows the tools of the selected panel. `E.active` and the `.act` left bar already exist (`setActive`, `stato.js`); nothing reads them to build a bar yet. Study: `docs/studies/PANEL_TOOLBAR_STUDY.md` (§3 design, §4.1-4.3 code facts and risks, §4.4 slices).

**Read first.** Study §3.2-3.4, `pannelli.js` `setActive`, `explore.js` (click outside clears the selection), `barra.js` (`SHEET`, `PHONE_W`), `azioni-barra.js` (`AZIONI.bar`, `AZIONI.refresh`: the header buttons are registry actions).

**Already on main (re-check before coding).** The `actions/1` registry, the right-click menu, Shift+F10, long press and the active panel (`E.active`, `.act`) exist; no shared panel bar exists. Check `origin/main` first: another PR may have moved part of this.

**Done means.** 1. `#pbar` (`role="toolbar"`, roving tabindex, `aria-label` names the panel) sticky under `#stbar`, one row, 40 px (44 on touch). 2. Left chip: panel number, kind and file; follows `E.active`; the first panel is selected when none is and a click on empty space keeps the last one (study §3.3). 3. With the setting on it shows the same buttons as the header through `AZIONI.bar`, header groups hidden by CSS; off = today. 4. Test: select panels 1-3 by click and key `1`-`9`, the bar follows and `data-a` buttons work. 5. New texts only via `lang/en`.

**Out of scope.** Any engine, CLI or `.wasm` change (interface only: parity site = app = CLI is unchanged, say so in the PR body). Not in this slice: Several selected panels (study §3.3, an optional later slice); the phone layout (own slice).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
