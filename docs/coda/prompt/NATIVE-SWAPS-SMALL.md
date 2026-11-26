Read docs/coda/BRIEF.md.

# NATIVE-SWAPS-SMALL: Four native swaps and one shared wait in the site JS

**Goal.** Replace hand-written code with a platform feature at or below the site floor (Safari 16.4, Chrome 111, Firefox 121): about 10 lines go.

**Read first.** `docs/agenti/web-modern.md` (NATIVE-SWAPS-SMALL), `docs/agenti/compat-safari.md` (the floor and the allowed list). Sites: `explore.js:283` `JSON.parse(JSON.stringify(...))` to `structuredClone`; `archivio.js:34` abort timer to `AbortSignal.timeout(WAIT_MS)` with the controller kept as fallback where `AbortSignal.timeout` is missing (Chrome 111-123); `file-sicuri.js:90` and `workflow.js:227` `Object.prototype.hasOwnProperty.call` to `Object.hasOwn`; the `wait` defined twice in `disegno-appunti.js` and inline in `reset.js` to one definition per file.

**Done means.** (1) Each swap behaves the same on plain data: for `structuredClone` show in the PR body that the cloned value holds no function, `undefined` key or `Date` that JSON handled differently. (2) For `AbortSignal.timeout` the error name seen by callers (`TimeoutError` instead of `AbortError`) is checked at every `catch` of `archivio.js` and the tests pass. (3) Tests green: `tests/test_archivio.py`, `tests/test_file_sicuri.py`, `e2e_reset`, `e2e20`, `e2e23`; `tests/test_safari_api.py` passes (add a line to `tools/safari_api_ammesse.txt` only if it asks). (4) PR body: `git diff --shortstat`.

**Out of scope.** `crypto.randomUUID` (secure contexts only; the desktop scheme is unproven), `Intl.Collator` for `naturalCmp`, the desktop base URL constant, anything in `compat.js`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
