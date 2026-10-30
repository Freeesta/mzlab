# V02: lean «Full reset» and isolation after the reset

You are a chat of mzLab (repository Freeesta/mzlab). Read AGENTS.md, then `docs/studi/VELOCITA_RICARICA.md` §3, §6–8. Branch `claude/reset-snello` from `origin/main`, after V01 is merged.

**Decision for Federico first** (ask the Coordinator if the prompt you got does not record it): may «Full reset» keep the program's own files (`mzlab-app-*`, `mzlab-big-*` caches and the service worker registration)? They hold no user data. Today `tests_e2e/e2e_reset.py` requires caches and workers to go. If the answer is no, do only points 2 and 3.

**Problem.** `wipe("all")` in `mzlab/web/reset.js` deletes OPFS, IndexedDB, every cache, the worker registration and storage, with safety waits (3 s per blocked database, 4 s for the worker), then `location.replace("?riparti=1")`. The reload comes from the network and is not isolated: `isolateOnce` in `mzlab/web/browser.js` sets `mzlab.coiReload` but never reloads in 9 runs out of 10, so the next manual reload is the slow one (empty cache, ~420 revalidations, 2–9 s). Repro: `python3 tests_e2e/misura_ricarica.py --out r.json`, then the summary: the «reset» page has no `coi` mark.

**Do.**
1. (If yes) keep the app caches and the registration; still delete OPFS, IndexedDB, `mzlab-libs`, localStorage `mzlab.*` / `mzlab-labels` and sessionStorage. The old `qqq*` names are still deleted.
2. Run the deletions in parallel, each with its own short timeout; report what was blocked as now.
3. Find why `isolateOnce` does not reload after a reset (guard left in sessionStorage? `controllerchange` already fired? `QQ_RESETTING`?) and fix it so the reset page ends isolated without a second manual reload. Keep the loop guard and `?cap-off=isolamento`.

**Tests.** Update `e2e_reset.py` to the decision (user data gone; program caches kept or gone); add: after reset the page becomes `crossOriginIsolated` with no manual reload; a second reload is not slower than a warm one.

**Measure.** `misura_ricarica.py` before/after (live or mirror, unthrottled and `--profile slow`). Expected: reset ≈ 1 s, warm-after-reset ≈ 0.4–0.8 s, isolated.

**Done.** PR ready with the decision quoted, both tables, one line on parity (browser only); CI green; message to the Coordinator.
