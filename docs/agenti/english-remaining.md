# English migration: what is still Italian

Measure it with `python3 tools/check_english.py` (counts per area; `--list` one line per file; `--file PATH` every hit). Counts at `main` 7f973633:

| area | files | with hits | comment lines | identifiers | keys | file names |
|---|---|---|---|---|---|---|
| `crates/` | 2829 | 950 | 5 | 2813 | 1466 | 258 |
| `mzlab/web/` | 511 | 420 | 4 | 1193 | 1697 | 354 |
| `flow/` | 82 | 29 | 0 | 50 | 48 | 1 |
| `TP_Mine/` | 52 | 35 | 1 | 40 | 40 | 15 |
| `tools/` | 410 | 346 | 4 | 1170 | 2173 | 236 |
| `tests_e2e/` | 693 | 622 | 0 | 304 | 1159 | 500 |
| `tests/` | 969 | 750 | 1 | 395 | 1394 | 647 |
| `docs/` | 348 | 346 | 0 | 0 | 53 | 346 |

## Families left, smallest risk first (each one is a PR with a guard test, no aliases, no converters)
1. Tool flags are English (`--update`, `--only`, `--write`, `--list`, `--full`, `--changed`, ...; guard `tests/test_no_old_flag_names.py`); the `cancello` flags follow `cancello/2`. Docs prose: the «HTML sicuro» heading of `data.md` is still open.
2. Panel action registry (`actions/1`, `AZIONI`): all registry fields and the API are English (`group`, `views`, `name`, `run`, `order`, `iconKey`, `submenu`, `checked`, `rows`, `menuLabel`, `AZIONI.register/get/list/forContext/run`, `KEYS`). Left for other families: the file names (`azioni*.js`, `menu-sotto.js`), `fila`/`soloIcona`/`barra`, the layout engine `voci`, `ESC.registra`, `SUGG.mostra`, local names in `azioni-tasti.js`.
Done: language area names (guard `tests/test_no_old_lang_area_names.py`) and schema ids (`levels/1`, `hypotheses/2`, `row/2`, `row_traces/2`, `feature_pipeline/3`; guard `tests/test_no_old_schema_ids.py`; old records are refused).
5. Rust identifiers (`crates/`: `mzlab-feat`, `-ops`, `-cli`, `-core`): by crate, rename-only, parity tests unchanged.
6. File and directory names (`tests*/`, `tools/`, `mzlab/web/*.js`, `TP_Mine/`) and the format name `regole_tp` itself.
7. `cancello/2` (deferred by the coordinator, untouched).

## Kept on purpose
- External names: `lit:ARCHITETTO_TP_FOTOCATALISI 1.2` (a workbook name) and any real literature or vendor name.
- The gate `tools/gate_release.py` and the generated `docs/agenti/mappa.md` follow the families above, never by hand.
