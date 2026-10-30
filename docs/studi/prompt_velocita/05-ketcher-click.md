# V05: Ketcher start-up off the click path

You are a chat of mzLab (repository Freeesta/mzlab). Read AGENTS.md, then `docs/studi/VELOCITA_RICARICA.md` §5. Branch `claude/ketcher-click` from `origin/main`, after landing D3 #2207 / D1b #2211 (they touch `first-paint.js`).

**Problem.** The «noticeable latency when clicking around» is not in the click handlers (24–96 ms): after opening Disegno there are main-thread long tasks of 2.1–2.5 s at CPU ×4, most likely the Ketcher start-up (`first-paint.js` prefetches Ketcher, 29 MB, 3 s after load; `draw.js` starts it). Opening the first file costs another 0.9–1.4 s of long tasks.

**Do.**
1. Measure with attribution first: `python3 tests_e2e/misura_ricarica.py --profile cpu4 --out k.json` plus a Performance trace (CDP `Tracing`) around the Disegno click; name the function that takes the time. Write the finding in the PR.
2. If it is the Ketcher iframe start-up: start it at idle after the first visit to Disegno (or at idle when the user lingers on Disegno), never inside a click; the button answers at once with the loading state already used by Disegno. Do not drop the 3 s prefetch unless the trace shows it is the cost.
3. If it is the first file opening: write the trace in the PR and stop (it belongs to the engine packages, not here).

**Tests.** `e2e_strumenti_ketcher.py`, `e2e_ketcher_grandi.py`, `e2e_lingua_disegno.py`, `e2e_tour.py` green; add: the Disegno click's longest event under 200 ms at CPU ×4 (PerformanceObserver `event`).

**Done.** PR ready with the trace finding and before/after long tasks; parity line (the desktop runs the same page); CI green; message to the Coordinator.
