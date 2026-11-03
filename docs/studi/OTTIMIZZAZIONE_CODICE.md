# Dead, duplicated and over-complicated code: what to delete, merge or load later

Study of code that nothing reaches, of logic written twice and of start-up and wasm bytes that buy nothing. Read-only on product code, measured on `origin/main` at `e4ab1f85f4750e304ce81c8d1d13ce34b2548ff9`. Every claim has a `file:line` and the command that shows it; «est.» marks an estimate. Build prompts: `docs/studi/prompt_codice/`. The duplicated-logic inventory is `docs/studi/DUPLICAZIONI_CODICE.md` (prompts `prompt_consolidamento/C1..C10`); this study does not repeat it and only updates its status (§6).

## 0. Summary
- **Unused Rust `pub fn` cost 0 wasm bytes** (release is `lto = true`, `Cargo.toml` `[profile.release]`: the linker drops them). They cost lines and review time only. **Unused `#[wasm_bindgen]` exports are roots and DO cost bytes**: they are the wasm lever of this study.
- **Two wasm exports nobody calls carry a whole engine path into the `hr` module**: `detect_mrm_file` and `align_mrm_series` (`crates/mzlab-feat-wasm/src/lib.rs:289-340`, feature `mrm`), plus `estensioni` (`mzlab-lettori-wasm/src/lettura.rs:192`, in all 9 reader modules) and `detect_format` (`mzlab-lib-wasm/src/lib.rs:65`). Measured on `hr`: −31.6 KB of code (§2.2).
- **16 Rust `pub fn` appear once in the whole repository** (their definition), ~210 lines (§1.1), among them an MRM calibration CSV writer (`mzlab-feat/src/mrm.rs:333`) that would contradict principle 1 if anyone wired it.
- **The site builds compile with 8 warnings that CI never sees**: CI checks the native build with all features; the wasm feature sets leave fields, imports and variables unused (§1.2). One is a stale binding after a behaviour change (`etichette_picchi.rs:103`).
- **The engine carries 95 std-sort instances, 98 KB of code before `wasm-opt`** (twiggy, §2.1). Five of them sort `usize` indices by one `f64` key with the same algorithm; one shared helper keeps the std speed and drops four copies (~29 KB before `wasm-opt`).
- **Start-up JS: 28 598 B headroom** (1 871 402 of 1 900 000, `tests/test_peso_avvio.py:8`, 145 files). Two files can follow the lazy pattern the codebase already uses for PCA/KMD/PFAS: `riferimento.js` (19 983 B, the pattern is in the same file, `:222-227`) and `libreria.js` (49 555 B, only three menu registrations are needed at boot). Together ≈ −65 KB of start-up (§3).
- **JS dead code is small**: `apertura-area.js` keeps six functions of an opening flow that `avvio-progressivo.js` replaced (~1.5 KB of start-up), `storia.js:106` `annullaVoce`, ~1 KB of CSS rules for elements that no longer exist (§1.3).
- i18n orphan keys are already refused by `tools/controlla_i18n.py:472` (exit 0 today); no action.

## 1. Code nothing reaches
### 1.1 Rust functions and exports
Method (reproducible): `python3` script that lists every `pub fn` under `crates/*/src` outside `#[cfg(test)]` and counts the word in every `.rs`, `.js`, `.py`, `.html`, `.sh` of `crates/ mzlab/ flow/ TP_Mine/ tools/ tests/ tests_e2e/ fuzz/`, then `rg -w <name>` by hand on each hit. Count = 1 means only the definition.

| `file:line` | Item | Lines | Note |
|---|---|---|---|
| `mzlab-feat/src/mrm.rs:286` | `qualifier_ratios` | 42 | no caller, no test |
| `mzlab-feat/src/mrm.rs:333` | `csv_calibration` | 41 | no caller, no test; a calibration table is what the LR world must not produce (principle 1) |
| `mzlab-feat/src/interp/albero.rs:176` | `null_margins` | 37 | |
| `mzlab-chem/src/mol3d.rs:141` | `to_xyz` | 13 | |
| `mzlab-lib/src/format.rs:273` | `unshuffle_peaks` | 12 | |
| `mzlab-feat/src/trace.rs:85` | `with_centwave_width` | 12 | |
| `mzlab-feat/src/attributio/review/raw.rs:55` | `find_raw_dir` | 11 | |
| `mzlab-feat-table-wasm/src/pol.rs:30` | `recipe_polarity` | 9 | not an export |
| `mzlab-feat/src/attributio/frame.rs:53` | `column_texts` | 6 | |
| `mzlab-lib/src/libset.rs:171` | `set_sidecar` | 4 | |
| `mzlab-lib/src/read.rs:644` | `index_entries` | 4 | |
| `mzlab-cli/src/normative/sources.rs:88` | `resolve_params` | 3 | |
| `mzlab-cli/src/normative/table_rd.rs:289` | `cas_of` | 3 | |
| `mzlab-feat/src/attributio/checks/mod.rs:328` | `campaign_fit` | 3 | |
| `mzlab-mol/src/descr.rs:145` | `crippen_mr` | 3 | |
| `mzlab-mol/src/descr.rs:208` | `rotatable_bonds` | 3 | |

wasm exports with no JS caller (script over every `#[wasm_bindgen] pub fn` of `crates/*-wasm`, JS name taken from `js_name`, searched in `mzlab/web flow TP_Mine`; dynamic calls checked by hand: `TP_Mine/files/tpmine-worker.js:25,288` builds `"finder_" + fn`, so `finder_identity_graph`, `finder_polarity_pairs`, `finder_origin_evidence` ARE reached):

| `file:line` | Export | Module(s) | Evidence |
|---|---|---|---|
| `mzlab-feat-wasm/src/lib.rs:292` | `detect_mrm_file` | `hr` | `rg -n 'detect_mrm_file' mzlab flow TP_Mine tests tests_e2e tools` → only `Cargo.toml` comment; no `load("mrm")`, no `mods.mrm` |
| `mzlab-feat-wasm/src/lib.rs:312` | `align_mrm_series` | `hr` | same |
| `mzlab-lettori-wasm/src/lettura.rs:192` | `estensioni` | the 9 `wasm-lettori/*` | only `lang/contesto.json` prose matches |
| `mzlab-lib-wasm/src/lib.rs:65` | `detect_format` | `wasm-lib` | the importer calls `mzlab_lib::format::detect` itself |

Exports reached only by node parity tests (`tests/feat_wasm_node.js`, `tests/attributio_descr_node.js`, `tests/feat_table_node.js`, `tests/test_normative_wasm.py`, `tests/guardie_memoria_node.js`): `detect_file`, `align_series`, `build_evidence`, `rt_descriptors`, `table_check`, `normative_listings`, `memory_bytes`. They are the parity harness of the wasm: **keep**.

`mzlab_feat::mrm` (`mrm.rs` 433 lines + `mrm/resolver.rs` 155) is reached in product code only by the two dead exports (`rg -n 'mzlab_feat::mrm|crate::mrm' crates --type rust`: `mzlab-feat-wasm/src/lib.rs:281-338`, `examples/nativo_mrm.rs`, tests `lr_mrm_radi.rs`). The MRM path of mzFinder in the browser is `mzlab-finder-lr-wasm` (`finder_lr_mrm`, `TP_Mine/files/finder-lr.js:113`). Deleting the module is a decision (§7); deleting the two exports is not (they serve nobody).

Not dead, by design: `mzlab-feat/src/deconvolve/old.rs`, `rtwarp/before.rs`, `centwave/before.rs` are `cfg(test)` oracles («verbatim copy … kept only so the unit tests can prove the new code is bit-identical», line 1 of each). `mzlab-core/src/bench_stats.rs` serves `tools/bench/statistica.py`. 118 more `pub fn` are reached only by tests (list reproducible with the same script); they are test APIs and cost 0 bytes: no action.

### 1.2 Warnings of the site builds
`cargo check --target wasm32-unknown-unknown` with the exact feature set of every `tools/build_*_wasm.sh` line (24 builds). The native `cargo check --workspace --all-features` shows only 2 of them, so CI's clippy never fails on the rest.

| `file:line` | Warning | Builds | Fix |
|---|---|---|---|
| `mzlab-core/src/etichette_picchi.rs:103` | `ys` unused; the comment above still says the valleys are measured on `ys`, but since the wide smoothing (`:114`) they are measured on `wide` | every build with mzlab-core (also native) | drop the binding, rewrite the comment |
| `mzlab-core/src/wiff/lettore.rs:48-49` | fields `bpc`, `bpc_seen` never read without feature `estesi` | engine, rotte, lettori | gate the fields (and their init `:299-300`) on `estesi` |
| `mzlab-core/src/wiff/lettore.rs:159` | `c`, `k` unused without `estesi` | same | `#[cfg_attr(not(feature = "estesi"), allow(unused_variables))]` or `_` names in the non-estesi arm |
| `mzlab-feat/src/tp/mod.rs:22` | unused import `tabella::measured` | 3 `mzlab-feat-wasm` builds | gate the import on the feature that uses it |
| `mzlab-feat/src/series.rs:202` | `workers` unused on wasm | 3 builds | move it inside the native branch |
| `mzlab-feat/src/chunked.rs:4` | unused import `Mutex` | 3 builds | gate on `not(target_family = "wasm")` |
| `mzlab-attributio-wasm/src/lib.rs:13` | unused import `self` | all attributio builds (also native) | remove |

### 1.3 JS and CSS
Method: every top-level `function NAME(` / `const NAME = (…) =>` of `mzlab/web`, `flow`, `TP_Mine` whose name occurs once in all product files (JS, HTML, CSS, `crates/mzlab-desktop`), then `rg` by hand. Whole files: every basename of `mzlab/web flow TP_Mine` searched in the others (none dead: the `an-*.js` are loaded by name, `avvio.js:14`; `TP_Mine/js/*` by glob, `tools/build_site.py:83`).

| `file:line` | Item | Evidence | Start-up bytes |
|---|---|---|---|
| `mzlab/web/apertura-area.js:22-33` | `openingStato`, `openingFile`, `openingFiles`, `openingStart`, `openingEnd`, `openingCancel`, `openMark` (`:9`), `window.MZLAB_OPEN` (`:10`, only `tools/tipi/globali.d.ts:84` declares it) | `rg -l '\bopeningStart\b' mzlab flow TP_Mine tests tests_e2e` → only the file itself; the live opening skeleton is driven by `avvio-progressivo.js:186-198` and `area-lavoro.js:71`; `openingView`/`openingRender`/`OPEN_ST` stay (`shell.js:34`) | ≈1 517 |
| `mzlab/web/storia.js:106-109` | `annullaVoce` (inner, not in the returned object `:156`) | name occurs once | 192 |
| `mzlab/web/app.css` | rules for `#resetnote`, `.ttlsel`, `.nlc`, `.nlhint`, `.nlpp`, `.cvst`, `.cvr[data-st=fail]`, `.cardhd`, `#helppop .hp-s`, `.cp-total` (24 rules) | no JS/HTML/Rust emits the names; several e2e assert they are absent (`e2e_reset.py:50`, `e2e_perdite.py:73`, `e2e_addotti_default.py:19`); `.lv-*` look dead but are built as `"lv-" + k` (`flow-run-end.js:28`): keep | ≈1 100 (CSS, not in `MAX_TOTAL`) |

Functions reached only by e2e hooks (`addTrace` `pannelli.js:850`, `ugSet` `lista-file.js:195`, `ibkPick` `grafici.js:246`, `uipContOn` `settings.js:118`, `undoZoom` `storia.js:167`): test API, keep.

## 2. wasm bytes
### 2.1 Engine (`mzlab_wasm`), twiggy on the `cargo` output
Commands: `tools/cargo_wasm.sh build --locked --release --target wasm32-unknown-unknown -p mzlab-wasm` then `twiggy top -n 100000 -f csv target/wasm32-unknown-unknown/release/mzlab_wasm.wasm` (twiggy 0.8.0 installs with `cargo install twiggy --locked` in the cloud). 3 370 960 B before wasm-bindgen and `wasm-opt` (site: 1 973 928 B, `stat-ci` at the same SHA); code 2 239 086 B.

Biggest crate::module groups (code bytes): `mzlab_core::agilent` 123 837, `core::slice` 100 476 (sorts), `core::iter` 82 191, `mzlab_core::routes` 80 208, `alloc::vec` 70 610, `scantable` 57 455, `mzml` 50 324, `wiff` 48 321, `methodinfo` 44 368, `zlib_rs::deflate` 42 042. Biggest functions: `composition::compose` 24 573, `methodinfo::method` 21 788, `Canali::da` 20 862, `mzml::parse_loop` 20 409.

**Std sorts: 95 instances, 97 999 B.** Most are marked hot on purpose (`#[allow(clippy::disallowed_methods)]` with «std is ~2x faster than sortc», `OTTIMIZZAZIONE_CORE.md` §4.1): keep the algorithm. But five of them are the SAME sort (stable, `usize` indices, ascending or descending `f64::total_cmp` key) instantiated once per closure:

| Call site | twiggy instance | Bytes |
|---|---|---|
| `mapgrid.rs:147` `rt_groups` (key `points[a].rt`) | `sort_by<mapgrid::rt_groups::{closure}>` | 7 363 + 217 |
| `specpeaks.rs:172` `pad_zeros` (key `om`) | `sort_by<specpeaks::pad_zeros::{closure}>` | 7 338 + 217 |
| `specdiff.rs:178` `subtract_hr` (key `rmz`) | `sort_by<specdiff::subtract_hr::{closure}>` | 7 310 + 217 |
| `specpeaks.rs:133` `merge_unit` (key `mz`) | `sort_by<specpeaks::merge_unit::{closure}>` | 5 421 + 217 |
| `mapcells.rs:162` `sticks` (key `pts[i].2`, descending) | `sort_by<Routes::sticks::{closure}>` | 3 626 + ~1 000 |

One `pub(crate) fn order_by_f64(key: &[f64], desc: bool) -> Vec<usize>` with ONE `sort_by` inside (`desc` reverses the `Ordering`, which is exactly `key[j].total_cmp(&key[i])`) gives the same permutation (same stable std algorithm, same comparisons) and one instance instead of five: ≈ −24…29 KB before `wasm-opt`, est. −15…20 KB on the site file. Two call sites need the key as a slice (`rt_groups`, `sticks`): one `Vec<f64>` of n per call, negligible next to the sort. Proof of no output change: the goldens of `specpeaks`, `specdiff`, `mapgrid`, map sticks (parity tests in `crates/mzlab-core/tests/`, `tests/test_*map*`), plus a new unit test that compares the helper with the old closure on data with ties and NaN.

Not proposed: `mzidx_ordine.rs:113` (`Peak<f32>` 12 796 B and `Peak<f64>` 10 218 B) is the hot radix fall-back of the `.mzidx` builder, generic over the float type on purpose.

### 2.2 `hr` module: dead MRM exports
`hr` (1 522 745 B on the site, cap `limit_hr=1605632`, `tools/build_feat_wasm.sh:12`) is built with `--features detect,proofs,mrm,finder,finder_edge,finder_msn,flow_ms2` (`:31`). Built twice with `tools/cargo_wasm.sh` (before wasm-bindgen and wasm-opt): with `mrm` 3 421 581 B, without 3 372 884 B; twiggy code bytes 1 788 556 → 1 756 973, **−31 583 B of code** (the rest of the difference is the name section, stripped on the site); gzip −9 of the raw files −15 336 B. Est. −22…30 KB on the site file, for two functions the browser never calls. `mrm` is not a manifest key the workers load: `rg -n 'load\("mrm"\)|mods\.mrm' TP_Mine flow mzlab/web` → nothing (`build_feat_wasm.sh:36` lists it only for the `MZLAB_WASM_SEPARATI` build).

### 2.3 Crates compiled into more than one module
`mzlab-core` is in all 25 modules by construction; the split and fusion of modules is `WASM_PACCHETTI.md` and `docs/agenti/wasm-tetti.md`. No new finding.

## 3. Start-up JS that can load later
`tests/test_peso_avvio.py` sums the list `L` of `mzlab/web/carica.js:6`: 1 871 402 B, cap 1 900 000. Biggest: `pannelli.js` 106 720, `mappa.js` 98 688, `explore.js` 88 732, `barra.js` 65 546, `eventi.js` 65 180, `grafici.js` 59 718, `picchi.js` 57 971, `banco.js` 57 900, `liste.js` 50 748, `libreria.js` 49 555.

| File | Bytes | Who needs it at boot | Verdict |
|---|---|---|---|
| `riferimento.js` | 19 983 | only `STAT.add(...)` of four sections (`:221-227`) and two event listeners that act only on a mounted box (`:218-220`); `pca`, `kmd`, `pfas` already load with `carica()` from the same lines | **lazy**, stub ≈1.6 KB: −18 KB |
| `libreria.js` | 49 555 | three `AZIONI.registra` (`:416-430`), `settings.js:206` (`if (window.LIB) LIB.open()`), the sync `isHr` of the menu label (`:382`, needs `queryOf`/`profOf`); everything else runs on a click or in `ident-pannello.js`/`libreria-finestra.js`, both already lazy | **lazy**, stub ≈3-5 KB with the same `LIB` names, async methods forward after `carica()`: −44…46 KB |
| `fair.js` | 16 546 | `AZIONI` `disponibile: c => FAIR.available(c.p)` is synchronous (`:185`) and `tests/test_fair.py:183` requires the file | not a pure win; skip |
| `banco.js`, `dda.js`, `liste.js`, `mappa.js` | 50-99 K | `BANCO` in 18 files, `DDA` in 29, `LISTE` in 11, `MAPPA` in 7 | backlog: needs a design per file |

Test hooks that must keep working: `e2e_differenza_hr.py:64-158` (`RIFERIMENTO.result` after opening the section), `e2e_libreria*.py`, `e2e_inglese_hr.py:65` (`LIB.open()`, `LIB.call`, `LIB.engine`). The stub keeps those names; nothing in a test is loosened.

## 4. Ranking
Gain = bytes the user downloads or lines removed; risk = chance of an output change.

| # | Item | Gain (measured) | Risk | Prompt |
|---|---|---|---|---|
| 1 | Lazy `riferimento.js` + `libreria.js` | start-up −62…64 KB of 1.87 MB (headroom ×3) | low-medium (boot order) | 03 |
| 2 | Dead wasm exports (`hr` MRM, `estensioni`, `detect_format`) + dead `mrm.rs` writers | `hr` code −31.6 KB pre-opt (est. −22…30 KB site); 9 reader modules a few hundred B each | low (no caller) | 02 |
| 3 | Shared index sort in the engine | engine −24…29 KB pre-opt (est. −15…20 KB site) | low (same algorithm, goldens) | 05 |
| 4 | Dead Rust `pub fn` + site-build warnings | −210 lines, 8 warnings, one stale comment | very low | 01 |
| 5 | JS/CSS dead code | start-up −1.7 KB, CSS −1.1 KB | very low | 04 |

## 5. Waves
Files are disjoint inside a wave; `tools/build_feat_wasm.sh` and `carica.js` each have one owner.

| Wave | Prompt | Owns |
|---|---|---|
| 1 | 01 dead Rust + warnings | the `.rs` files of §1.1 except `mrm.rs` and the `*-wasm` exports; the files of §1.2 |
| 1 | 02 dead wasm exports | `mzlab-feat-wasm/src/lib.rs`, `mzlab-feat-wasm/Cargo.toml`, `mzlab-feat/src/mrm.rs`, `mzlab-lettori-wasm/src/lettura.rs`, `mzlab-lib-wasm/src/lib.rs`, `tools/build_feat_wasm.sh` |
| 1 | 03 lazy start-up | `mzlab/web/carica.js`, `riferimento.js`, `libreria.js`, new `riferimento-corpo.js`, `libreria-corpo.js`, `tools/tipi/` entries |
| 2 | 04 JS/CSS dead code | `apertura-area.js`, `storia.js`, `app.css`, `tools/tipi/globali*.d.ts` |
| 2 | 05 engine index sort | `mzlab-core/src/specpeaks.rs`, `specdiff.rs`, `mapgrid.rs`, `mapcells.rs`, one new helper in `mzlab-core` |

## 6. Status of the duplication prompts (`prompt_consolidamento/`)
Checked on `e4ab1f8`: **done** C2 (`mzlab-desktop/src/protocol.rs` has no `{:?}` writer), C3 (`hr.js:49` calls `api/isotope_fine`), C4 (`mzlab-lib/src/sortutil.rs` gone). **Open**: C1 (`crates/mzlab-target` still has no dependent, `liste-target-core.js` still the engine, `liste.js:637`), C5 (`tables.js:13` `massOf`), C6 (5 literal protons outside tests: `rg -n '1\.007276\b|1\.00727646' crates --type rust -g '!tests'`), C7 (`fn median*` grew from 46 to 53), C8 (`family/rel.rs:9` still builds its own tables), C9 (`picchi.js:6` `integ`, `fondo.js:13` `snipBaseline`), C10 (42 local `esc` helpers, 22 of them in `TP_Mine/js`). They stay valid as written; launch them before inventing new merges.

## 7. Decisions for Federico
1. **Delete `mzlab_feat::mrm` (588 lines) and its tests `lr_mrm_radi.rs`, `examples/nativo_mrm.rs`?** After 02 nothing in the site, the app or the CLI reaches it; the MRM path of mzFinder is `mzlab-finder-lr-wasm`. Deleting a tested module is not «disabling a test», but it removes a capability that is only latent today.
2. **`mzlab-target` crate** (C1): finish the port (Rust engine of HRMS Target Screening) or delete the crate; today both the crate and `liste-target-core.js` exist.
3. **A wasm-target `cargo check` per site feature set in CI** (§1.2) would keep the warnings at zero; it adds ~3-4 min of CPU to one job. Worth it?
