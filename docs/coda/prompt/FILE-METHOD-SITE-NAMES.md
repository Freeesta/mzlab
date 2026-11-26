Read docs/coda/BRIEF.md.

# FILE-METHOD-SITE-NAMES: Name/precursor check beside the scan list of the site

**Goal.** `mzlab-cli sample list` warns when a sample name carries an m/z far from every precursor; the site's scan list does not show it. Show the same warning, with its numbers, beside the scan list of a sample that has one.

**Why.** The Method window now shows `warn.file.nameMz` from `file_method/1` (FILE-METHOD-WINDOW); the place where a user picks a sample is the scan list, and the warning belongs there too.

**Read first.** `grep -n 'namemz' docs/agenti/INDICE.md`, `docs/agenti/method.md`, `mzlab/web/metodo-file.js` (the warning text with its numbers), the scan list code (`grep -n 'wiffSamples\|sample list' mzlab/web/*.js` narrowly), `mzlab/web/lang/en/errors-file.js`.

**Done means.** The scan list shows the existing `warn.file.nameMz` text with its numbers and never blocks; no new wasm code (the `filemethod` module answers it); an e2e test on a synthetic `.wiff` (`python3 tools/verifica.py --changed`); `python3 tools/controlla_i18n.py` passes.

**Out of scope.** New wasm code, new CLI routes, changing `file_method/1`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
