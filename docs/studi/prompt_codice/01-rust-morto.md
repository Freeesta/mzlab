# 01 — Dead Rust functions and the warnings of the site builds

Prompt ready for a DEV chat: copy everything after the «---» line. Wave 1 (with 02 and 03).

---

**Role.** DEV chat for mzLab (public repo Freeesta/mzlab). English for code, commits, PR and messages. Read CLAUDE.md (already in context) and `docs/studi/OTTIMIZZAZIONE_CODICE.md` §1.1 and §1.2 only. Branch `claude/rust-dead-code` from `origin/main`.

**Goal.** Delete Rust code that nothing reaches and make every site wasm build warning-free, with no output byte changed. Unused `pub fn` cost 0 wasm bytes (LTO drops them): the gain is lines and clarity, so do not touch anything that is reached, even only by a test.

**Files you own (touch nothing else).**
- `crates/mzlab-chem/src/mol3d.rs` (`to_xyz` :141)
- `crates/mzlab-cli/src/normative/sources.rs` (`resolve_params` :88), `crates/mzlab-cli/src/normative/table_rd.rs` (`cas_of` :289)
- `crates/mzlab-feat-table-wasm/src/pol.rs` (`recipe_polarity` :30, not an export)
- `crates/mzlab-feat/src/attributio/checks/mod.rs` (`campaign_fit` :328), `attributio/frame.rs` (`column_texts` :53), `attributio/review/raw.rs` (`find_raw_dir` :55), `interp/albero.rs` (`null_margins` :176), `trace.rs` (`with_centwave_width` :85)
- `crates/mzlab-lib/src/format.rs` (`unshuffle_peaks` :273), `libset.rs` (`set_sidecar` :171), `read.rs` (`index_entries` :644)
- `crates/mzlab-mol/src/descr.rs` (`crippen_mr` :145, `rotatable_bonds` :208)
- warnings: `crates/mzlab-core/src/etichette_picchi.rs` (:101-103), `crates/mzlab-core/src/wiff/lettore.rs` (:48-49, :159, :299-300), `crates/mzlab-feat/src/tp/mod.rs` (:22), `crates/mzlab-feat/src/series.rs` (:198-203), `crates/mzlab-feat/src/chunked.rs` (:4), `crates/mzlab-attributio-wasm/src/lib.rs` (:13)

NOT yours: `crates/mzlab-feat/src/mrm.rs` and every `#[wasm_bindgen]` export (prompt 02), `mzlab-core/src/{specpeaks,specdiff,mapgrid,mapcells}.rs` (prompt 05), any build script.

**Work.**
1. Before deleting each function, re-run `rg -w <name> crates mzlab flow TP_Mine tools tests tests_e2e fuzz` and delete only if the definition is the only hit. If a helper becomes unused after the deletion (a private fn, an import, a const), delete it too; if the module then has nothing left, remove the module and its `mod` line.
2. Warnings: check every site build with its exact feature set, e.g.
   `cargo check --target wasm32-unknown-unknown -p mzlab-wasm`, `-p mzlab-rotte-wasm`, `-p mzlab-attributio-wasm --features all|export|ms2|rev`, `-p mzlab-feat-wasm --no-default-features --features <the hr list of tools/build_feat_wasm.sh as on main>` and `--features flow_formula|flow_tab|untargeted|interp`, `-p mzlab-lettori-wasm --no-default-features --features <family>` for the 9 families of `tools/build_lettori_wasm.sh`, plus the other `*-wasm` crates with no feature. Target: 0 warnings in all of them and in `cargo check --workspace --exclude mzlab-desktop --all-features --all-targets`.
   - `etichette_picchi.rs:103`: drop `ys` from the destructuring and rewrite the comment of :101 to say what is true: the valleys are measured on the wide-smoothed trace (`wide`, :114). Do not change any computation.
   - `wiff/lettore.rs`: gate `bpc`, `bpc_seen` (and their initialisation) on `feature = "estesi"`, and the unused parameters of :159 the same way. The struct layout of the engine changes only in the non-`estesi` build; nothing reads those fields there.
   - `tp/mod.rs:22`, `chunked.rs:4`: gate the import on the cfg that uses it. `series.rs:202`: declare `workers` only where it is read.
   - `mzlab-attributio-wasm/src/lib.rs:13`: remove the unused `self`.
   Never use a crate-wide `#![allow(...)]`; a cfg on the exact item.
3. No new dependency, no `unsafe`, no change of public behaviour.

**Tests (all must pass unchanged).** `cargo test -p mzlab-core -p mzlab-feat -p mzlab-lib -p mzlab-mol -p mzlab-chem -p mzlab-cli -p mzlab-feat-table-wasm -p mzlab-attributio-wasm`; `cargo clippy --workspace --exclude mzlab-desktop --all-features -- -D warnings`; `python3 tools/pre_pr.py`. Proof of no output change: the parity/golden tests of those crates are untouched and green (do not edit any expected file); state in the PR that no file under `tests/` or `crates/*/tests/` changed.

**PR.** Title «Remove unreached Rust functions and the warnings of the wasm builds». Body starts `VISIBLE: no`, then the list of deleted items with their `rg` evidence (one line each), the before/after warning count per build, lines removed. Caps unchanged; sizes unchanged (say so: LTO already dropped the functions).

**Delivery.** At green CI ONE message to the COO (≤5 lines: PR number, 40-char SHA, lines removed, warnings 8 → 0). PR events: skill `steward`.
