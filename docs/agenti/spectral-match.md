# Spectral matching against libraries: a single engine for mzFlow and the side panel

State of the spectral matching engine and open work (§12); every choice carries its why; open decisions are at the bottom (§13).

## 1. What exists (measured)
- **`.mzlib` v1 format** (`crates/mzlab-lib`, ~5000 lines, no `unsafe`): 32 B spectrum records (precursor, polarity, `u8` license, optional `f32` RT), m/z peaks quantized to 1e-5 Da (`u32`) and `u16` intensities in blocks of 512 zstd-compressed spectra, metadata (10 fields: name, adduct, CE, instrument, formula, InChIKey, SMILES, accession, authors, license) in blocks of 128, **Flash indexes on fragments and neutral losses** (10 B entries in 64 KB pages, sorted by m/z).
  Sliced reading with an LRU cache (64 MB, 32 MB on phone/iPad: `read.rs`); the page tells the worker the device in the `config` message (`libreria-corpo.js`, rule of `rust-budget.js` `device()`), because in the worker an iPad with the Mac user agent has no `maxTouchPoints`. Memory-limited construction (`build.rs`, scratch on disk). Flash cleaning (`clean.rs`: precursor − 1.6 Da, noise < 1 %, merge 0.05 Da, ≤ 500 peaks).
- **Parsers** (`parse.rs`): MSP, MGF, MassBank `.txt`, JSON MoNA/GNPS/mzMine, recognized from the content (`detect_format`).
- **Scores** (`scores.rs`, `prep.rs`): spectral entropy (Li 2021), cosine (square root of intensities), forward/reverse, explained intensity, common peaks, common neutral losses, modified cosine (32 peaks), GNPS cosine; per-peak contributions (`contrib.rs`, `score_terms`). Tolerance in Da or ppm (`frag_tol.rs`).
- **Searches** (`search.rs`): `identity` (precursor window, every candidate scored), `flash_scores` (open / neutral losses / hybrid on the index), `analogs` (hybrid in the ±200 Da window, then modified cosine ≥ 4 peaks). Target-decoy FDR (`decoy.rs`).
- **wasm** `crates/mzlab-lib-wasm` (`static/wasm-lib/`, ceiling 614,400 B, loaded only when the libraries are opened; worker `libreria-worker.js`): `Importer`, `Lib.open/identity/best/analogs/verify`.
- **Library folder** (`libreria-cartella.js`): desktop app = native memory-mapped folder; Chrome/Edge = `showDirectoryPicker`; elsewhere OPFS `librerie/`. User sources never modified, index in `.mzlab/<name>.mzlib` + `.idx.json` (size, date, SHA-256), states ready/to index/update/damaged.
- **Catalog** `librerie/2` (`tools/librerie_pacchetti.py` → `site/static/librerie/catalogo.json` + 4 MB pieces with SHA-256; `librerie/1` still accepted): the page downloads ONLY from the site itself (CSP `connect-src 'self'` unchanged); `fonti` (hand-written list `tools/librerie_catalogo.json`, `modo` site|external|link), `esterne` only in the desktop app with the dedicated switch; details in `libraries.md`.
- **Flow**: `spectra.match@1` (algorithms `spectral_entropy` default, `cosine`, `modified_cosine` with `delta_formula_mda`, `tree` for MSⁿ; filters name/adduct/instrument/CE/RT, `decoys`, `matched_peaks`, `score_terms`) and `table.match@1` (the MS² of every row of the `.mzfeat` against a library; output `candidates/1`, query = row id); `annot.level@1` (Schymanski cascade, `livelli-1.json`); `rt.predict` (Crippen/forest, RT direction); `polpair/1` (same neutral mass in the other polarity, evidence and never merging).
- **Page**: "What is it?" (HR, `libreria-finestra.js`: master/detail with Candidates and Analogs tabs, groups by first InChIKey block, 2D structure, mirror with solid/dashed/dotted lines and mouse hover, level from `LIVELLI.assess`); in LR "Search in the libraries…" without levels.
- **Reusable chemistry**: `COMMON_DELTAS` (42 modifications) and `delta_formulas` (`mzlab-feat/src/attributio/review/matching.rs`), `regole_tp_2.json` (49 photocatalysis rules with `delta_formula`, `direzione_rt`), `isoscore.rs` (fine isotope pattern), `kendrick.rs` (KMD), `mzlab-mol/src/descr.rs` (TPSA, Crippen logP), RDBE in `msngraph/candidates.rs`. NOSC: does not exist.
- **Multi-library engine** (`crates/mzlab-lib`): `libset.rs` (up to 16 libraries, one mask per library, shared accumulators, merge by score, sidecar of duplicate groups), `matchres.rs` (`libmatch/1`, `docs/schemi/libmatch-1.json`), `preset.rs` (`data/preset-1.json`), `libinfo.rs` (`SEC_LIBINFO`, hash `inchikey14`), `stein.rs` (`stein_cosine`, `ri_factor`, `hybrid_entropy`), flat accumulators in `flash.rs` (`FlashAcc`). Bench `examples/libset_bench.rs`, measurements in `measurements-rust.md`.
- **Missing**: `LibSet` in the wasm and in the panel, multi-library `table.match`, `table.attribute@1`, parent-constrained ΔFormula, `modifiche-1.json`, property kit (§12).

## 2. Architecture: a single engine
- `mzlab_lib::libset` (set of open libraries + preset) and `mzlab_lib::matchres` (`libmatch/1` result) sit ABOVE `search.rs`. Only the `mzlab-lib` tests use them; they must be called by `table.match`/Attribution node (native, `mzlab-cli`), the side panel (wasm `mzlab-lib-wasm`, export `LibSet.search(query_json) -> libmatch/1`) and the desktop app (same native code). *Why*: grouping, ordering and panel badges still live in `libreria.js`; in Rust they give panel = Flow parity by construction.
- The engine works per **query** `{prec, pol, mz[], it[], ms_level, tech, rt?, ri?, parent?}`; `parent` (parent formula, optional) enables constrained ΔFormula (§7).
- JS stays glue: it opens the files, passes the bytes (existing `read(offset,len)` callbacks), draws. No new Rust dependency.

## 3. Place in mzFlow: "Attribution" module
- **Point in the graph**: for each polarity, `features.table` → `table.match` → **`table.attribute@1`** (to do) → `annot.level` / `tp.link`. There is no step that merges the pos+neg tables: the two tables stay separate by choice (`polpair/1` is evidence). *Why*: the MS² belongs to the row of one polarity; merging pos+neg is a row join that `table.attribute` does by reading `polpair/1` as an optional input (column `other_polarity_hit`).
- **Palette**: "Attribution" is a palette group in the `attributio` package (no new package) with the chain preset `table.match → table.attribute → annot.level`. English only like the whole Flow.
- **Extended `table.match@1`** (to do, not a new step; it has a single library and `spectral_entropy`/`cosine`/`modified_cosine`): port `library` becomes `.many()` (several `.mzlib`/MSP/MGF), new parameters `library_preset` (§9), `search_mode` (`identity` | `identity+analog`), `algorithm` unchanged (`spectral_entropy` default; new `hybrid_entropy`, `stein_cosine`). Output: `candidates/1` + `libmatch` block per query (compound-centric, §6). *Why*: a single search step, already with tests and CLI.
- **`table.attribute@1`** (to do, file `flow_attribute.rs`): inputs `table` (`.mzfeat`), `hits` (`candidates/1`), optional `polpair`, `rt` (output of `rt.predict`), `parent` (structure). Output `attrib/1` (JSON per row, read by Tabui and the Flow views).
  Added columns: `best_name`, `best_inchikey14`, `match_type` (exact | analog | in_silico), `score` (+ `score_kind`), `matched` (n/N), `shifted` (shifted peaks), `reverse`, `explained`, `delta_ppm`, `delta_mz`, `delta_formula` (analogs), `n_compounds`, `library`, `confidence` (computed level, §8), `rt_dir_ok` (if `rt` is there). One parameter per choice: `min_score`, `min_matched`, `top_compounds` (10), `analog_max_delta` (200 Da).
- **CLI**: `--set ISTANZA.library_preset=…`, `--set ISTANZA.search_mode=…` etc. (CLI rule, `docs/agenti/cli.md`).

## 4. MSⁿ side panel
- The "Identification with the libraries" panel (`ident-pannello.js`, tab of the info bar): MS² spectrum of the active cell, loaded libraries with automatic selection of the compatible ones, "Load file…" / "Load folder…", catalog menu (`ident-catalogo.js`), candidates table. The search goes through `LIB.call("search", {ids})` in the worker, not yet through `LibSet`.
- **Behavior to build**: opening an MSⁿ spectrum (n ≥ 2) immediately starts `LibSet.search` on all the active libraries of the preset with the polarity of the spectrum; the panel shows the exact matches first, then the analogs, in < 100 ms perceived (identity) and without blocking (analogs queued, canceled if the spectrum changes: `AbortSignal` in the worker, `cancel(token)` in the engine). No "Search" button to press.
- MS³+: the query uses the spectrum and the precursor of the open level; MSⁿ libraries (MSnLib, `msn-lib/1`) go through `spectra.match` `tree`, outside this engine.
- LR (Principle 1): the search stays ("Search in the libraries…"), but `confidence` is empty and no badge suggests an identity; profile tolerances (0.5 Da / 0.3 Da). HR: instrument profile tolerances (precursor ppm, peaks 10 ppm, minimum 0.005 Da).
- Panel interface: keys in `lang/it.js`/`lang/en.js` (the panel is in the Data view, not in Tabui).

## 5. Library: format, folder, detection, deduplication
- **Format**: `.mzlib` stays (already memory-mapped on the desktop, read in slices in the browser). **v2** only for three additions: (1) `ri` (`f32`, Kovats index) in the record with `FLAG_HAS_RI`; (2) section `SEC_LIBINFO` (short JSON: `kind` experimental/in_silico, `tech` lc_msms/gc_ei/msn, `analyzers` (counters for Orbitrap/QTOF/QqQ/IT/EI), `pol` (counters), `source` (catalog id), `version`, `sha256` of the source); (3) `inchikey14` as `u64` (hash of the first 14 characters) in a per-spectrum section, to group without reading the metadata.
  The v2 reader reads v1 (`MIN_READER` stays 1). *Why*: no mandatory reimport; the information needed by presets and badges is in the file.
- **Automatic detection** (in `parse.rs`/`build.rs`, written in `SEC_LIBINFO`): polarity from `Ion_mode`/`IONMODE`/adduct; technique `gc_ei` if the precursor is missing and `Spectrum_type`/`IONIZATION` = EI or there is `RI`; `in_silico` from known names/comments (LipidBlast, "in-silico", `PREDICTED`) or from the catalog; analyzer from `Instrument_type` (keywords). *Why*: the user does not have to declare anything.
- **Folder**: unchanged (§1). Update = new SHA-256 of the source → "To index"; catalog package with a new `versione` → "Update available". Never merge the downloaded files.
- **Deduplication without merging** (`libset.rs`): the files stay separate, with a `u16` mask (bit = library) and a sidecar (`MZLS`) built once per file combination: for each spectrum of each library the key `(inchikey14, pol, round(prec, 0.001), hash of the 6 highest peaks)`; duplicates get a common `group_id`. The result shows a compound once with the libraries that contain it. *Why*: MassBank, MoNA and GNPS share many spectra; repeating them in the Top 10 takes room from other compounds.
- **User guide**: no paragraph; the panel shows the folder, the button "Choose folder", the list and the catalog menu (§10) with a link to the site of each "link only" library.
- **Memory at 1 million spectra** (browser): in RAM stay `Light` (precursor `u32` + scale `f32` = 8 MB), directory of blocks and pages (< 2 MB), flat accumulators of the open search (`f32` + `u16` = 6 MB), 64 MB LRU cache (32 MB on phone), mask and `group_id` (6 MB). Total ≈ 90 MB: allowed on desktop, borderline on iPad; on phone the panel does not exist (`telefono.js`). On disk: ≈ 320 B per synthetic 22-peak spectrum (320 MB per 1 million); measurement on real MassBank/GNPS still to do.

## 6. Engine, index and wasm measurements
Bench (scratch, not committed): deterministic synthetic library of `identity_bench.rs` (precursors 100–900, 5–40 peaks from a pool of 400 fragments), 200 queries (50 for the open searches), same machine (4 CPU), Node 22 for the wasm (`wasm32-unknown-unknown`, `opt-level 3`, LTO), whole library in memory.

| search (ms per query) | 100k unit | 100k HR | 1M unit native | 1M unit wasm | 1M HR native | 1M HR wasm |
|---|---|---|---|---|---|---|
| `identity` (precursor window, top 10) | 0.13 / wasm 0.31 | 0.013 / wasm 0.03 | 1.4 | 2.5–3.3 | 0.04 | 0.12–0.15 |
| `analogs` (hybrid ±200 Da + mod. cosine) | 3.4 / wasm 5.2 | 2.7 / wasm 4.1 | 45 | 66 | 27 | 35 |
| hybrid open on the whole index (`flash_scores`) | 6.9 / wasm 9.1 | 5.2 / wasm 7.0 | 106 | 140 | 60 | 81 |
| prototype with flat accumulators (same search) | – | – | 21 | 33 | 11 | 17 |

- **Reading**: identity meets the target < 5 ms per spectrum on 1 million even in wasm. The open/hybrid search uses the flat accumulators (`FlashAcc`: `f32` per spectrum + list of touched ones, one-to-one conflicts resolved only for the touched ones with a conflict, `score_pairs` for those): ×2–5 over the pairs version. The synthetic pool is pathological (every fragment appears in thousands of spectra): on real libraries the visited entries are far fewer (MassBank ≈ 100k and GNPS to be measured). Build: 1M spectra in 10–13 s native, 15–17 s wasm.
- **Rules**: (1) flat accumulators with the same result as the pairs version (bit-for-bit parity on the scores); (2) the hybrid limited by the `analog_max_delta` window (open searches on the whole index only on request); (3) analogs queued in the panel (35–66 ms tolerable if they do not block); (4) multi-library (`libset`): each library is searched in turn with the same accumulators, then merge of the tops by score (no global index: *why* libraries are updated one at a time).
- Acceptance goal: identity ≤ 5 ms and analogs ≤ 50 ms per query in wasm on synthetic 1M; real timings in `docs/agenti/measurements-rust.md`.

## 7. Algorithms per technique
| technique | default | why | exists? |
|---|---|---|---|
| LC-MS/MS identity | spectral entropy (Li 2021), Flash cleaning | more robust than cosine on noisy spectra; already `spectral_entropy` | yes |
| analogs / TP | **hybrid entropy** (direct + Δprec-shifted peaks, Flash hybrid) for the candidates, then modified cosine to sort + ΔFormula | the hybrid search is in the index; the modified cosine reproduces GNPS and gives the shifted peaks for the mirror | yes (`analogs`, `hybrid_entropy`) |
| GNPS reproduction | GNPS cosine (`gnps_cosine`) | comparison with GNPS/FBMN results | yes |
| GC-EI | Stein weighted cosine, weights I^a·(m/z)^b with forward and reverse; Gaussian ΔRI (default σ 10 RI units) multiplied into the score | NIST standard; there is no precursor: open search on fragments or window on the highest peak | in the engine (`stein_cosine`, `ri_factor` in `stein.rs`), without interface |
| MSⁿ | `tree` (`msngraph/search.rs`) | trees, not single spectra | yes, outside the engine |

- Stein weights: defaults a = 0.53, b = 1.3 (Kim et al. 2012, Bioinformatics 28:1158), RI σ 10; Stein & Scott 1994 used I^0.6·m/z³. All parameters of `SteinOpts`.
- **Out**: Spec2Vec and MS2DeepScore (models trained at tens of MB, ONNX/TensorFlow inference: new heavy dependencies and no simple parity); to reconsider only as a secondary ordering in the desktop app.

## 8. `libmatch/1` result
- Per query: `query` (row or scan id), `params` (tolerances, algorithm, preset), `compounds[]` sorted, at most `top_compounds` (10). A compound = first InChIKey block (or normalized name if missing): `inchikey14`, `name`, `formula`, `smiles`, `badge` (exact | analog | in_silico), `best` (the best spectrum), `spectra[]` (all the spectra of the compound with score, expandable), `libraries` (mask and names).
- Per spectrum: `library`, `accession`, `score` and `score_kind`, `reverse`, `explained` (explained intensity of the query), `matched` n and `of` N (query peaks), `direct` and `shifted` (direct and shifted pairs), precursor `delta_ppm`, `delta_mz`, for analogs `delta_formula[]` (§9), `adduct`, `ce`, `instrument`, `licence`, `pairs[]` (query_mz, library_mz, kind direct|shifted|loss: the mirror draws from here, already in the `matched_peaks` format).
- `confidence`: level computed by rules (Schymanski/MSI), never decided; `matchres` leaves it null. In the Flow `annot.level` gives it (Rust cascade, `livelli-1.json`); in the panel the JS computes it (`LIVELLI.assess` in `mzlab/web/libreria.js`, same rules, `livelli-1.js`). Exposing the `mzlab-feat` cascade as a pure function `level_of(evidence)` in the libraries wasm holds only if the 614,400 B ceiling allows it: decision 1 (§13). Experimental exact HR → 2a; analog → 3b; in silico → never 2a; LR → empty.
- Neutral badges and columns for flags (e.g. `chimeric` from the `spectra.chimeric` step): no warning text.

## 9. Analogs: ΔFormula and known modifications
- **Single table** (to do) `mzlab-core/data/modifiche-1.json` (moved from `COMMON_DELTAS`, which becomes a reader): entries `{id, label, delta_formula, mass, kind}` with at least +O 15.9949, ±CH2 14.0157, −2H 2.0157 (−H2), ±H2O 18.0106, −Cl+H −33.9610, −Cl+OH −19.9720, +CF2 49.9968, +SO3 79.9568, +glucuronide 176.0321, + the 42 existing and the `delta_formula` of `regole_tp_2.json` (a single source; *why* Attributio and Tabui have separate lists).
- **Parent-constrained decomposition** (to do): with the parent formula (port `parent`, or of the library compound for analogs) proposals that remove an absent element or more atoms than present are discarded (no Cl is lost if the parent has none); order: known modifications within `delta_formula_mda` (5 mDa HR), then combinations of two known modifications, then free CHNOPS(+F,Cl,Br) search within the same tolerance, the simplest first, at most 3. In LR no ΔFormula (nominal mass).
- The shifted peaks of the modified cosine (Δ = prec_query − prec_lib) are the structural evidence of the analog: the mirror draws them dashed.

## 10. Expected-property kit (candidate only, to do)
- From formula: RDBE (exists), NOSC (to write, 10 lines in `mzlab-core`), KMD CH2 and CF2 (`kendrick.rs`), expected versus observed isotope pattern (`isoscore.rs`). From structure: TPSA and Crippen logP (`mzlab-mol/descr.rs`).
- ΔRT versus polarity: work shared with the TP module. Hook: `rt.predict` (already a step) gives `rt_ok` and direction; `regole_tp_2.json` gives `direzione_rt` per rule; `table.attribute` reads the output of `rt.predict` and writes `rt_dir_ok` (RPLC: lower logP → lower RT; HILIC: the opposite, parameter `chrom_mode`). No new RT logic in the spectral engine.
- The properties are output columns, never default filters.

## 11. Catalog and presets
- **Catalog** `librerie/2` (`catalogo.json`, generated by `tools/librerie_pacchetti.py` plus the versioned hand-written list `tools/librerie_catalogo.json`, schema checked by `--check-catalog`, details in `libraries.md`): per entry `id`, `nome`, `url` (page), `download` (file), `licenza` (SPDX), `formato`, `dimensione`, `sha256` or null, `kind`, `tech`, `pol`, `doi`, `verificato` (date of the check with a real fetch), `modo` (`site` = pieces on the site, only licenses that allow redistribution; `external` = desktop app only; `link` = link to the site only).
- The browser downloads only `site` (CSP unchanged); `external` stays desktop only (decision 4, §13).
- Menu: base group (MassBank, MS-DIAL, GNPS/MoNA) and closed "Specialist / in silico"; `link` entries for non-redistributable libraries; commercial NIST, mzCloud, paid METLIN excluded. Licenses, sizes and checks are in `tools/librerie_catalogo.json`.
- **Presets** (`mzlab-lib/data/preset-1.json`, `preset.rs`): filtered view, not a copy: `experimental` (kind experimental), `environmental` (MassBank, DIMSpec PFAS, MultiMS2, MS-DIAL), `metabolomics` (MS-DIAL, MoNA, GNPS, LipidBlast), `natural_products` (GNPS, ISDB if present), `forensic` (SWGDRUG if present); polarity and analyzer from the spectrum, automatic. The preset translates into a `u16` mask over the open libraries.

## 12. Open work
| work | depends on | main files |
|---|---|---|
| Attribution node: multi-library `table.match` (`library` `.many()`, `library_preset`, `search_mode`, `hybrid_entropy`/`stein_cosine`, `libmatch` block) + `table.attribute@1` + CLI | `libset`, `matchres` (done) | `flow_attribute.rs` in `crates/mzlab-cli/src/`, `flow_tablematch.rs`, `mzlab-feat/src/steps/` |
| wasm `LibSet.search` + side panel on `libmatch/1` (search on spectrum open, cancelable queued analogs) | `libset`, `matchres` (done) | `crates/mzlab-lib-wasm`, `libreria-worker.js`, `ident-pannello.js` |
| analogs: `modifiche-1.json`, parent-constrained ΔFormula, property kit (NOSC, `rt_dir_ok`) | `matchres`; the columns wait for `table.attribute` and the panel | `mzlab-core`, `matching.rs` |
| bench on real MassBank and GNPS (disk, visited entries) | – | `examples/libset_bench.rs`, `measurements-rust.md` |

## 13. Open decisions
1. **Level in the panel**: the JS computes it (`LIVELLI.assess` in `mzlab/web/libreria.js`); *recommended* Rust cascade in the libraries wasm if it fits the ceiling; alternatives: (b) `LIVELLI.assess` stays in JS; (c) raise the libraries wasm ceiling.
2. **Open search on the whole index** (beyond ±200 Da): *recommended* only on request ("Open search" entry); alternatives: (b) always in the Flow, never in the panel; (c) never.
3. **GC-EI**: the engine has Stein + ΔRI; *recommended* no interface until needed; alternatives: (b) leave it only in the engine with no plans; (c) full interface right away with the panel.
4. **External downloads in the browser**: *recommended* only packages redistributed by the site (free licenses); alternatives: (b) also external URLs if the verified CSP/CORS allows it (CSP to widen); (c) link only, no download in the browser.
5. **Analogs in LR**: *recommended* no analogs or ΔFormula at unit resolution (Principle 1 and nominal mass); alternatives: (b) analogs without ΔFormula; (c) as in HR.
