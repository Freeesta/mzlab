# English code: inventory of the Italian left
Every identifier, comment, file and directory name of the code is English; the user-facing texts stay bilingual in `mzlab/web/lang/it/` and `lang/en/` (Italian text there is correct). Quotes of user-visible strings in comments are allowed. This file is the measured state; `tools/controlla_inglese.py` measures it again.

## The tool
- `python3 tools/controlla_inglese.py`: counts per area (report only, exit 0). `--elenco`: one line per file. `--file PATH`: every hit with its line. `--rinomine`: the rename map as TSV (`kind`, old path, new path, note), deterministic for a given tree.
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
125 directories and 2418 files change name (file rows show the path after the directory renames). Word-by-word translation; an Italian adjective after its noun is swapped (`lettore_finto` → `fake_reader`); hand choices where a name collides (`OVERRIDES`: `flow/corsa.rs` → `run_report.rs`, `msnalbero.rs` → `msntree_build.rs`, `avanzamento.js` → `job-progress.js`); MassBank accessions (`MSBNK-…`) are kept.

| area | dir renames | file renames | files moved only by a dir rename | named by `.github/` | word order to read (`ORDER`) |
|---|---|---|---|---|---|
| `crates/` | 54 | 551 | 331 | 2 (`mzlab-lettori`, `mzlab-feat-grafo-wasm`) | 88 |
| `mzlab/web/` | 2 | 300 | 2 | 4 | 88 |
| `flow/` | 0 | 1 | 0 | 0 | 0 |
| `TP_Mine/` | 0 | 27 | 0 | 0 | 5 |
| `tools/` | 6 | 265 | 74 | 28 | 78 |
| `tests_e2e/` | 0 | 467 | 0 | 1 | 193 |
| `tests/` | 25 | 423 | 254 | 4 | 117 |
| `docs/` | 36 | 355 | 623 | 8 (`docs/agenti`, `docs/schemi`, `docs/utenti`, …) | 129 |
| `mzlab/*.py` | 0 | 3 | 0 | 0 | 0 |
| other | 2 | 26 | 3 | 2 (the workflow files themselves) | 5 |

- Notes of the map: `GITHUB` = a `.github/` file names the old path (crate names included): it is renamed only with Federico, together with the workflow. `MERGE` = the new directory exists (`tests/dati` → `tests/data`). `COLLISION` = two sources or an existing file get the same path (none left). `ORDER` = two or more Italian words: the English word order is a proposal to read before the rename.
- Crate renames (`mzlab-lettori` → `mzlab-readers`, `mzlab-picchi` → `mzlab-peaks`, `mzlab-rotte-wasm` → `mzlab-routes-wasm`, `mzlab-msngrafo-wasm`, `mzlab-msnfigura-wasm`, `mzlab-msnregole-wasm`, `mzlab-lettori-wasm`, `mzlab-feat-grafo-wasm`) also rename the `.wasm` files: `sw.js` precache, `tools/build_site.py`, the caps of `wasm-tetti.md` and the size checks follow.

## Batch plan, by risk
1. Comments, all areas in one PR (65 lines + `tests/data/nomi_tp/` docstrings): no behaviour.
2. Local identifiers, one area per PR: `tools/` and `tests/` (Python locals and private helpers), then `crates/` non-`pub` items, then `TP_Mine/` and `flow/`, then `mzlab/web/` (classic-script globals such as `TITOLI`, `MAPPA`, `REGISTRO` are shared across files: one global per commit, with every user).
3. Public Rust API and wasm exports (`pub fn`, `#[wasm_bindgen]` names called from JS): crate by crate, site and app together, parity tests unchanged.
4. Keys seen from outside: CSS ids/classes and `data-*` (e2e selectors and `tour.js` follow), lang keys (717, `tools/controlla_i18n.py` and `lang/contesto.json` follow), CLI flags (`--strutture`, `--cicli`, `--giri`). Old CLI flags need an alias for one release.
5. Serialized names (highest risk, decision of Federico): schema names and JSON keys of saved files (`campioni/2`, `corsa/1`, `metodo/1`, `mzflow/1` keys `formato`, `programma`, `operazioni`), OPFS paths, IndexedDB and `localStorage` keys, regex literals that read old Italian session titles. Renaming them breaks files users already saved unless the reader accepts both names; the proposal is to keep them or to bump the schema version with a reader of the old one.
6. File renames with `git mv`, smallest coupling first: `mzlab/*.py` (3), `flow/` (1), `TP_Mine/` (27), `tests_e2e/` (467, plus the `AREE` map of `tools/verifica.py`), `tests/` (423 + 25 directories), `tools/` (265 + 6 directories, 28 named by workflows stay), `mzlab/web/` (300: `index.html` order, `sw.js`, `tools/build_site.py`), `crates/` (551 + 54 directories, crate renames last), `docs/` last (`docs/agenti` is named by CLAUDE.md, the workflows and every chat: with Federico).
