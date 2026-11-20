# Browser reset: what is left and how to measure
Entry points: "Full reset" (`QQRESET.run("all")`), "Files" (`QQRESET.run("files")`) and the error-banner reset (`window.qqHardReset()`, the same full wipe as `run("all")`). How it works: `data-3200.md` ("Reset vero"); here only the residue and the measuring tools.

## What is left
- **Full reset** and **qqHardReset**: OPFS only `mzlab/` with empty `conv/ feat/ files/ idx/ tiles/ warm/` (recreated by the new page); `mzlab_lab` recreated empty (`kv:1` if a second tab was indexing); caches only `mzlab-app-<version>` (sometimes `mzlab-big-*` from idle prefetch); localStorage only keys written by the new page (`mzlab.launches`, `mzlab.migrated`, `mzlab.bar.*`, `mzlab.memBudget.log`).
- **Files**: kept are `librerie`, `mzlab-libs`, `mzlab-libdir`, the preferences, the user lists (`user_lists:1`) and molecules (`molecules:1`) and the caches that do not start with `mzlab-big-` (also `mzlab-app-*` and foreign caches); the invented folder and database are removed; `kv` is rebuilt (2-3 rows); `conv/ files/ idx/ warm/` stay in `mzlab/`.
- The session marker `mzlab.session` is absent at the start of the new page with all three entry points. After "all" the new page reloads once for isolation (`isolateOnce`) unless it is already `crossOriginIsolated`; "files" reloads once.
- Run-to-run spread is large only where the service worker unregister or a second indexing tab is involved (a second tab indexing makes the OPFS wipe retry for about 2 s).

## How to measure
- `python3 tools/prepara_sito.py` (builds the `.wasm` engine too; `tools/build_site.py` alone is not enough), then `python3 tests_e2e/misura_reset.py [--rep 3] [--only all_a_50,files_b_50] [--out FILE] [-v]`, then delete `site/`. It prints one JSON line per run and a table (median/max); it asserts nothing. `verifica.py` runs only `e2e*.py`, so CI never launches it.
- Probe `tests_e2e/lib_reset.py`: `seed(pg, n)` creates `n` filler files in `mzlab/files` and `mzlab/idx`, an invented folder and database, `librerie`, `mzlab-libs`/`mzlab-libdir`, one list, one molecule, three caches and three preferences; `probe(ctx)` wraps, for timing only, `removeEntry`, `deleteDatabase`, `caches.delete`, `unregister`, `qqEngineStop`, `QQRESET.run`/`last`, the `setItem` calls made during `QQ_RESETTING`, `lr_clear` and `qqReady`; `state(pg)` lists OPFS, databases and rows per store, caches, localStorage, service workers and `storage.estimate()`.
- Binding calls made while the page unloads are lost: the probe also keeps its log in sessionStorage (`__rs.prev`) and the next page sends it again.
- Variants (50 files): a = idle engine, filler files only; a2 = two example files open; b = the same tab indexes a large synthetic mzML (~33 MB, `tests_e2e/mzml_grande.py`); c = a second tab open and idle; d = a second tab indexing; e = user lists read once (`LISTE.ready()`).
- "Total" = click to `qqReady` of the new page. A new page in a second-tab variant (c, d) is not `crossOriginIsolated`, so its total has no isolation reload.
- WebKit and Firefox are not available in the container (Chromium only).
