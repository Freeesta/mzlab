Read docs/coda/BRIEF.md.

# PBAR-REMOVE-OLD: Default `qqq.pbar` on and delete the per-panel tool rows and their overflow code

**Goal.** Default `qqq.pbar` on and delete the per-panel tool rows and their overflow code.

**Why.** Only after the owner has used the bar for some time. Removes `fitHd` machinery for tools and the e2e that guard it. Study: `docs/studies/PANEL_TOOLBAR_STUDY.md` (§3 design, §4.1-4.3 code facts and risks, §4.4 slices).

**Read first.** Study §4.3 (risks), §4.4 (slices); `pannelli.js` `fitHd`, `hdRestore`, `fitHdNow`; tests `e2e_intestazioni*`, `e2e_pannelli_stabili`, `e2e_xic_stretto`, `e2e_intestazione_cromato`.

**Already on main (re-check before coding).** The `actions/1` registry, the right-click menu, Shift+F10, long press and the active panel (`E.active`, `.act`) exist; no shared panel bar exists. Check `origin/main` first: another PR may have moved part of this.

**Done means.** 1. Setting removed, bar always on. 2. Header holds only selectors, badges, title, expand and close; `⋯` stays only for selectors. 3. Obsolete tests deleted with the code they guard; none skipped; the 64 e2e files that name header buttons all use the bar helper. 4. Docs (`actions.md`, `data-tools.md`, `high-resolution.md`) updated, no stale text. 5. `python3 tools/verifica.py` full round once.

**Out of scope.** Any engine, CLI or `.wasm` change (interface only: parity site = app = CLI is unchanged, say so in the PR body). Not in this slice: A configurable bar (user picks buttons).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
