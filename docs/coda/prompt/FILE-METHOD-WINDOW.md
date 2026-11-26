Read docs/coda/BRIEF.md.

# FILE-METHOD-WINDOW: Show the file method in the site

**Goal.** The lazy module `filemethod` (`rust-filemethod.js`, worker op `filemethod`) answers `file_method/1` in the browser, but no window shows it. Show per segment precursor, CE, CES, DP, fill time, Q1 resolution, the `unavailable` reasons and the name/precursor warning, in the existing Method window; the same text as `mzlab-cli file method`.

**Read first.** `grep -n 'method' docs/agenti/INDICE.md`, `docs/agenti/method.md`, `mzlab/web/rust-filemethod.js`, the Method window code (`grep -n 'method' mzlab/web/*.js` narrowly; never read `vendor/`), `docs/schemi/file_method-1.json`, `mzlab/web/lang/en/`.

**Done means.** The window lists the document without a new bundle or dependency; every text is a `lang/` key (`python3 tools/controlla_i18n.py`); null values show their reason; a warning shows its numbers and never blocks; an e2e test on a synthetic file (`python3 tools/verifica.py --changed`); the warning key `warn.file.nameMz` has its text.

**Out of scope.** New wasm code, new CLI routes, changing `file_method/1`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
