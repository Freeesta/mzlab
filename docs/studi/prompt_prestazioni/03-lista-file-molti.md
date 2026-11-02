# PERF-03: the Data file list with hundreds of files (measure first)

You are a chat of mzLab (repository Freeesta/mzlab). Read AGENTS.md, `docs/studi/OTTIMIZZAZIONE_PRESTAZIONI.md` §1-2, `docs/agenti/ottimizzazioni-tentate.md` (the 25/10 rows on «pronto» at 6×: `renderFileList` 11 calls = 646 ms, coalescing tried with 6 files and discarded), `docs/agenti/misure-macchine.md`, `docs/agenti/caricamento-mappa.md`. Branch `claude/perf-lista-molti` from `origin/main`. English for code, commits and PR.

**Hypothesis (not yet measured at scale).** `renderFileList` (`mzlab/web/explore.js:772`) rebuilds the whole list with one `innerHTML` and re-wires every handler. It is called from ~50 places, several times per file while files arrive (`explore.js:477`, `484`, `553`, `630`, and `lista-file.js`). Its own work is O(files): group lookup `groups.find` per file, DDA dedupe `findIndex` per file inside a group, `G.fs.every` per group. So a load of N files costs O(N²) on the main thread. At 6 files coalescing gave nothing. The question is whether it does at 100-300 files, which researchers open.

**Do.**
1. **Measure first, change nothing.** Write `tests_e2e/misura_lista_molti.py` (Playwright on the built site, `python3 tools/build_site.py` first; synthetic LR files from `tools/dati_sintetici.py` copied N times with distinct names, NEVER lab data). For N = 20, 100, 300, record:
   - time from the drop to the last list row present and to «pronto» (first TIC drawn, as `misura_macchine.py` defines it);
   - number of `renderFileList` calls and their total time (wrap the global in the page; do not change the product to measure);
   - main-thread long tasks (> 50 ms) during the load.
   Run 1× and 4× CPU throttling (`misura_macchine.py` has the CDP code), 4 runs each, median.
2. **Decide on numbers.** Go to step 3 only if, at N = 100 and 1×, the `renderFileList` total is ≥ 15 % of drop→pronto or ≥ 500 ms. Otherwise stop. Put the table in the PR and add one row «scartata» to the PR body for `ottimizzazioni-tentate.md`, with the numbers and «riprovare solo se».
3. **Change (only if 2 says so).** Make `renderFileList` coalesce: calls in the same frame mark the list dirty, and one `requestAnimationFrame` renders it once. Callers that need the DOM at once must keep that (find them: the tour (`tour.js`), focus restoration after remove, `aria-current`, drag and drop `GDND`, tests that query rows right after a call). Give those a synchronous `renderFileList.now()`, or flush before reading. Also remove the O(N²) inner lookups: a `Map` from group key to group, and a `Set` of DDA bases. Keep the HTML string byte-identical.
4. Re-measure with the same script, same machine, alternating builds.

**Zone (only these files).** `mzlab/web/explore.js` (`renderFileList` and its callers inside explore.js), `mzlab/web/lista-file.js` (only the calls), `tests_e2e/misura_lista_molti.py` (new), one new e2e `tests_e2e/e2e_lista_molti.py` if step 3 runs. Do not touch `shell.js`, `rust-*.js`, `impronta*.js`, `tour.js` (if the tour breaks, the fix is a flush in explore.js), `docs/agenti/misure-velocita.md` and `ottimizzazioni-tentate.md`. Your row for those goes in the PR body under "Row for misure-velocita.md" / "Row for ottimizzazioni-tentate.md".

**Identity test.** `e2e_lista_molti.py`: after a load of 30 files, and after toggling a group and removing a file, `#flst.innerHTML` equals what the synchronous renderer produces for the same state (call `renderFileList.now()` on a clone of the state, or compare against a snapshot taken with coalescing disabled by a test-only flag on `window`). Must stay green: `e2e_lista_casella`, `e2e_lista_colore`, `e2e_lista_hr`, `e2e_lista_ms2`, `e2e_lista_ordine`, `e2e_tour`, `e2e_tour_ancoraggio`, `e2e_axe_grafici` (`python3 tools/verifica.py --cambiati`).

**Parity.** The list is the same page in the site and in the desktop app (`mzlab://`). State in the PR that the change is page-only and that the desktop loads the same `explore.js`.

**Rules.** No new dependency, no engine change, accessibility unchanged (keyboard, `aria-current`, focus). Texts only from `lang/`. `python3 tools/pre_pr.py` before pushing.

**PR.** Title `Render the Data file list once per frame while files arrive` (or, if step 2 stops you: `Measure the Data file list with hundreds of files`). Body starts with `VISIBLE: yes - opening hundreds of files no longer freezes the file list` (or `VISIBLE: no` for a measure-only PR). Ready for review. Send the Coordinator: PR number, 40-char SHA at green CI, the N = 100 numbers before/after.
