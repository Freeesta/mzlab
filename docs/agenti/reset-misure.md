# Browser reset: measurements
Timings and residues of the reset on the built site, for the three entry points: "Full reset" (`QQRESET.run("all")`), "Files" (`QQRESET.run("files")`) and the error-banner reset (`window.qqHardReset()`, which since #3369 is the same full wipe as `run("all")`). How it works: `dati-3200.md` ("Reset vero"); here only the numbers and what is left.

## How to measure
- `python3 tools/prepara_sito.py` (builds the `.wasm` engine too; `tools/build_site.py` alone is not enough), then `python3 tests_e2e/misura_reset.py [--rep 3] [--only all_a_50,files_b_50] [--out FILE] [-v]`, then delete `site/`. It prints one JSON line per run and a table (median/max); it asserts nothing. `verifica.py` runs only `e2e*.py`, so CI never launches it.
- Probe `tests_e2e/lib_reset.py`: `seed(pg, n)` creates `n` filler files in `mzlab/files` and `mzlab/idx`, an invented folder and database, `librerie`, `mzlab-libs`/`mzlab-libdir`, one list, one molecule, three caches and three preferences; `probe(ctx)` wraps, for timing only, `removeEntry`, `deleteDatabase`, `caches.delete`, `unregister`, `qqEngineStop`, `QQRESET.run`/`last`, the `setItem` calls made during `QQ_RESETTING`, `lr_clear` and `qqReady`; `state(pg)` lists OPFS, databases and rows per store, caches, localStorage, service workers and `storage.estimate()`.
- Binding calls made while the page unloads are lost: the probe also keeps its log in sessionStorage (`__rs.prev`) and the next page sends it again.
- Variants (50 files): a = idle engine, filler files only; a2 = two example files open; b = the same tab indexes a large synthetic mzML (~33 MB, `tests_e2e/mzml_grande.py`); c = a second tab open and idle; d = a second tab indexing; e = user lists read once (`LISTE.ready()`).
- "Total" = click to `qqReady` of the new page. After "all" the new page reloads once for isolation (`isolateOnce`, two document requests, ~0.3-0.4 s included in the total) unless it is already `crossOriginIsolated`; "files" reloads once.
- WebKit and Firefox are not available in the container (Chromium only).

## Numbers
Chromium, origin/main 34b3f13 (2026-11-17), 3 runs, median/max in ms. These replace the 2026-11-14 tables (origin/main 198fea6), which predate #3324 (lists no longer hold `mzlab_lab` open), #3326, #3365 (one 10 s OPFS deadline, folders removed in parallel), #3369, #3375 and #3376; none of the old numbers is kept.
- MEASURE reset_all_a (1/50/240 files): first `removeEntry` after `qqEngineStop` 15/27, 14/18, 14/15; OPFS 7/15, 24/57, 92/98; IndexedDB 8/12, 8/16, 9/12; wipe 16/30, 33/70, 108/1932; reload to ready 700/755, 712/724, 757/758; total 719/814, 769/798, 871/2702. No `NoModificationAllowedError`. The 2702 at 240 files is one run whose service-worker unregister took 1925 ms.
- MEASURE reset_all_a2 (two example files open, 50 files): wipe 33/42, total 741/850.
- MEASURE reset_all_b (same tab indexing, 50 files): OPFS 292/446 with 1/1 retry; wipe 304/455; total 1266/1495.
- MEASURE reset_all_c (second tab idle): OPFS 20/23, total 350/357; one document load only and the new page is NOT `crossOriginIsolated` (3/3, also in all_d and hard_c, hard_d, never with "files"): the low total is the missing isolation reload.
- MEASURE reset_all_d (second tab indexing): OPFS 1798/1799 with 7/7 retries; total 2128/2193; the other tab keeps its 2 files and rewrites `kv` (1 row) in the recreated database.
- MEASURE reset_all_e (lists read once): no database blocked (`mzlab_lab` is no longer held open); total 820/915; lists and molecules gone.
- MEASURE reset_files_a (1/50/240 files): wipe 19/55, 24/26, 84/90; OPFS 5/7, 18/21, 78/83; total 558/566, 415/421, 506/520; a2 total 532/560.
- MEASURE reset_files_bcde (50 files): b OPFS 299/406 with 1/1 retry, total 809/882; c total 440/462; d OPFS 1794/1796 with 7/7 retries, total 2205/2207; e total 471/474.
- MEASURE reset_hard (qqHardReset, same wipe as "all"): total a 753/940, 769/809, 897/1092 (1/50/240 files); a2 803/883; b 1177/1362; c 392/408; d 2201/2205; e 793/803. No database blocked, no unfinished step reported in any variant.
- MEASURE reset_session: the session marker `mzlab.sessione` is absent at the start of the new page in 0/3 of every variant, with all three entry points (#3375).

## What is left
- **Full reset** and **qqHardReset**: OPFS only `mzlab/` with empty `conv/ feat/ files/ idx/ tiles/ warm/` (recreated by the new page); `mzlab_lab` recreated empty (`kv:1` if a second tab was indexing); caches only `mzlab-app-<version>` (sometimes `mzlab-big-*` from idle prefetch); localStorage only keys written by the new page (`mzlab.avvii`, `mzlab.migrato`, `mzlab.barra.*`, `mzlab.memBudget.log`).
- **Files**: kept are `librerie`, `mzlab-libs`, `mzlab-libdir`, the preferences, the user lists (`liste_utente:1`) and molecules (`molecole:1`) and the caches that do not start with `mzlab-big-` (also `mzlab-app-*` and foreign caches); the invented folder and database are removed; `kv` is rebuilt (2-3 rows); `conv/ files/ idx/ warm/` stay in `mzlab/`.
- Run-to-run spread is large only where the service worker or a second indexing tab is involved (all_a_240 total max 2702, the d variants about 2.2 s).
