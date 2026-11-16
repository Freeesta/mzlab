# Audit of unused and old files (report only)
Verified list of tracked files that look unused, dead, duplicated or obsolete, for the owner to decide what to remove. Nothing here was deleted or moved. Sizes in bytes; «last change» = date of the last commit that touched the path (`git log -1 --format=%cs`, full history; a rename counts as a change, so many `tools/` files show the day of their English rename).

## Method
- Inventory: `git ls-files` (5 759 files), size per file, last-change date from one pass of `git log --name-only` over the full history.
- References: every tracked text file tokenised; a file with no other file naming its basename became a candidate (1 238), then each candidate was checked for dynamic loading: directory joins (`join("../../tests/data/x")`, `read_dir`, `glob`), string-built names (`format!("{name}.json")`, `"static/an-" + n + ".js"`, `lang/en.js` list), `hashFiles(...)` in workflows, `tools/moduli/*.toml`, `tools/build_site.py` copies, pytest `testpaths`, `tools/verifica.py` `E2E.glob("e2e*.py")`, `proxy/test/all.mjs`, Tauri `icon` list, Cargo workspace members.
- Rust: module reachability from every `lib.rs`/`main.rs`/`bin`/`build.rs`/`tests`/`benches`/`examples` root following `mod x;` and `#[path]` (2 442 `.rs`, 2 not reached); every crate has a dependent or a `tools/moduli/*.toml`; every `[features]` key is used by a `cfg(feature)` or forwarded to a dependency.
- Generated or untracked outputs (`docs/agenti/mappa.md`, `graphify-out/`, `site/`) are not tracked, so `git grep` never counts them as a reference.
- Proof of «no reference» = the `git grep` command shown with an empty result (run on the tree of this branch).

## SAFE-TO-DELETE (10 files, 32 085 bytes)
No reference at all; deleting changes no build, test, site or app.

| File | Bytes | Last change | Proof (empty) and note |
|---|---|---|---|
| `crates/mzlab-feat/src/rtwarp/before.rs` | 4 862 | 2026-10-28 | `git grep -nE '\bmod (before\|tests)\b' -- crates/mzlab-feat/src/rtwarp.rs` and `git grep -nE 'rtwarp/(before\|tests)\|before\.rs'`: no `mod` declares it, never compiled. |
| `crates/mzlab-feat/src/rtwarp/tests.rs` | 2 786 | 2026-10-28 | Same proof. It is a bit-identity test of LOESS/RANSAC that has never run: the alternative to deleting both is `#[cfg(test)] mod before; #[cfg(test)] mod tests;` in `rtwarp.rs` (then `cargo test -p mzlab-feat rtwarp`). |
| `tools/tipi/moduli.tsbuildinfo` | 1 140 | 2026-10-13 | `git grep -n tsbuildinfo -- ':!tools/tipi/*.tsbuildinfo'`. `tools/tipi/base.json` has `noEmit` and no `incremental`/`composite`; `controlla_tsc.py` runs `tsc -p`, which neither reads nor writes it. Stale tsc state. |
| `tools/tipi/pagina.tsbuildinfo` | 3 208 | 2026-10-13 | Same as above. |
| `tools/tipi/worker.tsbuildinfo` | 459 | 2026-10-13 | Same as above. |
| `tests_e2e/misura_raw_carico.py` | 4 602 | 2026-11-09 | `git grep -n misura_raw_carico -- ':!tests_e2e/misura_raw_carico.py'`. Manual timing script; not `e2e*`, so `verifica.py` never runs it. |
| `tests_e2e/misura_sul_posto.py` | 7 196 | 2026-11-14 | `git grep -n misura_sul_posto -- ':!tests_e2e/misura_sul_posto.py'`. Manual timing script; its only doc mention went with the deleted `docs/studi` studies. |
| `tools/bench/genera_d2.py` | 1 756 | 2026-11-08 | `git grep -n genera_d2 -- ':!tools/bench/genera_d2.py'`. `tools/bench/rapporto.py --d2` builds the D2 series itself. |
| `tools/measure_reload_summary.py` | 5 291 | 2026-11-14 | `git grep -n measure_reload_summary -- ':!tools/measure_reload_summary.py'`. Summary of `misura_ricarica.py` output; no doc or caller left. |
| `TP_Mine/tests/rete_vista.html` | 785 | 2026-10-20 | `git grep -n rete_vista -- ':!TP_Mine/tests/rete_vista.html'`. Hand test page of `rete-vista.js`; `tests_e2e/e2e_rete.py` mounts the view itself. |

## PROBABLY-UNUSED (56 files, 1 611 580 bytes)
Only self, sibling-README or doc references; no code, test, workflow or build reads them.

### Assistant study data (14 files, 30 100 bytes)
Proof: `git grep -nE 'copioni|oro/(G|U|fumo)|insiemi\.json' -- ':!docs/studi'` gives only `tools/italian_words.tsv` (dictionary) and a base64 match inside `mzlab/web/esempi/HRMS_t000.mzML`. They are described in `docs/studi/assistente-dati/06-valutazione.md` and `prompt/00_esecuzione-test.md` for an `assist run --copione` command that `mzlab-cli` does not have. `oro/R.jsonl` and `reg/*.jsonl` are live (`llm_scope_tests.rs`, `llm_assist_session_tests.rs`).

| File | Bytes | Last change |
|---|---|---|
| `tests/data/assist/copioni/T10.json` … `T19.json` (10 files) | 4 767 | 2026-10-22 |
| `tests/data/assist/oro/G.jsonl` | 7 254 | 2026-10-22 |
| `tests/data/assist/oro/U.jsonl` | 13 747 | 2026-10-22 |
| `tests/data/assist/oro/fumo.jsonl` | 1 388 | 2026-10-22 |
| `tests/data/assist/insiemi.json` | 2 944 | 2026-10-22 |

### `tests/data/nomi_tp/` (5 files, 51 641 bytes)
Self-contained generator + checker of a TP-name case set; nothing else reads it. Proof: `git grep -nE 'nomi_tp|casi_nomi' -- ':!tests/data/nomi_tp'` gives only a mention in `docs/agenti/inglese.md` and in `tests/data/decoy/LEGGIMI.md`.

| File | Bytes | Last change |
|---|---|---|
| `tests/data/nomi_tp/LEGGIMI.md` | 1 276 | 2026-10-16 |
| `tests/data/nomi_tp/casi_nomi.csv` | 9 213 | 2026-10-16 |
| `tests/data/nomi_tp/controlla_casi.py` | 9 600 | 2026-11-08 |
| `tests/data/nomi_tp/genera_casi.py` | 20 579 | 2026-11-08 |
| `tests/data/nomi_tp/trasformazioni.csv` | 10 973 | 2026-10-16 |

### Web assets nobody requests (3 files, 164 760 bytes)
| File | Bytes | Last change | Note |
|---|---|---|---|
| `mzlab/web/esempio-frammentazione-m.png` | 56 811 | 2026-10-11 | `git grep -nE 'esempio-(frammentazione\|trasformazione)-m'` is empty. `build_site.py` copies all of `mzlab/web/` to `static/`, so the generated `sw-manifest.json` precaches it, but no page loads it; the lab build already deletes it (`lab_variant`). |
| `mzlab/web/esempio-trasformazione-m.png` | 88 361 | 2026-10-11 | Same as above. |
| `mzlab/web/app-icon.svg` | 19 588 | 2026-10-09 | `git grep -n app-icon.svg -- ':!tools/generate_icons.py'` is empty: written by `generate_icons.py`, but `index.html` uses `favicon.svg` and the web manifest the PNG icons. Removing it also means dropping that line of the generator. |

### Docs outside `docs/agenti/` (16 files, 1 289 785 bytes)
| File | Bytes | Last change | Note |
|---|---|---|---|
| `docs/pr-img/adducts-compact/after-expanded.png` | 32 532 | 2026-11-04 | `docs/pr-img/`: before/after screenshots linked only from PR descriptions on GitHub (`git grep -n pr-img -- ':!docs/pr-img'` → only the exclusion in `tools/verifica_aree.py`). Deleting them breaks the images in those merged PR pages. |
| `docs/pr-img/adducts-compact/after-folded.png` | 24 583 | 2026-11-04 | as above |
| `docs/pr-img/adducts-compact/before-expanded.png` | 36 089 | 2026-11-04 | as above |
| `docs/pr-img/adducts-compact/before-folded.png` | 26 572 | 2026-11-04 | as above |
| `docs/pr-img/dati-una-pagina-1000-dopo-tabella.png` | 90 165 | 2026-11-15 | as above |
| `docs/pr-img/dati-una-pagina-1000-prima-panels.png` | 69 248 | 2026-11-15 | as above |
| `docs/pr-img/dati-una-pagina-1366-dopo-tabella.png` | 115 816 | 2026-11-15 | as above |
| `docs/pr-img/dati-una-pagina-1366-prima-panels.png` | 91 000 | 2026-11-15 | as above |
| `docs/pr-img/dati-una-pagina-1600-dopo-tabella.png` | 119 051 | 2026-11-15 | as above |
| `docs/pr-img/dati-una-pagina-1600-prima-panels.png` | 113 784 | 2026-11-15 | as above |
| `docs/pr-img/flow-layout/after-1000.png` | 250 732 | 2026-11-14 | as above |
| `docs/pr-img/flow-layout/before-1000.png` | 262 108 | 2026-11-14 | as above |
| `docs/letteratura_cc/RAPPORTO.md` | 22 067 | 2026-10-16 | `git grep -n 'RAPPORTO\.md' -- ':!docs/letteratura_cc/RAPPORTO.md'` is empty; the CSVs next to it are live (`tests/data/reattivita/validazione.json`). |
| `docs/studi/CONFRONTO_MZFLOW.md` | 23 853 | 2026-11-14 | Recent study, not in INDICE; only the roadmap below links it (`git grep -nE 'CONFRONTO_MZFLOW\|ROADMAP_MZFLOW'` outside the pair is empty). |
| `docs/studi/ROADMAP_MZFLOW.md` | 12 545 | 2026-11-14 | as above |
| `TP_Mine/tests/gruppi_vista.html` | 607 | 2026-10-16 | Hand test page; only `docs/agenti/gruppi.md` names it (`e2e_tabui_gruppi_vista.py` mounts the view itself). |

### Documented manual tools with no caller (18 files, 74 327 bytes)
Not imported or run by any test, workflow, `pre_pr.py` or `verifica.py`; each is described only in the doc named. They are measurements/benches run by hand: remove one only if its doc procedure is obsolete, and fix that doc in the same PR.

| File | Bytes | Last change | Only referenced by |
|---|---|---|---|
| `tools/bench/corri_asari.py` | 1 889 | 2026-11-08 | `mzfinder-benchmark.md` |
| `tools/bench/corri_openms.py` | 2 972 | 2026-11-08 | `mzfinder-benchmark.md` |
| `tools/bench/parita_raw.py` | 3 135 | 2026-11-07 | `mzfinder-benchmark.md` |
| `tools/bench/prepara_d1.py` | 2 208 | 2026-11-08 | `mzfinder-benchmark.md` |
| `tools/bench/thermo_velocita.py` | 3 960 | 2026-11-07 | `thermo-velocita.md`, INDICE |
| `tools/bench_librerie_cartella.py` | 3 399 | 2026-11-14 | `misure-rust.md` |
| `tools/bench_raw.py` | 5 227 | 2026-11-14 | `misure-rust.md`, `lettori-modulo.md`, `mappa-fluida.md`, `thermo-velocita.md` |
| `tools/bench_table_hr.py` | 4 062 | 2026-11-13 | `alta-risoluzione-statistica.md`, `misure-grafici.md`, INDICE |
| `tools/compare_profiles.py` | 1 883 | 2026-11-13 | `rilascio.md` |
| `tools/lat_arrows.py` | 4 488 | 2026-10-12 | `test.md` |
| `tools/measure_formats.py` | 9 613 | 2026-11-13 | `misure-formati.md` |
| `tools/measure_groups.py` | 4 482 | 2026-11-13 | `gruppi.md`, `misure-rust.md` |
| `tools/measure_redraw.py` | 1 018 | 2026-11-13 | `misure-diario.md` |
| `tools/mzml_diagnosis.py` | 3 426 | 2026-11-13 | `diagnosi.md` |
| `tools/state_pr.py` | 4 479 | 2026-11-13 | `ci-veloce.md` |
| `tools/synthetic_data_large.py` | 7 892 | 2026-11-13 | `misure-rust.md`, `misure-velocita.md`, `misure-velocita-raw.md` |
| `tools/test_isotopes.py` | 4 718 | 2026-11-13 | `alta-risoluzione.md` (lives in `tools/`, so pytest `testpaths = ["tests"]` never collects it) |
| `tools/wiff_probe.py` | 5 476 | 2026-11-13 | `wiff.md` |

## UNSURE (9 files, 86 005 bytes)
| File | Bytes | Last change | Why unsure |
|---|---|---|---|
| `docs/schemi/runorder-1.json` | 2 724 | 2026-10-30 | `git grep -n runorder-1.json` is empty: the only schema of `docs/schemi/` no test validates against, but `runorder/1` is a live output (`crates/mzlab-feat/src/runorder.rs`). Better wired into a test than deleted. |
| `tools/generate_preview.py` | 3 277 | 2026-11-13 | `git grep -n generate_preview -- ':!tools/generate_preview.py'` is empty, yet it is the only generator of the live `mzlab/web/anteprima.png` (link preview). |
| `docs/inventari/TOOLTIP-disegno.md` | 8 423 | 2026-11-13 | The seven `TOOLTIP-*.md` are written by `python3 tools/inventory_tooltip.py --md docs/inventari`; no test checks they are current and nothing links them except the generator. `TOOLTIP.md` itself is live (read by tests). |
| `docs/inventari/TOOLTIP-liste-1.md` | 21 411 | 2026-11-13 | as above |
| `docs/inventari/TOOLTIP-liste-2.md` | 918 | 2026-11-13 | as above |
| `docs/inventari/TOOLTIP-pannelli-1.md` | 21 392 | 2026-11-13 | as above |
| `docs/inventari/TOOLTIP-pannelli-2.md` | 2 197 | 2026-11-13 | as above |
| `docs/inventari/TOOLTIP-spettri.md` | 14 695 | 2026-11-13 | as above |
| `docs/inventari/TOOLTIP-tabui.md` | 10 968 | 2026-11-13 | as above |

## KEEP-BUT-OLD (referenced; no action)
Oldest or zero-basename-reference groups that are live, with what reads them.

| Files | Bytes | Oldest change | Read by |
|---|---|---|---|
| `mzlab/web/an-*.js` (12) | 31 508 | 2026-10-13 | Switched off at runtime (`LD_ANALYSERS = false` in `avvio.js`, loaded by built name `"static/an-" + n + ".js"`), still smoke-tested by `tests/test_analizzatori.py`. Dead code for the user while the flag stays off: owner's call. |
| `mzlab/web/esempi/` (16) | 68 969 012 | 2026-10-10 | Example files (never removable); `tools/golden.py` globs `HRMS_*.mzML`. |
| `tests/golden/esempio_*.json`, `sintetico_*.json` (12) | 1 870 440 | 2026-10-10 | `tools/golden.py` + `tests/test_golden.py`, `crates/mzlab-core/tests/golden.rs` (`format!("{name}.json")`). |
| `tests/golden/ref/`, `tests/golden/feat/` (28) | 9 073 825 | 2026-10-11 | Parity tests of `mzlab-core`/`mzlab-feat`, `tools/create_golden_ref.py`. |
| `tests/golden/librerie/massbank_unito/` (61) | 186 132 | 2026-10-11 | `tools/librerie_pacchetti.py`, `hashFiles` in `pages.yml`/`release.yml`, `LICENZE-TERZI.md`. |
| `tests/data/xtb/` (102) | 1 072 831 | 2026-10-16 | `crates/mzlab-chem/tests/xtb_671.rs` (`"{name}{}_fukui.csv"`). |
| `crates/mzlab-llm/tests/data/` (52) | 40 355 | 2026-10-21 | Adapter tests read the folder by provider/case name. |
| `tests/data/llm/` (27) | 16 169 | 2026-10-21 | `llm_eval_tests.rs`, `llm_replay_tests.rs` (`llm eval CASES_DIR`). |
| `normative/lists/` (17) | 1 735 954 | 2026-10-30 | `mzlab-cli normative pack`, `hashFiles('normative/lists/**')`. |
| `fuzz/regressioni/` (3) | 89 557 | 2026-10-31 | `crates/mzlab-lettori/tests/fuzz_readers.rs` reads the folder. |
| `vendor/opentfraw/` (33) | 509 320 | 2026-10-12 | Path dependency of `mzlab-core` feature `thermo`; all its modules are reached. |
| `crates/mzlab-desktop/icons/` (6) | 394 257 | 2026-10-10 | Tauri `icon` list in `tauri.conf.json`. |
| `docs/studi/assistente-dati/prompt/` (11) | 51 467 | 2026-10-22 | `crates/mzlab-llm/tests/assist.rs` reads the folder (prompts are exact copies). |
| `mzlab/web/lang/{it,en,contesto}/` (126) | 892 860 | 2026-10-24 | Loaded by the list in `lang/it.js`/`lang/en.js`; context read by `tools/controlla_i18n.py` and `tools/lang_catalog.py`. |
| `tools/moduli/*.toml` (32) | 8 369 | 2026-11-14 | `tools/build_feat_wasm.sh`, `tools/build_lettori_wasm.sh`, `tools/ci_wasm_dimensioni.py`, `tests/test_moduli_dati.py`. |
| `proxy/test/*.test.mjs` (5) | 24 456 | 2026-11-03 | `proxy/test/all.mjs` imports every `*.test.mjs`. |

## Duplicates (identical blobs)
All intentional; no action:
- `tests/data/untargeted/mzmine39/` and `mzmine410/` share `adap.csv`, `adap_asse.csv`, `adap_scansioni.csv`, `isotopes.csv`: two mzmine versions with equal results, each read by its own `_410` test.
- `tests_e2e/fixtures/msngrafo_esempio.json` = `msngrafo_esempio_vista.json` (each name read by its own e2e); `tests/msn_figura/testi-albero.txt` = `testi-radiale.txt` (two golden outputs).
- Three `*_truncated.events` and two `*_text.events` in `crates/mzlab-llm/tests/data` (same normalised events per provider); `rust-toolchain.toml` = `tools/banco_wasm/rust-toolchain.toml` (separate workspace); two empty `__init__.py`.

## Checked and clean
- i18n: `python3 tools/controlla_i18n.py` → OK; it already fails on an orphan key, so there are no unused keys.
- Every `docs/agenti/*.md` has its INDICE row (`tools/gate_release.py` enforces it); backticked paths that no longer exist in those docs are run outputs (`corsa.json`, `plan.json`, …), not removed features.
- All 31 crates are workspace members with a dependent or a `tools/moduli/*.toml`; no dead `[features]` key; `TP_Mine/{js,files,lazy}` and `flow/{js,lazy}` are globbed by `build_site.py`; all `tests/test_*.py` are collected and all `tests_e2e/e2e*.py` run (`NOT_TESTS = {"lib", "synth"}`).
- Python reference modules in `mzlab/`, helpers in `tests/` and `tests_e2e/`: all imported by a test or a tool.
