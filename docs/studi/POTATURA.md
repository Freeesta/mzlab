# POTATURA: pruning candidates (read-only study)

Goal (Federico, 3 Nov 2026): keep mzLab current and aimed at the future, and drop the weight of extreme backward compatibility. mzXML stays input-only, and no mzXML writer will be built. This file lists the candidates with their evidence, so that each cut can be approved on its own. Nothing has been deleted.

Base: `origin/main` at `cb45d8f`. Line counts are `wc -l`. Wasm bytes are measured after `wasm-opt`, from `tools/build_feat_wasm.sh`, `tools/build_lettori_wasm.sh` and `docs/agenti/wasm-tetti.md`. "Tests" counts the test files that mention the item.

**Verdicts**
- **KEEP**: do not cut.
- **CUT NOW**: nothing user-visible depends on it.
- **CUT AFTER <condition>**: cut once the condition is met.

**FREEZE** marks anything that touches Flow (`crates/mzlab-core/src/flow/`, `flow_*.rs`, `steps/`, `mzflow`, `workflow.js`, `flow/`) or `crates/mzlab-lib`. These stay untouched until the Po-river untargeted study delivers its packages.

## 0. Things that look like legacy but are not

Do not cut these:
- **`crates/mzlab-feat/src/legacy/`** (78 files, 15 747 lines). Despite the name, this is the live mzFinder API layer. The desktop app (`feat_finder.rs`, 33 references), the wasm (`finder.rs`, `finder_edge.rs`, `msn.rs`) and `mzlab-finder-lr-wasm` all use it, 181 `legacy::` uses in total. **KEEP**. A rename (for example `finder_api/`) would remove the false signal. That rename is a mechanical PR, but it touches the `sortc` guard (`tests/test_guardie_regole.py` exempts `legacy/`).
- **mzXML reader** (`crates/mzlab-lettori/src/aperti/mzxml*.rs`, 1 142 lines, module `aperti`, cap 561 152 B shared with Numpress).
  - mzXML is 11.9 % of MetaboLights studies (377), the third most common format after `.raw` (42.1 %) and mzML (27.1 %) (`docs/studi/ROSINA.md:107-118`).
  - 2.x and 3.x share one code path (`mzxml_indice.rs:1-2`), so there is no variant code to prune.
  - **KEEP** as input-only. No writer exists outside the test fixture encoder `crates/mzlab-lettori/tests/comune/mzxml.rs`.
- **ANDI / netCDF-3** (`crates/mzlab-lettori/src/andi/`, 1 323 lines, module `andi`, cap 344 064 B, 11 crate test files, 3 e2e).
  - It covers 6.6 % of studies (210), and it is the only route for JEOL, LECO and PerkinElmer (`docs/studi/formati-matrice.md:39`).
  - **KEEP**. Possible later trim: ANDI-Chromatography (`andi/cromato.rs`, 99 lines) is tested only on synthetic data (`lettori-andi.md`). CUT AFTER a year with no real file reported.
- **Formats recognised but not read** (`mzlab/web/formati-noti.js:13-21`: `.wiff2`, `.baf`, `.yep`, `.lcdx`, LECO, mzData) cost only one line each and tell the user why a file is refused. **KEEP**.
- **Dead i18n keys**: none. `tools/controlla_i18n.py` reports 0 orphans among 4 542 keys. An independent pass found 676 keys that are reached only through dynamic prefixes (`cmd.`, `el.`, `tour.` and others), and every dynamic segment exists in the code. The check could be stricter: it misses `"x." + y` concatenation and does not scan `crates/` or `tests_e2e/`. That is a tooling PR, not a cut.
- **Python reference** (`mzlab/*.py` 5 093 lines, `tools/*_riferimento.py` 51 scripts). These are the parity oracles that CLAUDE.md requires, so **KEEP** them, apart from items D3 and D4 below.

## A. Storage migration from the old name "qqq" (largest legacy block)

| # | Location | What it is | Size / tests | Risk / user effect | Verdict |
|---|---|---|---|---|---|
| A1 | `mzlab/web/nomi.js:177-235` | `hookSetItem` / `hookGetItem` / `hookRemoveItem` patch `Storage.prototype` globally. Every `localStorage` write is mirrored under both `qqq.*` and `mzlab.*`, which doubles the writes and recreates the old keys. | ~60 lines; part of `tests/test_migrazione_qqq.py` (490 lines, 9 tests) | Low. Nobody should notice, and every storage call gets cheaper. | **CUT NOW** |
| A2 | `mzlab/web/nomi.js:91-175` | `migrateLs` / `migrateSs` copy `qqq.*` keys once, then purge them on the third start (`mzlab.avvii`). | ~85 lines; same test file | A user who has not opened the site since the rename loses their preferences (theme, language, tour seen). | **CUT AFTER** the rename is older than one release cycle (Federico to confirm the date) |
| A3 | `mzlab/web/qqq-db.js` (222 lines), `nomi.js:8` `DB_OLD`, `browser.js:45` | Copies IndexedDB `qqq_lab` → `mzlab_lab` store by store and deletes the old DB. `migrateIdb` runs on every open (`qqq-db.js:213-214`). | ~170 lines of migration; imported by 10 files (the open helper stays, renamed) | The same users as A2 lose their old notebooks. Removing it also saves an IndexedDB open at every start. | **CUT AFTER** A2's condition, in the same PR |
| A4 | 28 `typeof NOMI !== "undefined" ? NOMI.KEY(x) : "qqq.x"` fallbacks in 17 files (`draw.js:252,541,542,651,804`, `liste.js:260-279`, `tour.js:57,780`, `fair.js:100`, `livelli.js:13`, `TP_Mine/js/19,22,34,42,45,52_*.js`) | Fallback key names for when `nomi.js` is not loaded | ~28 lines | None. `nomi.js` is always loaded first. | **CUT NOW** |
| A5 | Explicit old-key reads: `teoria/tema.js:13,25,29,77`, `teoria/teoria.js:48-50,80`, `teoria/pratica/motore.js:9`, `reset.js:14-20,50,73,88,114-118`, `sw.js:120` (deletes `qqq-` caches), `esporta-dati.js:17-19,52` | Reads and cleanups of `qqq.*` keys, `qqq-big-` caches and the `qqq-theme` message | ~30 lines | The same users as A2 | **CUT AFTER** A2 |
| A6 | `browser.js:8` `QQQ_BROWSER`, `telefono.js:19` `QQQ_PHONE`, plus `QQQ_LOADFAIL`, `QQQ_DATI`, `QQQ_NET`, `QQQ_MZML` | Global aliases kept "for backward compatibility": 11 + 21 + 7 + 5 + 3 + 3 references | Mechanical rename to `MZLAB_*` | None | **CUT NOW** (rename) |
| A7 | `logo-piccone.js:10,89` `LEGACY_KEY`, `settings.js:114`, `teoria/teoria.js:48` | Old admin flag `teoria.esteso` (and its `qqq.` copy) | ~6 lines | An admin who never re-unlocked must click the logo five times again | **CUT NOW** |

Do **not** touch `teoria/07-qqq.html` or the glossary «QqQ» entries: there "QqQ" means the instrument, not the storage keys.

## B. Code paths that cannot run any more

| # | Location | What it is | Size / tests | Risk / user effect | Verdict |
|---|---|---|---|---|---|
| B1 | `reset.js:59` (`fetch("api/new")`), `avvio-progressivo.js:25`, `shell.js:326,338` (`QQQ_NET` upload progress) | `!window.QQQ_BROWSER` branches for the retired Python server. `browser.js` is always injected, by `tools/build_site.py:240` for the site and `crates/mzlab-desktop/src/update.rs:33` for the app. | ~20 lines | None. First check that no e2e serves raw `mzlab/web` without `browser.js` (`tests_e2e/servi.py` serves `site/`). | **CUT NOW** |
| B2 | `?motore=rust` and `&lr=1` | Query switches that no JS reads. They survive in comments (`rust-worker.js:2`, `peaks-worker.js:3`, `shell.js:248`) and in the URLs of 19 `tests_e2e/` files. | Comments plus URL strings | None | **CUT NOW** |
| B3 | `crates/mzlab-feat/src/attributio/campaign.rs:216-240` `HISTORICAL_NAMES` | Old Attributio `compat.py` name table. It has 0 references, tests included. | 25 lines | None | **CUT NOW** |
| B4 | `crates/mzlab-lib/src/read.rs:650-653` `#[allow(dead_code)] fn _unused` | Dead function | 3 lines | None | **CUT AFTER** the freeze (in `mzlab-lib`) |
| B5 | `rust-budget.js:14-15,62,111,129` `MANUAL_ENABLED = false` | The manual memory-budget choice. It has no UI, and its stored value is cleared at every start. | ~15 lines | None | **CUT NOW** |
| B6 | `capacita.js:25-30` `MZLAB_NO_SIMD` | The older test switch beside `MZLAB_CAP_OFF` / `?cap-off=`, which do the same job | ~4 lines | None (tests use `cap-off`) | **CUT NOW** |
| B7 | `crates/mzlab-feat/src/legacy/lrisf/isf_report.rs` (251), `isf_split.rs` (91), `isf_text.rs` (116) | ISF report path reachable only from `crates/mzlab-feat/tests/legacy_lrisf.rs` and its golden `tests/golden/finder_lrisf.json` (896 KB) | Live: `isf_eval` calls `classify_report` (`lr/isf.rs`), shipped in desktop `feat_finder.rs` and `mzlab-finder-lr-wasm`. `legacy_lrisf.rs` also holds the tree/compare parity tests that read `finder_lrisf.json`. Only the DP ramp (`Ramp`) is test-only, about 40 lines. | None: nothing is cut. | **KEEP** |
| B8 | `tools/build_feat_wasm.sh:28-37,80-99` `MZLAB_WASM_SEPARATI` | The separate-module build "as before", with six HR modules and four Attributio modules instead of `hr` and `attributio-all`. It needs the per-module caps `limit_rev`, `limit_export`, `limit_ms2`, the `rev`/`export`/`ms2` split in `mzlab-attributio-wasm` and the same-URL dedup in `TP_Mine/files/feat-worker.js:66`. | ~30 lines of script, 3 caps, docs in `wasm-tetti.md:8` and `INDICE.md:74`. No workflow or test sets it. | None. The shipped bytes do not change. | **CUT NOW** (no `.github/` edit needed) |
| B9 | `mzlab/web/browser.js:5` and `composizione.js:3` | Stale comments ("the local program does not load it"; "server: mzlab/chem/composition.py") | 2 lines | None | **CUT NOW** (with B1) |

## C. Old saved-file and settings formats

| # | Location | What it is | Size / tests | Risk / user effect | Verdict |
|---|---|---|---|---|---|
| C1 | `mzlab/web/libreria-worker.js:17,236-241,592-617,640-663` | The MZL1 library archive format, the `migrateOne`/`migrateAll` conversion to `.mzlib`, and the merge of "legacy" rows | ~60 lines; `tests/test_libreria.py` | A user with a library imported before `.mzlib` would see it disappear instead of being converted | **CUT AFTER** a release note says "re-import libraries older than X" |
| C2 | `libreria-worker.js:20-245` and `addLibraryJs`/`runSearchJs`/`runAnalogsJs`/`runAllJs` (`:320-400`) | The pure-JS library engine, which duplicates `crates/mzlab-lib` (`scores.rs:4,7`: "ported from the former JS search"). It is reached only by `?libjs=1` (`libreria.js:11`) and by the fallback after two worker crashes in a minute (`libreria.js:32`). Browsers without wasm are already refused at `browser.js:12`. | ~400 lines; `tests/test_libreria.py` (207 lines, runs the JS in node) | Medium. When the wasm worker crashes, the user gets an error instead of a slower search. It also breaks the rule "all logic in Rust". | **CUT AFTER** the wasm-lib crash rate is known to be ~0 (`stat-ci` / bug reports), and with C1 |
| C3 | `libreria-worker.js:246-260,440-452` (IndexedDB store and `memFile`, keyed on `probeOpfs()`) | The no-OPFS library store. Its comment ("Safari / old browsers: no synchronous handles") has been stale since Safari 15.2. | ~50 lines | Private windows may still refuse OPFS (not checked on current Firefox and Safari) | **CUT AFTER** an e2e check in Firefox and WebKit private windows |
| C4 | `rust-worker.js:218-224,307` `lrMigrate` and `TP_Mine/js/19_tpmine_base.js:163-170` `idbFiles` | Moves the IndexedDB "files" store to OPFS on every non-fresh `lr_new` | ~20 lines; `tests/test_migra.py` (48 lines) | Files stored in IndexedDB before OPFS would not reopen | **CUT AFTER** A2's condition. Keep the `migra.js` step framework. |
| C5 | `explore.js:235-279` (`NB_V = 2`, `CDOC.isOld`/`CDOC.migrate`), `campioni-doc.js:51-58`, `lista-file.js:183-188` `migrateBrowse` | Old notebook v1 → v2 | ~30 lines JS | An old notebook opens without its sample roles | **CUT AFTER** C6 (same data) |
| C6 | `crates/mzlab-core/src/campioni/json.rs:229-~330` `da_legacy` (`legacy_kind` at :157 stays: `modello.rs:127` and `csv.rs:161` use it) | Migrates notebook v1, QA/QC roles, `campioni/1` and `.mzflow` roles to `campioni/2`. Called from `mod.rs:255`, `mzlab-wasm/src/campioni.rs`, `mzlab-feat-table-wasm/src/core.rs`, `campioni-doc.js:52`, `workflow.js:146`, `TP_Mine/files/finder-ruoli.js:37`. | ~100 lines Rust; `crates/mzlab-core/tests/campioni.rs` (4 uses) | Old notebooks and `.mzflow` files lose their roles | **CUT AFTER** the freeze (**FREEZE**: `.mzflow` input) |
| C7 | `mzlab/web/workflow.js:17` `LEGACY_FORMATS = ["mzworkflow/1"]` | The old name of the workflow file | 1 line + check | An `.mzworkflow` file from before the rename stops opening | **CUT AFTER** the freeze (**FREEZE**) |
| C8 | `crates/mzlab-core/src/flow/parse.rs:128,143-147` | Reads the Italian keys of `mzflow/1` (`formato`, `programma`, `operazioni`, …). `workflow.js:16` still **writes** `mzflow/1`, so this is not legacy yet. | ~6 lines | Not applicable | **KEEP**. Revisit when `mzflow/2` is the only format written (**FREEZE**). |
| C9 | `pannelli.js:122-132` `PT_OLD` (Italian panel titles → `@codes`), `pannelli.js:242,678` (`yz`, `bk`), `fondo.js:72-83` `migra`/`clipOld` | Old notebook panel fields | ~30 lines | An old notebook shows raw titles or the default baseline zone | **CUT AFTER** C5 |
| C10 | `livelli.js:14-27` `LEGACY` (`ent`, `shared`, `ppm`, `ent2`) | The first flat threshold keys | ~14 lines | Old thresholds are reset to the defaults | **CUT NOW** |
| C11 | `picchi.js:22` | Drops the old `method` field from saved settings | 1 line | None | **CUT NOW** |
| C12 | `stato.js:11` `VIEW_ALIAS` `attributio` → `flow` | Old URL hash | 1 line | Old bookmarks open the default view | **CUT NOW** |
| C13 | `libreria-scarica.js:24`, `crates/mzlab-desktop/src/scarica.rs:108-116`, `tools/librerie_pacchetti.py:12,207` (`esterne` kept "for readers of librerie/1") | Accepts the `librerie/1` download manifest. Only `librerie/2` is published (`librerie_pacchetti.py:43`). | ~20 lines over 3 places; `tests/test_libreria_scarica.py:101-113`, `crates/mzlab-desktop/tests/scarica.rs` | An app older than `librerie/2` can no longer download libraries | **CUT AFTER** the oldest supported desktop release reads `librerie/2` |
| C14 | `crates/mzlab-feat/src/attributio/readers/feature_decisions.rs:80-93` `LEGACY_COLUMNS`, `attributio/tox/snapshot.rs:63` | Italian column names read but never written | ~20 lines | Old campaign decision files lose those columns | **CUT AFTER** Federico confirms no campaign files from before the rename are still in use (`Freeesta/mzlab-dati`) |
| C15 | `crates/mzlab-lib/src/format.rs` `MIN_READER = 1`, `read.rs:116,328,338` | `.mzlib` v1 is still readable | Small | An old `.mzlib` would need rebuilding | **CUT AFTER** the freeze (**FREEZE**, `mzlab-lib`) |

## D. Duplicate code paths and engine variants

| # | Location | What it is | Size / tests | Risk / user effect | Verdict |
|---|---|---|---|---|---|
| D1 | `crates/mzlab-feat/src/untargeted/` (admin-only part: `adap`, `resolve`, `join`, `gapfill`, `isotopes`, `ident`, `pipeline`, `pipeline_stream`, `localmax`, `smooth`, `arpls`, `to_attributio`, `table`; ~3 000 of the 5 190 lines). Wasm entry `mzlab-feat-wasm/src/untargeted.rs` (114) and `lib.rs:502-533`; desktop `feat_untargeted.rs` (70); UI `TP_Mine/js/32_pipeline.js:84-109,198`. | The mzmine-3.9.0-like engine, which only admin mode can choose in the Pipeline panel | ~3 300 lines Rust; wasm `untargeted` 465 469 B (cap 585 728) disappears from the site; 26 test files `crates/mzlab-feat/tests/untargeted_*.rs` (4 144 lines) + 3 more + `tools/valida/cancello_untargeted.py`; Python refs `arpls`, `localmax` | It is a product choice. **`formula_id.rs` (453) and `isofinder` (1 695) must stay**, because the Flow step `formula.ms1`, `36_feature_qc.js` and the `formula` wasm use them. `broad.rs` and `aligned.rs:49` are shared. | **CUT AFTER** the Po-river study states whether it compares against the mzmine-like engine (**FREEZE**: entangled with `formula.ms1`) |
| D2 | `crates/mzlab-feat/src/chem/grafo.rs` (440 lines, SMILES `Mol::parse` + `from_molfile`) beside the `mzlab-mol` crate (`smiles.rs`, `molfile.rs`, `molfile_v2000.rs`) | Two molecule readers. `tests/grafo_record.rs` calls grafo "the legacy reader" and checks parity with `mzlab-mol` over 625 compounds. | `grafo::Mol` is used in ~50 files (`legacy/fragtree/mol.rs`, `mzlab-chem/src/reatt.rs:177`); `rtlogp.rs:204` already uses `mzlab-mol` | High: a large refactor. The parity test makes it safe. Wasm saving not measured (grafo is in `grafo`, `hr`, `msngrafo`, `msnregole`, `chem`). | **CUT AFTER** a measured plan (twiggy on `hr` and `msngrafo`), after the freeze |
| D3 | `tools/insilico_riferimento.py` (38), `rearrange_riferimento.py` (89), `rtpredict_riferimento.py` (51), `cc_riferimento.py` (102) | One-shot generators of frozen golden JSON. No test runs them; they need `ATTRIBUTIO_SRC`, rdkit or sklearn. | 280 lines; mentioned in `docs/agenti/attributio-ms2.md:21-30` | The golden files can no longer be regenerated (they are frozen anyway) | **CUT AFTER** Federico agrees the goldens are final |
| D4 | `tools/centroidi_nucleo.py` beside `centroidi_riferimento.py` (it imports the latter) | A possible overlap | Not measured | Low | Check, then **CUT NOW** if it duplicates |
| D5 | `crates/mzlab-feat/src/samples.rs` (221 lines, `campioni/1`) beside `campioni/2` and `/3` in `mzlab-core` (`massacc/mod.rs:249` reads all three) | Two models of the same sample metadata. 40+ users in `flow_quant`, `flow_stats`, `report`, `filters` and others. | Large | High | **CUT AFTER** the freeze (**FREEZE**: Flow steps). It is a consolidation, not a prune. |
| D6 | `crates/mzlab-feat/src/liste/mod.rs:103` `from_legacy_text` ↔ `legacy/transf.rs:167` | A circular call (each calls the other). It is still the mzFinder input format. | Small | None | **KEEP** the format; untangle the cycle in the `legacy/` rename PR |
| D7 | `campioni-doc.js:33` `number()` | Re-implements the Rust replicate count in JS ("counted as the Rust does") | ~15 lines | Low–medium. It drifts if the Rust changes. | **CUT AFTER** a route returns the count (part of the consolidation, `docs/studi/DUPLICAZIONI_CODICE.md`) |

## E. Browser fallbacks and the support floor

Documented floor: Chrome/Edge 91, Firefox 89, Safari 16.4 (`docs/agenti/wasm-potenziale.md:116`). The real floor is already higher:
- The engine requires `simd128`, `opfs` and `opfsSync` (`capacita.js:11`). Sync access handles need Chrome 102, Firefox 111 and Safari 15.2.
- `DecompressionStream` (`normative-scarica.js:57`, `insilico-scarica.js:97`) needs Firefox 113.
- CSS `:has()` (`app.css:47,62,329,514` and others) needs Chrome 105 and Firefox 121. It only fails cosmetically.

**Proposed floor: Chrome/Edge 111, Firefox 121, Safari 16.4** (all from 2023). This writes down what the code already needs. Under it, these become dead:

| # | Location | Feature (universal since) | Lines | Verdict |
|---|---|---|---|---|
| E1 | `docs/agenti/wasm-potenziale.md:116` | The written floor itself. `rust-mzidx.md:15` names a `simdOk()` that does not exist; the SIMD check is in `capacita.js:37` and `rust-bridge.js:100`. | doc | **CUT NOW**: rewrite both lines and state the floor in one place |
| E2 | `capacita.js:7,42`, part of `rust-worker.js:33` | The `offscreen2d` and `locks` probes. No `REQUISITI` entry uses them and nothing reads them. | ~4 | **CUT NOW** |
| E3 | `capacita.js:40` | The `webgl2` probe (`typeof WebGL2RenderingContext`). It is always true and cannot see a GPU blocklist. | 1 | **CUT NOW**, or replace it with a real context test |
| E4 | `migra.js:33-34`, `qqq-db.js:198-199`, `archivio.js:32-33` | No-Web-Locks branches (Chrome 69, Firefox 96, Safari 15.4) | ~6 | **CUT NOW** after E1 |
| E5 | `shell.js:53`, `mappa3d.js:694`, `frame.js:7`, `barra.js:246,1064` | ResizeObserver guards (Safari 13.1) | 5 | **CUT NOW** after E1 |
| E6 | `libreria-finestra.js:267`, `teoria/sim-quad.js:143`, `teoria/sim-qqq.js:120`, `teoria/ei-spettri.js:93` | IntersectionObserver guards (Safari 12.1) | ~8 | **CUT NOW** after E1 |
| E7 | `transizioni.js:68` (MutationObserver), `i18n.js:134` ("old browser" try/catch) | One-liners | 2 | **CUT NOW** after E1. Keep `mirino.js:20` (the rAF guard serves the node tests). |
| E8 | `export-pezzi.js:16,77` | An in-memory blob instead of OPFS. It is dead, because without OPFS the page refuses to start the engine (`browser.js:84`). | ~10 | **CUT AFTER** the C3 private-window check |

**KEEP**:
- `requestIdleCallback` guards (Safari has none): `rust-budget.js:50`, `shell.js:416`, `mappa.js:37,315` and others.
- `showDirectoryPicker`, which is Chromium-only (`capacita.js:39`, `cartella-puntata.js`, `libreria*.js`).
- `webkitdirectory`, which is the main folder path.
- Safari's missing long tasks (`perf.js:73-77`) and its gesture events (`infusione-grafo-gesti.js:24`).
- The `idxpool.js` two-worker limit.
- The isolation reload in `browser.js:13-27`, which depends on headers rather than on the browser version.

**Feature flags**:
- **KEEP**, as test and support switches: `?libjs=1` (it goes with C2), `?lettori=modulo`, `?cap-off=`, `?telefono=`, `?perf`, `?reset`.
- `features.js` (20 lines) holds one switch, `qaqc` (`?qaqc=` / `mzlab.dev.qaqc`), used by 4 files. **KEEP** while QA/QC stays dormant, or **CUT AFTER** Federico decides whether QA/QC ships.

## F. Stale docs and studies

| # | Location | What it is | Size | Verdict |
|---|---|---|---|---|
| F1 | `docs/studi/gemini-*verifica*.md` (11 files) | Phase-1 read-only verification reports. Nothing cites them. | ~145 KB | **CUT NOW**: move to `docs/archivio/studi/` (the archive convention exists, `docs/archivio/INDICE.md`) |
| F2 | `docs/studi/LANDING_AREA_LAVORO.md`, `docs/studi/prompt_landing/` | Option C, superseded by D. Its own LEGGIMI says "do not run". | 46 KB | **CUT NOW** → archive |
| F3 | `docs/studi/ci_veloce/` | Six `.yml` drafts that differ from `.github/workflows` by 100-575 diff lines. The live doc is `docs/agenti/ci-veloce.md`. | 54 KB | **CUT NOW** → archive |
| F4 | Finished studies with 0 references: `TABUI_FEATURE_TABLE.md`, `LISTE_NORMATIVE_C2B.md`, `MOTORE_STAT_MOLECULES_FLOW.md`, `WASM_PACCHETTI.md`, `WASM_ULTIME_FUNZIONI.md`, `apertura-fluida.md`, `prompt_neg_lr/`, `prompt_velocita/`, `prompt_gruppi/`, `prompt_landing_d/`, `flow_grafica/` (prototype) | The work is in the code and described in `docs/agenti/` | ~250 KB | **CUT NOW** → archive, in batches |
| F5 | Finished studies that are still cited (`ATTRIBUTIO.md`, `CC_MOTORE_NEL_BROWSER.md`, `FLOW_GRAFICA.md`, `WORKFLOW_MODULARI.md`, `workflow/`, `STRUTTURA_DATO*.md`, `TABUI_*`, `regole-frammentazione-*.md`, `OTTIMIZZAZIONE_CORE.md` with obsolete numbers, and others) | Historical rationale cited from `docs/agenti/` | — | **CUT AFTER** the citing lines in `docs/agenti/` point to the archive path |
| F6 | `docs/agenti/match-spettrale.md:28,101` and `INDICE.md:26` (`table.attribute@1`, `flow_attribute.rs`, `attrib/1`) | These describe a step that does not exist in `crates/` | 3 lines | **CUT NOW**: rewrite as current state (**FREEZE**-adjacent, docs only) |
| F7 | `docs/agenti/librerie.md:4,8,9` (`flow_match_filter.rs`, `flow_match_evidence.rs` → `crates/mzlab-lib/src/flowmatch/{filter,evidence}.rs`), `attributio-export.md` (`sha1.rs` → `mzlab-core/src/metodo.rs:283`), `dati-3200.md:25` (`isotab.rs` → `isotable.rs`) | Stale file names | 5 lines | **CUT NOW** (fix) |
| F8 | `docs/agenti/wasm-tetti.md:12`, `lettori-modulo.md` («Costruzione e tetti»), `lettori-andi.md`, `lettori-aperti.md` | Old `-Oz` caps that disagree with `tools/build_lettori_wasm.sh:9-17` (for example, aperti 416 768 against 561 152) | ~5 lines | **CUT NOW**: keep only `wasm-tetti.md:19` |
| F9 | Active studies: `ROSINA.md`, `rosina/`, `PROMPT_ROSINA/`, `LETTORE_THERMO_PERFETTO.md`, `PROMPT_LETTORE_THERMO/`, `PROMPT_RAW_VELOCE/`, `RAW_VELOCE.md`, `DUPLICAZIONI_CODICE.md`, `prompt_consolidamento/`, `flow-semplice.md`, `parita-browser.md`, `assistente-dati/`, `PROMPT_TABUI_MS2_LR/` | Work still open | — | **KEEP** |

## G. Golden and parity fixtures that protect only removed behaviour

- `tests/golden/finder_lrisf.json` (896 KB): it stays (B7 is KEEP).
- `tests/test_migrazione_qqq.py` (490 lines): it shrinks with A1 and A4–A7, and goes entirely with A2–A3.
- `tests/test_libreria.py` (207 lines): it goes with C1 and C2 (its node run of the JS engine).
- The untargeted goldens and tests (`crates/mzlab-feat/tests/untargeted_*.rs`, `tests/data/untargeted/`, `tools/valida/cancello_untargeted.py`, Python refs `arpls`, `localmax`): they go with D1. The `formula_id` and `isofinder` refs stay.
- `tests/test_libreria_scarica.py:101-113` and the `librerie/1` cases in `crates/mzlab-desktop/tests/scarica.rs`: they go with C13.
- I found no other golden that protects removed behaviour. The `tests/golden/esempio_*` and `sintetico_*` files guard live engines.

Removing a test together with the code it guards is a cut, not skipping a test. Each PR below names the tests it removes.

## Proposed order: small independent PRs

Each PR runs `python3 tools/pre_pr.py` plus the named check, and touches no `.github/`.

1. **Dead bits (Rust + JS)**: B3, B5, B6, B2 (comments only; the e2e URLs lose `&motore=rust`/`&lr=1`), C11, C12. Check: `cargo test -p mzlab-feat`, `python3 tools/verifica.py --cambiati`.
2. **Separate wasm build**: B8. Check: `tools/build_feat_wasm.sh site/static` gives the same `manifest.json` SHA-256 as before; `wasm-tetti.md` and `INDICE.md:74` rewritten.
3. **Storage hooks and fallbacks**: A1, A4, A6, A7, B1, B9. Check: the trimmed `tests/test_migrazione_qqq.py` still proves the one-time copy; `e2e_tour.py` and a full `verifica.py --cambiati` are green; `grep -rn 'QQQ_' mzlab/web TP_Mine` is empty apart from `nomi.js`.
4. **Browser floor**: E1–E7. Check: the floor is written once (`docs/agenti/wasm-potenziale.md`) and `browser.js` names it in `load.err.oldBrowser`; e2e green in Chromium, Firefox and WebKit (`--completo`).
5. **Docs archive**: F1–F4, F6–F8. Check: `python3 tools/controlla.py` and `tools/controlla_link_docs.py` are green, and `grep -rn 'docs/studi/<moved>'` is empty.
6. **Old settings in JS**: C10. Check: `verifica.py --cambiati`.
7. *(after A2's condition)* **Drop the qqq migration**: A2, A3, A5, C4. Check: `test_migrazione_qqq.py` removed, `tests/test_migra.py` green, `qqq-db.js` gone or renamed, and the IndexedDB open count at start is 1 (`?perf`).
9. *(after the C3 check)* **No-OPFS library store**: C3, E8. Check: new e2e in Firefox and WebKit private windows.
10. *(after the C1/C2 conditions)* **JS library engine and MZL1**: C1, C2, `?libjs=1`. Check: `e2e_libreria_wasm.py` green; `test_libreria.py` removed; `libreria-worker.js` under 300 lines.
11. *(after the freeze)* **Flow and mzlab-lib items**: B4, C5–C7, C9, C15. **FREEZE**. Check: `cargo test -p mzlab-core -p mzlab-lib`, and an `.mzflow` round trip in `e2e_flow_*`.
12. *(after the freeze plus Federico's decision)* **The mzmine-like engine**: D1. **FREEZE**. Check: the `untargeted` wasm is gone from `manifest.json`; `formula.ms1` parity tests (`flow_formula_ms1`) stay green; `hr` bytes do not change.
13. *(after the freeze, measured plan)* **Consolidations**: D2, D5, D7, the `legacy/` → `finder_api/` rename, and C13, C14, D3 once their conditions are met.

**Expected total** (all cuts):
- Rust: about 4 000 lines.
- JS: about 1 000 lines.
- Tests: about 5 400 lines and ~1 MB of golden data.
- Docs: about 500 KB moved to the archive.
- Wasm: one lazy module (`untargeted`, 465 KB) leaves the site.

The size of every module that always loads stays the same, except for any D2 saving, which has not been measured.
