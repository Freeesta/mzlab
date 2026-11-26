Read docs/coda/BRIEF.md.

# CLIP-SAVE-UNIFY: One copy path and one Blob download path in the site

**Goal.** Several files re-implement what `clipboardCopy` (`mzlab/web/compat.js`) and `saveBlob` (`mzlab/web/ordine.js`) already do. Remove the copies; about 20 lines go, and the copies without a fallback gain one.

**Read first.** `docs/agenti/web-modern.md` (CLIP-SAVE-UNIFY), `docs/agenti/compat-safari.md` (clipboard row: the gesture is consumed after an `await`, so the `ClipboardItem` is created inside the click). Sites: `elementare.js` `copyText`, `perf.js` `#perf-copy`, `tdati.js` copy button, `area-copia.js` `legacy` and `put` (keep its feedback message), then the other files that call `navigator.clipboard.writeText` directly (`grep -n 'clipboard.writeText' mzlab/web/*.js`); `bug-report.js` (`-dl` button), `liste-target.js`, `rapporto-flow.js` `download` against `saveBlob`.

**Done means.** (1) Every copy goes through `clipboardCopy` and keeps its message and focus behaviour; every Blob download goes through `saveBlob` (check the script load order in `carica.js` and the lazy modules: a module that runs before `ordine.js` needs an import or a lazy lookup, not a copy). (2) Download file names, MIME types and bytes unchanged; revoke delay not shorter than today. (3) Tests green: `e2e_area_copia`, `e2e_perf`, `e2e_tabelle_dati`, `e2e_banco_barra`, `e2e_hr_composizione`, `e2e_bug_report`, `e2e_liste_target`, `e2e_rapporto_flow`, `tests/test_compat.py`, `tests/test_bug_report.py`; run `python3 tools/verifica.py --changed`. (4) `python3 tools/safari_api.py --list` unchanged or only fewer uses. (5) PR body: lines before and after (`git diff --shortstat`).

**Out of scope.** New features, a new clipboard API, changed texts, `.wasm` modules and their caps (this task touches none). No Python and no logic moves: JS stays interface only.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
