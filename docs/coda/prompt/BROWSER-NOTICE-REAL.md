Read docs/coda/BRIEF.md.

# BROWSER-NOTICE-REAL: Try the browser notice in real Safari and Firefox

**Goal.** Check the closable browser notice (`avviso-browser.js`) in real Safari (macOS) and real Firefox, where only Chromium with `?cap-off=` was tested.

**Why.** The cloud has Chromium only. The notice is decided by feature checks (`showDirectoryPicker`, SIMD128, OPFS); only a real browser proves that Safari and Firefox really lack the folder picker, show the notice once, remember the close, and fit 390 px.

**Read first.** `docs/agenti/capabilities.md` (browser notice), `docs/agenti/compat-safari.md`, `tests_e2e/e2e_avviso_browser.py`, `tests_e2e/e2e_safari_ipad.py` (how WebKit is used when present). `grep -n 'webkit' docs/agenti/test.md`.

**Already on main (checked 27 Nov, re-check before coding).** The notice and its Chromium e2e are merged; `notte.yml` runs `--smoke` with `--browser webkit` on macOS.

**Done means.** 1. The e2e runs in real WebKit (nightly macOS) without `?cap-off=` and expects the folder notice; a Firefox run does the same and expects Chrome and Edge only. 2. A short manual check by the owner on a Mac (Safari) and in Firefox is recorded in the doc, with the screenshots. 3. If a real browser disagrees with the feature check, fix the check, not the test.

**Out of scope.** New browser features; the app.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
