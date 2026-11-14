# English code: inventory of the Italian left
Every identifier, comment, file and directory name of the code is English; the user-facing texts stay bilingual in `mzlab/web/lang/it/` and `lang/en/` (Italian text there is correct). Quotes of user-visible strings in comments are allowed. This file is the measured state; `tools/check_english.py` measures it again.

## The tool
- `python3 tools/check_english.py`: counts per area (report only, exit 0). `--elenco`: one line per file. `--file PATH`: every hit with its line. `--rinomine`: the rename map of `tools/english_rename_map.py` as TSV (`kind`, old path, new path, note), deterministic for a given tree.
- Word list: `tools/italian_words.tsv` (Italian → proposed English), whole words only, no stemming, so `controller` or `integrate` never match. Names are split on `_`, `-`, `.`, camelCase; a name is Italian when one of its words is in the list.
- Ambiguous words (also English or Latin: `solo`, `coda`, `piano`, `con`, `non`, `le`, `di`, `da`, `col`, `vista`, `tempo`, …, list `AMBIGUOUS`) count only next to a clear Italian word; proper nouns (`attributio`, `tabui`, `rosina`, …, list `PROPER`) never count. In paths every listed word counts.
- Comments: a comment is Italian when it has an Italian-only accented word (`è`, `più`, `perché`, …), or at most three words with one Italian, or two Italian words making at least 25 % of its words. Quotes («», "", backticks), file names, JSDoc types and code-like tokens (`a.b`, `x_y`, camelCase, CAPS) are removed first; JSDoc types and `@typedef` names are counted as identifiers.
- Keys: string literals without spaces (`storia.fondo`, `#np-liste`, `campioni/2`) — lang keys, CSS selectors, ids, schema names, file paths. In `lang/` only the keys are read, never the texts.
- Not read: `vendor/`, `mzlab/web/esempi/`, `mzlab/web/elements.js`, `target/`, `graphify-out/`, files over 2 MB, test data JSON (their names are still checked). Markdown prose is counted apart (`prose_lines`): docs are not code.
- Known limits: compound words without a separator (`mzmlvalida`, `calcqui`) are only found when listed; JS regex literals matching old Italian titles (compatibility with saved sessions) count as keys; English words that are also Italian words in the list can still give a rare false hit.

## Measured inventory
Counts per area. `identifiers` and `keys` are distinct names per file, summed over files; `comment_lines` are comments (a block counts once); `it_names` are files whose path has an Italian word.

| area | files | files with hits | comment lines | identifiers | keys | Italian paths | prose lines |
|---|---|---|---|---|---|---|---|
| `crates/` | 2268 | 1221 | 10 | 9493 | 3004 | 708 | 0 |
| `mzlab/web/` | 443 | 362 | 22 | 1889 | 2621 | 300 | 0 |
| `flow/` | 78 | 38 | 0 | 135 | 109 | 1 | 0 |
| `TP_Mine/` | 68 | 55 | 2 | 166 | 116 | 27 | 0 |
| `tools/` | 331 | 318 | 4 | 2460 | 2447 | 297 | 0 |
| `tests_e2e/` | 618 | 599 | 7 | 743 | 1815 | 467 | 0 |
| `tests/` | 850 | 708 | 19 | 969 | 1462 | 630 | 175 |
| `docs/` | 629 | 624 | 7 | 89 | 430 | 624 | 15100 |
| `mzlab/*.py` | 15 | 7 | 0 | 7 | 3 | 3 | 0 |
| other (`fuzz/`, `proxy/`, `normative/`, root, `.claude/`, `.github/`) | 132 | 57 | 1 | 275 | 50 | 29 | 98 |

- Distinct Italian identifiers in the whole repository: 3500 (1864 in `crates/`); distinct Italian keys: 4606. Lang keys: 717 of the 4961 keys of `lang/it/` have an Italian word (`asse.*`, `canali.*`, `storia.*`, …).
- Most frequent Italian words in identifiers (number of distinct names): `dati` 403, `lettori` 384, `nome` 346, `attendi` 306, `campioni` 226, `lettore` 223, `sito` 222, `metodo` 201, `scansione` 195, `apri` 177, `indice` 173, `sintetici` 172, `voce` 172, `righe` 169, `tipo` 166, `grafo` 160, `limiti` 159, `testo` 150, `tabella` 149.
- Comments are almost all English already (the tool's own files and this file carry Italian names because the task named them so; they follow the `tools/` and `docs/` renames): the 65 Italian comment lines outside `docs/` are section rulers of the classic scripts (`eventi.js`, `explore.js`, `pannelli.js`), the docstrings of `tests/data/nomi_tp/*.py`, a few test docstrings and the step comments of `tests_e2e/e2e_tour.py`.

## Rename map (summary of `--rinomine`)
- 125 directories and 2418 files change name (file rows show the path after the directory renames). Word-by-word translation, then three Italian word-order rules: an adjective after its noun is swapped (`lettore_finto` → `fake_reader`, `campioni_variabili` → `variable_samples`), `X di Y` → `Y X`, and a head noun before its complement is swapped (`calibrazione_massa` → `mass_calibration`; heads in `HEADS`; quantifiers and file-kind words such as `prove`, `vista`, `tour` are never complements).
- Topic-first names (`gruppi-vista`, `e2e_mappa_assi`) are already English order and keep it.
- Hand choices where a name collides (`OVERRIDES`: `corsa.rs` of `flow/` → `run_report.rs`, `msnalbero.rs` → `msntree_build.rs`, `avanzamento.js` → `job-progress.js`); a Rust module directory follows its file (`msnalbero/` → `msntree_build/`). MassBank accessions (`MSBNK-…`) are kept.

| area | dir renames | file renames | named by `.github/` | word order fired (`REORDERED`) |
|---|---|---|---|---|
| `crates/` | 54 | 551 | 2 (`mzlab-lettori`, `mzlab-feat-grafo-wasm`) | 15 |
| `mzlab/web/` | 2 | 300 | 4 | 11 |
| `flow/` | 0 | 1 | 0 | 0 |
| `TP_Mine/` | 0 | 27 | 0 | 0 |
| `tools/` | 6 | 265 | 28 | 16 |
| `tests_e2e/` | 0 | 467 | 1 | 31 |
| `tests/` | 25 | 423 | 4 | 23 |
| `docs/` | 36 | 355 | 8 (`docs/agenti`, `docs/schemi`, `docs/utenti`, …) | 32 |
| `mzlab/*.py` | 0 | 3 | 0 | 0 |
| other | 2 | 26 | 2 (the workflow files themselves) | 0 |

- Notes of the map: `GITHUB` = a `.github/` file names the old path (crate names included): it is renamed only in the final batch, together with the workflow. `MERGE` = the new directory exists (`tests/dati` → `tests/data`). `COLLISION` = two sources, an existing file, or a module directory next to another module's file get the same path (none left). `REORDERED` = a word-order rule fired: read the row before the rename.
- Crate renames (`mzlab-lettori` → `mzlab-readers`, `mzlab-picchi` → `mzlab-peaks`, `mzlab-rotte-wasm` → `mzlab-routes-wasm`, `mzlab-msngrafo-wasm`, `mzlab-msnfigura-wasm`, `mzlab-msnregole-wasm`, `mzlab-lettori-wasm`, `mzlab-feat-grafo-wasm`) also rename the `.wasm` files: `sw.js` precache, `tools/build_site.py`, the caps of `wasm-tetti.md` and the size checks follow.

## Scope (decisions)
- Out of scope: the Markdown prose of `docs/` stays Italian (15100 lines); only `docs/` file and directory names are renamed, and only those without the `GITHUB` note.
- Kept as they are, never renamed: serialized names (schema names and JSON keys of saved files such as `campioni/2`, `corsa/1`, `metodo/1`, `mzflow/1` keys `formato`, `programma`, `operazioni`; OPFS paths; IndexedDB and `localStorage` keys; regex literals that read old Italian session titles). Renaming them would break files users already saved.
- The 49 `GITHUB` paths (workflows, `docs/agenti`, `docs/schemi`, `docs/utenti`, `mzlab-lettori`, `mzlab-feat-grafo-wasm`, tools run by workflows, `fuzz/regressioni`, `tests/golden/librerie`, `tools/banco_wasm`, `tools/tipi`) are a separate final batch, started only with the owner's explicit approval at that point.
- Queue gates: identifier batches go one area per PR while fewer than 7 PRs are open; `git mv` renames only while fewer than 4 are open. One English PR at a time.

## Rename rules
- Comments that only name an Italian identifier, CLI flag, schema key or quoted UI string change with that name.
- Classic-script globals (`TITOLI`, `MAPPA`, `REGISTRO`, …) are shared across files: one global per commit, with every user.
- Public Rust API and wasm exports (`pub fn`, `#[wasm_bindgen]` names called from JS): crate by crate, site and app together, parity tests unchanged.
- Keys seen from outside: CSS ids/classes and `data-*` (e2e selectors and `tour.js` follow), lang keys (`tools/controlla_i18n.py` and `lang/contesto.json` follow), CLI flags (`--strutture`, `--cicli`, `--giri`) with the old flag kept as an alias for one release.
- File renames with `git mv`: `tests_e2e/` renames update the `AREAS` map of `tools/verifica.py`; `mzlab/web/` renames update the `index.html` order, `sw.js` and `tools/build_site.py`; crate renames go last in `crates/`, `docs/` names last overall; rows with the `GITHUB` note wait for the final batch.
- Rust identifiers (local and public) go only when the name is defined nowhere but as a `fn`, method, local or `const` (no field, variant, type, module, macro body or attribute; no `#[wasm_bindgen]`, `#[tauri::command]` or `no_mangle` export), as a `fn`, method or `const` is named in no non-Rust code file and in no backticked doc span (a local is seen by no other file), and its English name is not already a token of any file using it. Test names that cite a serialized key, a schema (`campioni/2`) or a route name keep that word.

## Tabui: English only
- Rules: every Tabui text (interface, tables, tooltips, reports, exported columns) is English; «formula» is never bare (`Neutral formula`, `Ion formula`, `neutral_formula`); no decorative Unicode; no `lang/` keys, tooltips only in `TP_Mine/js/40_tabui_testi.js` (`TABUI_T.en`). Guard: `python3 -m pytest tests/test_tabui_inglese.py`.
- Open: `p.segno` (`contro`, `pro`) in `TP_Mine/js/34_pipeline_run.js` to align to `against` / `supports`: only `contro` gets an English label (`against`), `pro` is shown as is.
- Kept as internal identifiers (never shown): select values `nessuna`, `somma`, `mediana` of `TP_Mine/js/37_feature_table.js` (Rust normalisation parameters); `"predefinito": "fotocatalisi"` of `TP_Mine/data/regole_tp_2.json`; the `hojas`/`nombre`/`filas` keys of `crates/mzlab-feat/src/export.rs` read by `xlsx.js`; object keys such as `Q3_osservato`, `RT_attesa_min` of `TP_Mine/js/20_tpmine.js` (exported headers are English).
