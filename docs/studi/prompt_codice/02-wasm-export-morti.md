# 02 — wasm exports nobody calls (the MRM path of the `hr` module and two small ones)

Prompt ready for a DEV chat: copy everything after the «---» line. Wave 1 (with 01 and 03). You own `tools/build_feat_wasm.sh` for this wave.

---

**Role.** DEV chat for mzLab (public repo Freeesta/mzlab). English for code, commits, PR and messages. Read AGENTS.md (in context), `docs/agenti/wasm-tetti.md` and `docs/studi/OTTIMIZZAZIONE_CODICE.md` §1.1 and §2.2. Branch `claude/wasm-dead-exports` from `origin/main`.

**Goal.** Every `#[wasm_bindgen]` export is a root the linker keeps with everything it calls. Remove the exports no page, worker or desktop file calls, so the modules shrink with no behaviour change. Measured in the study: building `hr` without `mrm` removes 31 583 B of code before wasm-opt (twiggy), est. −22…30 KB on the site file.

**Evidence (re-check before deleting).**
- `crates/mzlab-feat-wasm/src/lib.rs:289-340` `detect_mrm_file`, `align_mrm_series` and their helper `mrm_params` (:277-287), all under feature `mrm`: `rg -n 'detect_mrm_file|align_mrm_series|load\("mrm"\)|mods\.mrm' mzlab flow TP_Mine tests tests_e2e tools crates/mzlab-desktop` finds only the definitions and a comment of `Cargo.toml`. The MRM path of mzFinder in the browser is `mzlab-finder-lr-wasm` (`finder_lr_mrm`, `TP_Mine/files/finder-lr.js:113`).
- `crates/mzlab-lettori-wasm/src/lettura.rs:192` `estensioni`: no JS caller (the page lists extensions itself; only prose in `lang/contesto.json` matches). Compiled into all 9 `wasm-lettori/*`.
- `crates/mzlab-lib-wasm/src/lib.rs:65` `detect_format`: no JS caller; the importer calls `mzlab_lib::format::detect` itself.
- `crates/mzlab-feat/src/mrm.rs:286` `qualifier_ratios` and `:333` `csv_calibration`: no caller and no test anywhere (a calibration table is also against principle 1 for the LR world).
- Do NOT touch `finder_identity_graph`, `finder_polarity_pairs`, `finder_origin_evidence` (`finder_edge.rs`): `TP_Mine/files/tpmine-worker.js:288` calls them as `f["finder_" + fn]`. Do NOT touch exports used by node parity tests (`detect_file`, `align_series`, `build_evidence`, `memory_bytes`, `table_check`, `rt_descriptors`, `normative_listings`).

**Files you own.** `crates/mzlab-feat-wasm/src/lib.rs`, `crates/mzlab-feat-wasm/Cargo.toml`, `crates/mzlab-feat/src/mrm.rs`, `crates/mzlab-lettori-wasm/src/lettura.rs`, `crates/mzlab-lib-wasm/src/lib.rs`, `tools/build_feat_wasm.sh`, and `docs/agenti/wasm-tetti.md` / `docs/agenti/mzfeat-wasm.md` lines that name the `mrm` feature. Nothing else (prompt 01 owns the other dead functions and the warnings).

**Work.**
1. Delete the two MRM exports and `mrm_params`. Then the cargo feature `mrm` gates nothing: remove it from `[features]` (and from `default`), from the `cfg(any(...))` list at `lib.rs:7-13`, from the feature list of the `hr` build (`build_feat_wasm.sh:31`) and of the separate build (`:36-39`, `MZLAB_WASM_SEPARATI`), and from the comments that list the group. If the manifest of the HR group names a key `mrm`, keep the manifest format but drop the key only if no worker reads it (prove with `rg`); if unsure, keep the key and say so in the PR.
2. Keep `mzlab_feat::mrm` (its tests `crates/mzlab-feat/tests/lr_mrm_radi.rs` stay green and unchanged); delete only `qualifier_ratios` and `csv_calibration` and whatever becomes unused with them (`rg` each). `crates/mzlab-feat-wasm/examples/nativo_mrm.rs` uses `read_srm`: keep it compiling. Whether to delete the whole module is Federico's decision (study §7.1): do not do it.
3. Delete `estensioni` and `detect_format` (and any import that becomes unused).
4. Measure: build `hr`, one reader module (e.g. `sciex`) and `wasm-lib` before and after with the site scripts (`tools/build_feat_wasm.sh`, `tools/build_lettori_wasm.sh`, then `tools/ci_wasm_dimensioni.py site/static`); put the table (raw and gzip) in the PR. Caps (`limit_*`) are never raised; if `hr` drops a lot, propose the lower cap in the PR text only (decision of the Coordinator).
5. App parity: `rg -n 'detect_mrm|align_mrm|estensioni|detect_format' crates/mzlab-desktop crates/mzlab-cli` must stay empty (no native twin is lost; nothing in the app called them either).

**Tests.** `cargo test -p mzlab-feat -p mzlab-feat-wasm -p mzlab-lettori-wasm -p mzlab-lib-wasm`; the node parity tests of the feat wasm (`tests/feat_wasm_node.js` via its pytest, `tests/test_feat_pool_budget.py`); `python3 tools/pre_pr.py`; the e2e of mzFinder MRM and readers run by CI (`e2e_tpmine*`, `e2e_lettori*`) must stay green. Never skip, disable or loosen a test.

**PR.** Title «Drop wasm exports nobody calls (MRM in hr, estensioni, detect_format)». Body starts `VISIBLE: no`, then the evidence lines, the size table, the note on the `mrm` manifest key.

**Delivery.** At green CI ONE message to the COO (≤5 lines: PR, 40-char SHA, bytes saved on `hr` and readers). PR events: skill `steward`.
