# English migration: what is still Italian

Measure it with `python3 tools/check_english.py` (counts per area; `--list` one line per file; `--file PATH` every hit). Counts at `main` 4b843e80 plus the docs names PR (the table also counts `mzlab/*.py` and the root files in `other`, not listed):

| area | files | with hits | comment lines | identifiers | keys | file names |
|---|---|---|---|---|---|---|
| `crates/` | 2849 | 933 | 3 | 2080 | 1446 | 258 |
| `mzlab/web/` | 512 | 411 | 3 | 956 | 1683 | 333 |
| `flow/` | 82 | 29 | 0 | 50 | 48 | 1 |
| `TP_Mine/` | 52 | 35 | 1 | 40 | 39 | 15 |
| `tools/` | 410 | 343 | 4 | 1072 | 2164 | 236 |
| `tests_e2e/` | 695 | 624 | 0 | 299 | 1156 | 502 |
| `tests/` | 978 | 756 | 1 | 395 | 1419 | 649 |
| `docs/` | 350 | 313 | 0 | 0 | 53 | 313 |

## Families left, smallest risk first (each one is a PR with a guard test, no aliases, no converters)
1. Tool flags are English (`--update`, `--only`, `--write`, `--list`, `--full`, `--changed`, ...; guard `tests/test_no_old_flag_names.py`); the `cancello` flags follow `cancello/2`. Docs prose: the «HTML sicuro» heading of `data.md` is still open.
2. Panel action registry (`actions/1`, `AZIONI`): all registry fields and the API are English (`group`, `views`, `name`, `run`, `order`, `iconKey`, `submenu`, `checked`, `rows`, `menuLabel`, `AZIONI.register/get/list/forContext/run`, `KEYS`). Left for other families: the file names (`azioni*.js`, `menu-sotto.js`), `fila`/`soloIcona`/`barra`, the layout engine `voci`, `ESC.registra`, `SUGG.mostra`, local names in `azioni-tasti.js`.
Done: the names of the `docs/` folders and files outside `docs/agenti` and `docs/schemi` (`inventories`, `literature_cc`, `release`, `studies`, `users`, their file names and the tooltip zone keys; guard `tests/test_no_old_docs_names.py`), language area names (guard `tests/test_no_old_lang_area_names.py`) and schema ids (`levels/1`, `hypotheses/3` (English outcomes), `tp/2`, `row/2`, `row_traces/2`, `feature_pipeline/3`; guard `tests/test_no_old_schema_ids.py`; old records are refused).
The JSON of the Dig and `tp.*` steps is English too: Dig labels `parent`/`strong`/`possible`/`weak`, MS2 transition keys (`q3_observed`, `q3_suggested`, `rt_expected_min`, ...), registry outcomes `not_searched`/`not_found` and the `tp/2` meta keys (guards `english_json.rs`, `lr_english.rs`).
5. Rust identifiers (`crates/`, `tools/*.rs`, `fuzz/`): by word, rename-only, parity tests unchanged; the words done so far are the `OLD_WORDS` of `tests/test_no_old_rust_names.py` (`apri`, `scrivi`, `chiave`, `lunghezza`, `percorso`, `nodo`, `nome`, `dati`, `membri`). Kept: the `dati_percorsi`/`dati_sintetici` module names (file-name family, 6) and string contents (JSON keys, labels).
6. File and directory names (`docs/agenti` -> `docs/agents` and `docs/schemi`, 904 files cite the first; `tests*/`, `tools/`, `mzlab/web/*.js`, `TP_Mine/`) and the format name `regole_tp` itself. Also open in `docs/`: the Italian prose of `docs/studies/TABUI_MS2_LR.md` (74 lines).
7. `cancello/2` (deferred by the coordinator, untouched).

## Kept on purpose
- External names: `lit:ARCHITETTO_TP_FOTOCATALISI 1.2` (a workbook name) and any real literature or vendor name.
- The gate `tools/gate_release.py` and the generated `docs/agenti/mappa.md` follow the families above, never by hand.
