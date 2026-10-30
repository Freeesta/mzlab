# Code written twice: inventory and consolidation plan

Study of tools and algorithms that exist in more than one copy (JS and JS, JS and Rust, two Rust crates, two panels doing the same job), so that the optimisation work reuses ONE copy. Read-only on code, measured on `origin/main` at `5376565`. Every claim has a `file:line`; numbers marked «est.» are estimates, not measures. Ready-to-launch prompts are in `docs/studi/prompt_consolidamento/`.

## 0. Summary
- **Two real bugs found by the sweep** (fix them first, they are cheap):
  1. HRMS Target Screening finds nothing in the browser. `liste.js:638` imports `liste-target.js`, which reads `window.LISTE_TARGET_CORE` (`liste-target.js:7`), but no page file loads `liste-target-core.js` (not in `carica.js` `L`, no `carica()`/`import()`; only node tests `require` it). `CORE` is `{}` and every stub returns empty (`liste-target.js:358-366`, `cercaTargets` → `[]`). Static reading; confirm in a browser.
  2. The desktop app writes invalid JSON for errors with control or invisible characters: `mzlab-desktop/src/protocol.rs:38-50` formats params with `{:?}` (Rust Debug escaping: `\u{1}`, `\u{200b}`) and does not escape the key. The wasm side has the right writer and a test for exactly this case (`mzlab-wasm/src/lib.rs:663` `error_text_is_valid_json_for_control_characters`).
- **Start-up headroom is 10 886 B** (1 889 114 of 1 900 000, `tests/test_peso_avvio.py:8`, 143 files). JS chemistry that should be Rust sits in start-up files (`hr.js` 15 566 B, `tables.js` 17 503 B, `picchi.js` 58 485 B, `fondo.js` 7 072 B): moving it out frees start-up bytes, not only removes copies.
- **Rust: the biggest byte lever is generic code repeated per crate** (std sorts, 46 `fn median*`, CSV/MGF readers). `mzlab-lib/src/sortutil.rs` re-implements `mzlab-picchi/src/sortc.rs`; `mzlab-feat/src/flow_screen_suspects/` uses 5 std sorts in the `table` module against the `sortc` rule of `docs/agenti/wasm-tetti.md`.
- **One orphan crate**: `mzlab-target` is a Rust port of `liste-target-core.js` that no crate, wasm module or binary depends on (only its own tests, `Cargo.toml` workspace member). Today the JS is the engine, against the «Tutto in Rust» rule.
- **Desktop and wasm share the routes** (`mzlab_core::routes::Routes`, `mzlab-desktop/src/rpc.rs:15`, `mzlab-wasm/src/lib.rs:10`); the duplicated parts are the glue around them (error JSON, samples JSON, MSn-graph optional keys).
- **CLI and page Flow mostly share one Rust function per step** (§5); three steps diverge in their parameters, and eight CLI-only steps keep their logic inside `mzlab-cli`, where the wasm cannot reach it.

## 1. Ranking (top 10)
Score = (start-up/wasm bytes saved + bug risk removed) / effort, each 1-5. Prompts: `prompt_consolidamento/C<N>.md`.
| # | Item | Copies | Verdict | Bytes (est.) | Bug risk removed | Effort | Prompt |
|---|---|---|---|---|---|---|---|
| 1 | Target screening engine | `liste-target-core.js` (12 516 B, never loaded) + `mzlab-target` crate (orphan) + feat `screen.suspects` | fix the load now; then one Rust engine, delete the other two | lazy only; JS −12.5 K | 5 (feature dead in browser) | 2 → 4 | C1 |
| 2 | Error JSON and wasm glue | `js_err` ×9 (`fn js_err`), desktop `protocol.rs:38`, `JsReader` ×3 | one `mzlab_core::Error::json()`; desktop uses it | ~0 | 4 (desktop invalid JSON) | 1 | C2 |
| 3 | Fine isotope pattern | `hr.js:66-91` `isoFine`/`mergeRes` (start-up) vs `isoscore.rs:124/165` | Rust route, delete JS | start-up −2…3 K | 3 (floors 1e-10 vs 1e-12, element sets differ) | 2 | C3 |
| 4 | Std sorts in wasm (lib and table) | `sortutil.rs` vs `sortc.rs`; 6 std sorts in `mzlab-lib`, 5 in `flow_screen_suspects` | `mzlab-lib` uses `sortc`; replace std sorts | lib −12…60 K, table −10…50 K | 1 | 2 | C4 |
| 5 | JS formula mass and adduct tables | `tables.js:14` `massOf`, `pfas-calc.js:8`, `draw.js:461` `MONO`, `liste-target-core.js:7`, `fair.js:11`, `kmd-calc.js:5` | Rust `formula_mz`/`adduct_table` (exist: `elements.rs:290`, `adduct_table.rs:101`) | start-up −3…5 K | 3 (H mass 1.00782503 vs 1.00782503223) | 3 | C5 |
| 6 | Mass constants in Rust | proton in 4 values, electron in 2, 13C step in 3 (§2.1) | `mzlab_core::consts`, re-exported | ~0 | 3 (ppm-level drift between steps) | 3 (golden files) | C6 |
| 7 | Rust median/interp/FWHM/area helpers | 46 `fn median*` (15 core, 28 feat), interp ×5, FWHM ×5, area-over-line ×5 | public helpers in `mzlab-picchi/src/signal.rs` | each std-sorted median 2-10 K (est.) | 2 (NaN panic `rtlogp.rs:35`, empty → NaN/0/None) | 3 | C7 |
| 8 | Rust chemistry twins | `family/rel.rs:9` = `legacy/edge.rs:50`; `tp/candidati.rs:58` = `subformulas.rs:21`; formula parsers `explain.rs:63`, `confirm.rs:126` | call the core function | feat −3…8 K | 2 | 2 | C8 |
| 9 | JS signal code in start-up | `picchi.js:6-36` (`integ`, `sgFilter`, `autoEdges`), `fondo.js:13` SNIP, `cromato.js:6-28` | Rust (`peakq`, `signal.rs:229/275`) with parity test | start-up −2…4 K | 2 (edge handling differs) | 4 | C9 |
| 10 | JS UI helpers | escapers ×15, number formatters ×15, `cpNear`=`nearIdx`, `FILE_COLORS`=`OKABE`, CSV writers ×10, Blob download ×12 | one shared module per helper | start-up −2…3 K | 2 (`palEsc` does not escape `>`) | 2 | C10 |

## 2. Same algorithm written twice
### 2.1 Chemistry
| Tool | Copies (file:line) | What differs | Authoritative / tested | Verdict |
|---|---|---|---|---|
| Element masses | `mzlab-core/src/elements.rs:7` (14, AME2016); `mzlab-feat/src/attributio/review/adduct.rs:7` (36, older H 1.00782503207); `mzlab-feat/src/import/chem.rs:10` (+B, Fe); `mzlab/web/draw.js:461` (26, 8 decimals); `pfas-calc.js:8`; `matching.rs:175-180` | values up to 9e-9 relative | core; Attributio table is deliberate (`attributio/formula.rs:5-8`, `tests/attributio_formula.rs`) | merge JS into Rust; keep Attributio table, document |
| Electron | 0.000548579909 (`elements.rs:23`, `ricerca.rs:15`, `roles.rs:11`, `adduct.rs:5`); 0.00054858 (`subformulas.rs:6`, `tp/candidati.rs:29`, `tables.js:10`, `hr.js:50`, `draw.js:464`); inline `libreria.js:356` | 1.4e-9 Da | core | one const (C6) |
| Proton | 1.00727646688 (`roles.rs:10`); 1.00727646677 (`insilico.rs:14`, `flow_insilico.rs:14`, `rotture.rs:14`, `fragmenter.rs:18`); 1.007276466 (`envelope.rs:15`); 1.007276 (`adducts.rs:4`, `campaign.rs:10`, `b1.rs:11`, `mzlab-target/src/entry.rs:20`, `liste.js:10`, `liste-target-core.js:8`, `fair.js:11`) | up to 4.7e-7 Da | none declared | one const; where a golden pins 1.007276, keep a named `PROTON_6DP` (C6) |
| 13C step | 1.003355 (8 places incl. `roles.rs:12`, `liste-target-core.js:133`); 1.0033548 (`purity.rs:15`); 1.00335 (`ms2link.rs:32`) | 5e-6 Da | none | one const (C6) |
| Formula parsing | `elements.rs:114` (canonical, parentheses); `explain.rs:63` (lenient, i64); `reglists/confirm.rs:126` (lenient, u64, bad count → 0 where explain gives 1); `review/adduct.rs:75`; `import/chem.rs:19`; `msntree.rs:32`; JS `tables.js:16/121`, `liste-target-core.js:16`, `molui.js:8` | leniency, int type, bad-count rule | core (`chimica_parity.rs`) | core gets `parse_formula_lenient`; explain/confirm call it (C8); JS → Rust (C5) |
| Adduct tables | `adduct_table.rs:40` `CATALOG` (20, declared single source); `elements.rs:33` (7); `composition.rs:83` (11); `msngrafo/formule.rs:10` (9); `roles.rs:18` (literals 18.0338262, 41.0265491); `adducts.rs:4` (5, 6 dp); `import/chem.rs:54`; `mzlab-target/src/entry.rs:18`; JS `liste-target-core.js:7`, `fair.js:11`, `tables.js:28` | count, precision | `CATALOG`; `tests_e2e/e2e_calc.py:130` checks `tables.js` `ADD` against it | derive all from `CATALOG` (C5, C6) |
| Adduct + loss tables, whole | `mzlab-feat/src/family/rel.rs:9-35` `Tables::new` = `legacy/edge.rs:50-73` `Iimn::new`; same 23 losses (`rel.rs:134` = `edge.rs:26`), same isotopes (`rel.rs:132` = `edge.rs:23`) | only names | `legacy_parity.rs` pins edge | `rel.rs` reads `edge.rs` tables (C8) |
| Fine isotope pattern | `hr.js:66-91` (JS, start-up) vs `isoscore.rs:124` `fine_pattern`, `:165` `merge_res` | prune 1e-10 vs 1e-12, Rust cuts at +6.5 Da, JS all elements vs Rust 13 | Rust (`tests/test_isoscore.py`); JS only `e2e_hr_isotopi.py` (behaviour) | Rust route, delete JS (C3) |
| M+1/M+2 ratios | `composition.rs:289` `isotope_ratios` vs `attributio/review/explain.rs:310` `iso_pattern` | S abundances 0.9499 vs 0.9493 | core | explain calls core (C8) |
| Unit-resolution pattern | `tables.js:147` `isoPattern` (callers `picchi.js:496`, `spettro.js:238`) | no Rust twin | — | Rust nominal grouping on `fine_pattern` (C5) |
| Sub-formula masses | `mzlab-feat/src/tp/candidati.rs:58` `sub_masses` = `mzlab-core/src/subformulas.rs:21` `submasses` (same 300 000 cap) | no element sort, no `min_mz`, Option vs Result | core | call core (C8) |
| RDBE | `composition.rs:25`, `explain.rs:27`, `tp/candidati.rs:117`, `msngrafo/candidati.rs:61`, `fragtree.rs:99`, `purity.rs:180`, JS `elementare.js:24` | element sets (As, B, Se) | core | one `rdbe(counts)` in core (C8, low priority) |
| KMD units | `kmd-calc.js:5` vs `kendrick.rs:28` | literals vs computed | real parity test `tests/test_kmd.py:15` | keep (tested, 656 B) |
| PFAS MD/C | `pfas-calc.js:8-51` own masses, MD/C | no Rust twin | `tests/test_pfas.py` (JS only) | move to Rust when PFAS is next touched (C5) |
| SMILES | `mzlab-mol/src/smiles_parse.rs` (381 lines), `mzlab-feat/src/chem/smiles.rs` (279), `legacy/formula.rs:81-290` (+valence table `:9` = `chem/smiles.rs:12`) | aromaticity, valences | `legacy_parity.rs:61` pins legacy | backlog: one reader in `mzlab-mol` (large effort, wasm module boundaries) |
| Molfile V2000 | `mzlab-mol/src/molfile_v2000.rs` vs `mzlab-feat/src/chem/grafo.rs:37` | — | — | backlog |
| Spectral similarity | `mzlab-lib/src/scores.rs:86/101`, `prep.rs:19` (authoritative); `mzlab-core/src/entropy.rs:10/24/59` (O(n·m), used by `msngrafo/cerca.rs:197`); `legacy/similarity.rs:41`; JS `libreria-worker.js:107-180` | match algorithm | `mzlab-lib/tests/parita.rs:85` core vs lib (1e-9); no JS↔wasm | JS fallback: delete (one engine, AGENTS §4); core entropy: keep until `msngrafo` can depend on `mzlab-lib` |

### 2.2 Signal and data
| Tool | Copies | What differs | Authoritative / tested | Verdict |
|---|---|---|---|---|
| Savitzky-Golay | `picchi.js:24` `sgFilter` (JS, start-up); `mzlab-picchi/src/signal.rs:194/229/234`; `mzlab-core/src/centroidi/smooth.rs:6` (zero pad); `mzlab-feat/src/untargeted/smooth.rs:23` (truncated, renormalised) | edge mode | `signal.rs`; no JS↔Rust test | Rust copies: `savgol_coeffs(w,2,0)` + edge parameter (C7); JS: C9 |
| Smoothing 1-2-1 | `shell.js:24` `smooth` (display toggle) | — | — | keep (display only, 1 line) |
| SNIP baseline | `fondo.js:13` vs `signal.rs:275` | JS leaves ends unclipped, zeros on short trace | JS only `tests/fondo_node.js` | Rust (C9) |
| Area over a line | `picchi.js:6` `integ`, `cromato.js:17`, `liste-target-core.js:72`; Rust `mrm/resolver.rs:68`, `legacy/lr/peak1d.rs:105`, `broad_front.rs:26`, `peakq_auto.rs:171`, `origin.rs:62` | clamp at 0, s vs min, pairwise sum | `e2e_integra_hr.py:46-53` (page vs `auto_integrate`) | Rust: one helper, keep the pinned ones (C7) |
| FWHM | `etichette_picchi.rs:32` (no interp), `peakext.rs:73`, `params.rs:155`, `uv_bands.rs:238`, `quality.rs:91`; JS `cromato.js:8/26` | interpolation | — | one `fwhm(x,y,apex,lo,hi,base)` (C7) |
| Median | 46 `fn median*` (core 15, feat 28; e.g. `drift.rs:118`, `qc.rs:58`, `rtlogp.rs:35` panics on NaN); JS `composizione.js:40`, `fondo.js:11`, `mappa-render.js:50`, `mappa-dati.js:10`, `xic-lente-disegno.js:40` | empty → NaN/0/None/panic; upper/lower median variants (`hidden_fam.rs:71`, `centroidi/smooth.rs:122`) | `signal.rs:3/23`, `sortc.rs:82` `select_median` | `median_or(v, empty)`; keep pinned variants (C7) |
| MAD noise | `signal.rs:36` `noise_level`; `centroidi/smooth.rs:128`; `components.rs:62` | sub-range | signal.rs | call signal.rs (C7) |
| Linear interp | `signal.rs:298` (private), `rtmap.rs:44`, `analog.rs:99`, `differenza.rs:8`, `deconvolve/mcr.rs:348` | NaN fallback, panic-free | — | make `signal.rs` public (C7); `deconvolve/old` is a frozen test reference: keep |
| Nearest index | `grafici.js:609` `nearIdx` = `cromato.js:6` `cpNear` (character-identical); `mirino.js:12`; `dda.js:286`; `numeri.js:11` `lowerIdx`, `dda.js:285`, `liste.js:35`, `mappa-render.js:66` | tie rule | `test_cromato.py` (one copy) | `numeri.js` owns them (C10) |
| Window [lo,hi] | `decima.js:20` `windowIdx`, `numeri.js:14` `spanOf` (+sorted cache = `decima.js:14`), `mappa.js:391` | — | `decima_node.mjs` | `numeri.js` (C10) |
| M4 decimation | `decima.js:55` `m4`, `xic-lente-disegno.js:23`, `registro-serie.js:90`, `mappa.js:398`, `picchi.js:451`; Rust `analysis.rs:77` `minmax_indices` | — | `xic_lente_disegno_node.js` | `decima.js` (lazy) for JS draw paths (C10, low) |
| Map normalisation | `grafici.js:534` `mapNorm` vs `mappa.js:93` `normFactor` | — | — | one function (C10) |
| Stable sort | `mzlab-lib/src/sortutil.rs` (189 lines) = `mzlab-picchi/src/sortc.rs` (222) | — | `sortc` | delete `sortutil` (C4) |

### 2.3 Rust infrastructure
| Item | Copies | Verdict |
|---|---|---|
| Error → JSON | `fn js_err` in 9 bridge crates (e.g. `mzlab-feat-wasm/src/lib.rs:56`, `mzlab-chem-wasm/src/lib.rs:9`); `mzlab-llm-wasm/src/lib.rs:62` (+`error`); desktop `protocol.rs:38` (Debug escaping, bug) | `Error::json()` in core (C2) |
| `JsReader: ReadAt` | `mzlab-feat-wasm/src/lib.rs:92` = `mzlab-feat-table-wasm/src/lib.rs:42` = `mzlab-finder-lr-wasm/src/lib.rs:18`; variants `msngrafo-wasm/src/lib.rs:21`, `mzlab-wasm/src/nativi.rs:42` | finder-lr already depends on feat-wasm: re-export (C2) |
| JSON writing | `mzlab-core/src/json.rs:239` `Out` (authoritative); `attributio/review/proposal.rs:174-248` own `enum J`; `mzlab-libro/src/index.rs:180`; `mzlab-lib/src/matchres/json.rs:70` (serde number format differs from `Out::num`) | backlog: use `Out` when those files are next touched (output bytes change: golden) |
| Python float repr / round | `py_float` ×3 (`scantable/json.rs:24`, `attributio/export/pyfmt.rs:8`, `readers/mzmine.rs:141`); `fmt_g` ×2 (`json.rs:336`, `annotate/text.rs:13`); Python round ×7 (`jsonround.rs:4` authoritative; `chimeric.rs:34` uses `round_ties_even`, not Python's rule) | one `pyfmt` in core; check `chimeric.rs:34` against Python (backlog) |
| Base64/zlib | `mzmlarray.rs:109`, `numpress.rs:32`, `mzlab-lettori/src/aperti/mzxml_scan.rs:147` | backlog (readers, perf-sensitive) |
| SHA | crates `sha1`/`sha2` in core; hand-written `mzlab-lib/src/sha256.rs:56`, `attributio/export/sha1.rs:33`; `sha256_hex` ×2 (`budget.rs:178`, `desktop/update.rs:179`); `sha1_of` ×2 (`agilent/run.rs:248`, `agilent_ext/run.rs:264`) | hand-written copies exist to keep wasm modules small: keep, but agilent twins merge (low) |
| CSV writers | 9+ (`feat/export.rs:61`, `attributio/table.rs:250`, `liste/route.rs:17`, `attributio/frame.rs:300`, `reglists/schema.rs:107`, `core/campioni/csv.rs:346`, `molecules/csv.rs:300`, `msngrafo/tabelle.rs:32` (no `\r`), `cli/flow_quant.rs:198`) | one `csv::cell` in core (backlog; byte-identical outputs required) |
| CSV readers | 11+ (`mzlab-target/src/csv.rs:167` RFC 4180, `feat/import/delim.rs:88`, `campioni/csv.rs:86`, `cli/normative/csvrd.rs:27`, `cli/flow_quant.rs:71`, …) | backlog |
| MGF readers | `mzlab-lib/src/parse.rs:402` (authoritative; `mzlab-feat` depends on `mzlab-lib`) + 7 (`feat/import/mgf.rs:66`, `readers/ms2.rs:94`, `review/propose.rs:393`, `export/sirius.rs:135`, `flow_level/extra.rs:153`, `flow_screen_suspects/evidence.rs:23`, `cli/flow_chimeric.rs:44`) | backlog: call `mzlab-lib` (check feature flags pull no extra code into `hr`) |
| MSn graph optional keys | `desktop/rpc_msn.rs:262/296`, `msngrafo-wasm/src/lib.rs:98/147`, `cli/msn_graph.rs:147`; `desktop/tests/parita_rotte.rs` covers `msn_albero`, not `msn_grafo` | one function in core + parity test (backlog) |
| Samples JSON | `desktop/nativi.rs:51` vs `mzlab-wasm/src/nativi.rs:233` | one function in core (backlog) |
| To-mzML | `mzlab-lettori-wasm/src/lettura.rs:276` `to_mzml` vs `mzlab-wasm/src/nativi.rs:468` `native_to_mzml` | verify: probably two bridges over one writer; merge glue only |

### 2.4 JS UI written twice
| UI | Copies | Verdict |
|---|---|---|
| Sortable table headers | `riferimento.js:73-81` (button + `aria-sort`, tested `e2e_differenza_hr.py`), `infusione-albero.js:88`; without keyboard/`aria-sort`: `mappa-tabella.js:96`, `dda.js:394`, `banco.js:425` | one header helper on the `riferimento.js` pattern; also an accessibility fix (AGENTS §1.6) — backlog after C10 |
| Shared table component | only `TP_Mine/js/42_tp_table.js:413` `TPTAB` (4 users); `TP_Mine/js/33_pipeline_view.js:14` writes its own virtual list | `33_pipeline_view.js` → `TPTAB` (backlog) |
| Virtual lists | `banco.js:117-131` and `banco.js:415-427` (same scan list, overscan 2/5 vs 1/3) | one windowing function in `banco.js` (backlog, start-up −0.5 K) |
| Dialogs | shared `ask`/`yesno`/`info`/`big`/`menu` in `explore.js:906-936`; own copies: `predefinito.js:27`, `reset.js:97`, `memoria-avviso.js:22`, `molecole-ui.js:206`, `libreria.js:368` and `:439` (creates `#libres` twice) | use `explore.js` helpers (backlog) |
| Toasts | `window.toast` (`explore.js:8`), `nearMsg` (`picchi.js:116`), `hr.js:38-45` `#hrerr` (copy of `nearMsg`) | fold `#hrerr` into `nearMsg` (C10) |
| CSV/TSV export | shared `dl` (`explore.js:1002`, provenance), `xlsx.js:94` `csvBuild`, `ordine.js:53` `saveBlob`; own writers: `liste.js:202` (no provenance), `liste-target-core.js:260`, `liste-target.js:262` (no provenance); own Blob download: `rapporto-flow.js:146`, `esporta-dati.js:146`, `molecole-ui.js:233` | use shared (C10) |
| Escaping | `EH` (`explore.js:153`), `h` (`html-sicuro.js:9`); copies `shell.js:8`, `ricerca.js:12`, `paletta.js:12` (no `>`), `barra.js:472` (no `>`), `liste.js:114/457/551`, `tavola-periodica.js:12`, `cartella-puntata.js:56`, `tpmine-loader.js:66`, `msngrafo-vista.js:67`, `rapporto-flow.js:35`, `avanzamento.js:29`, `avviso-capacita.js:9` | `h` everywhere (C10) |
| Number format | sci rule copied in `bianco.js:17`, `riferimento.js:19`, `qaqc.js:111`, `profilo.js:21`, `rapporto.js:68`; `'–'`/`toFixed` in `tabella.js:108`, `bianco.js:16`, `pca.js:18`, `kmd-vista.js:9`, `pfas.js:13`, `mappa-estrai.js:27` | one `I18N.sci`/`I18N.fix` (`i18n.js:58` already holds the number format) (C10) |
| Palette | `lista-file-menu.js:6` `FILE_COLORS` byte-identical to `explore.js:78` `OKABE` | alias (C10) |
| Canvas DPR | `shell.js:14,22`, `mappa-margini.js:8` (+ DPR watcher twice: `shell.js:17`, `mappa-margini.js:26`), `xic-lente.js:30,44` (no cap, ignores pinch) | backlog |
| Mirror spectra | `libreria.js:273`, `TP_Mine/js/52_attributio_banco.js:51`, `27_untarget_ui.js:261`, `44_tp_riga.js:158`, `44_posizioni.js:157` | backlog: one SVG mirror in TP_Mine |
| Sample/role tables | `shell.js:109-135` + `ruoli-celle.js`, `campioni-tabella.js` + `campioni.js`, TP `finder-ruoli.js` | different jobs on the same data: keep, share the cell editors later |
| Elemental composition panel | `composizione.js:136` and `elementare-pannello.js:116` both call `api/composition` | verify the overlap before merging (two entry points, one engine: not a code duplicate) |

## 3. Rust and JS both present: who is authoritative
| Pair | Authoritative | Parity test |
|---|---|---|
| Target screening (`liste-target-core.js` / `mzlab-target`) | JS (Rust is unused) | none |
| Fine isotope (`hr.js` / `isoscore.rs`) | Rust for scores, JS for the HR panel | none |
| Formula mass (`tables.js` / `elements.rs`) | Rust for the calculator, JS for tables/losses | `e2e_calc.py:130` (adducts only) |
| Library scores (`libreria-worker.js` / `mzlab-lib`) | Rust (JS is a fallback, `forceJs` `libreria-worker.js:400,691`) | none numeric (`e2e_libreria_wasm.py` checks the engine choice) |
| Peak integration (`picchi.js` / `peakq_auto.rs`) | both, different algorithms on LR and HR | `e2e_integra_hr.py:46-53` (area) |
| SNIP (`fondo.js` / `signal.rs:275`) | JS on page | none |
| KMD units (`kmd-calc.js` / `kendrick.rs`) | Rust | `tests/test_kmd.py` (real parity) |
| Losses (`perdite.js` formulas) | JS masses via `tables.js` | `tests/test_perdite.py` (against Python) |

## 4. Size rules for every consolidation
- Start-up JS: removing code from a start-up file only helps; adding a shared module to `L` costs its full size. A shared JS helper goes into an existing start-up file that all users already load (`numeri.js`, `html-sicuro.js`, `i18n.js`), never a new start-up file.
- Wasm: moving a function into `mzlab-core` costs nothing in modules that already link it, but a function moved into `mzlab-core` and called from a module that did not use core pulls core's dependencies (json, fmt): check with `tools/ci_wasm_dimensioni.py` before and after. Caps never rise (`docs/agenti/wasm-tetti.md`).
- Every Rust replacement keeps byte-identical output where a golden or a parity test pins it; where values change (constants, medians), the prompt says which golden moves and why.

## 5. CLI vs page Flow
- Shared Rust function on both sides: `table.drift` (`flow_drift::drift_text`), `spectra.ms2` (`flow_ms2`), `spectra.match` non-tree (`mzlab_lib::flowmatch`, `cli/flow_match.rs:43`, `lib-wasm/lib.rs:39`), `table.match` (`rows_mgf`), `formula.ms1` (`cli/flow_formula.rs:18`, `feat-wasm/run_formula.rs:11`), `formula.ms2`, `annot.level`, `spectra.insilico`, `screen.suspects`, `qc.mass_accuracy` (`massacc::check`), exports, `table.filter`, `tp.review/refilter/position`.
- Divergences (parity bugs, not code duplicates; backlog for the Flow area):
  1. `spectra.msn_graph`: CLI `opzioni()` (`cli/flow_msngraph.rs:234-279`) resolves `molecule` from the list and handles `molfile`, `regole`; the page `graphQuery` (`flow/lazy/flow-web-msn.js:17-32`) drops `molfile` and `regole`.
  2. `spectra.match` tree: CLI `tree_text` (`cli/flow_match_tree.rs:33-62`) converts `precursor_da`, applies `name_contains`, `decoys=spectrum`; the page sends raw `api/msnlib/cerca` with its own defaults (`flow-web-msn.js:10,53-64`).
  3. `qc.mass_accuracy`: the CLI passes an empty SHA-256 per file (`cli/flow_massacc.rs:93`), so `istd_flags` (`feat/massacc/mod.rs:265`) matches by name only; the page passes the SHA.
- Logic that lives only in `mzlab-cli` (the wasm cannot call it, so the page has no executor; only a name in `flow/lazy/flow-names.js:51`): `flow_quant.rs` (own CSV), `flow_rtpredict.rs`, `flow_compare.rs`, `flow_chimeric.rs` (own MGF), `flow_link.rs`, `flow_explain.rs`, `flow_uv.rs`, `flow_pca.rs`. Moving each into `mzlab-feat` is the precondition for the site/app parity rule; not a duplicate today, a future one if the page re-implements them.

## 6. Not duplicates (checked)
- Python in `mzlab/*.py`, `mzlab/chem/*.py`, `tools/*_riferimento.py`: parity references only.
- `lr.js`, `rust-worker.js`, `rust-bridge.js`, `rust-nativo.js`, `TP_Mine/files/*.js`, `flow/lazy/flow-web-*.js`: glue, no computation.
- `etichette-picchi.js`, `peaks-worker.js`: display Rust results.
- `composizione.js:42` parabola centroid vs `centroidi/emit.rs:86` Gaussian: different algorithms.
- `mzlab-feat/src/deconvolve/old/`: frozen test reference (`deconvolve/tests.rs:107`).
- `attributio/review/isotopes.rs:106`: kept apart on purpose (`:5`).
- Desktop routes: shared through `mzlab_core::routes::Routes`.

## 7. Zones of running PRs (gates)
Open PRs at the time of the study and the files that gate a prompt: #2207 landing (`area-lavoro.js`, `carica.js`, `conversione.js`, `explore.js`, `lista-file.js`, `lr.js`, `index.html`); #2209 neg-LR facets (`mzlab-core/src/scantable.rs`, `flow/check.rs`, `mzlab-feat/src/steps/tabui*.rs`, `mzlab-cli/src/flow_run.rs`); #2218 normative screening (`mzlab-attributio-wasm`, `mzlab-cli/src/flow_screen_suspects*.rs`, `cli/normative/`); #2219 key guard (`azioni.js`, `eventi.js`, `elementare-pannello.js`, `msngrafo-vista.js`, `storia.js`); #2220 chromatogram menu (`eventi.js`, `picchi.js`); #2221 theme menu (`settings.js`). Each prompt names its gate; a prompt whose files are in a still-open PR waits for that PR to merge.
