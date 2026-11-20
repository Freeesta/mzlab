# Test-suite quality study (report only)

State studied: `origin/main` at 4e74ee9. Nothing in the tests or in the code was changed. The study is **partial** (time limit): what was not done is listed in "Unfinished" at the end, and every number below says what it was measured on.

Severity tags: **[H]** hides a real bug · **[W]** weak (passes, proves little) · **[C]** cost (time, noise, dead files).

## Method

- Greps over `tests/`, `tests_e2e/`, `crates/**/tests`, `tools/verifica.py`, `.github/workflows/`, plus the current-state docs `docs/agenti/test.md`, `docs/agenti/mutants.md`, `docs/agenti/parity.md`.
- Hand mutation: 30 one-line bugs in a scratch git worktree (never committed, removed afterwards). Rust mutants ran `cargo test -p mzlab-core --lib` and then the matching integration tests; JavaScript mutants ran only their pytest/node tests. `cargo mutants` is not installed and was not added.
- A mutant counts as killed only when a test that passes on the clean tree fails on the mutated one (the clean baseline was re-run for the JavaScript tests: 11 passed).

## 1. Tests that cannot fail

1. **[H]** `tests_e2e/e2e_sottrazione.py:44` — `assert line and min(line.values()) < 0 or True`: the negative part of a subtracted trace is never checked. Fix: drop `or True`.
2. **[H]** `tests_e2e/e2e_mappa_correzione.py:80` — `assert "stretch" in chip.lower() or "tract" in chip.lower() or True`: the correction chip may show anything. Fix: drop `or True` and assert the text for the active language.
3. **[H]** `tests_e2e/e2e_studenti.py:91` — `assert "uno spettro alla volta" in ... or True`; the script also needs lab data, so it never runs in CI at all. Fix: drop `or True` and give the script a synthetic input.
4. **[H]** `tests_e2e/e2e_rotte_lazy.py:23-27` — the `assert st >= 400` sits inside `try: ... except Exception: pass`, so its own `AssertionError` is swallowed: "a failed fetch of the module is an error" prints OK even when the call answers 200. Fix: move the assert out of the `try`, or catch only the rejected-request error.
5. **[W]** `tests_e2e/e2e_nome.py:36` — `assert pg.evaluate("plotMeta(...) ? true : true") or True`: two tautologies; it only proves `plotMeta` does not throw. Fix: assert the returned title.
6. **[W]** `tests_e2e/e2e_llm_consenso.py:101` — `assert any("trasporto.html" in u for u in seen) or True`: the page that must be requested is never required. Fix: drop `or True`.
7. **[W]** `tests_e2e/e2e_tabelle_dati.py:38` — `next(r for r in t["rows"] if "TIC" in r[0] or True)`: the filter is always true, the first row is taken whatever it is. Fix: drop `or True`.
8. **[W]** `tools/verifica.py:282-295` (`judge`) — a failure is an output line matching FAIL/Traceback or a non-zero exit; a `SKIP` line followed by `sys.exit(0)` is green. 89 e2e scripts have such a line (e.g. `tests_e2e/e2e_flow_web.py:17` and `:21`, cargo/wasm-bindgen/numpy missing). Fix: in CI make `judge` report a skip of a prerequisite that the job declares in `NEEDS` as a failure (same idea as `MZLAB_GOLDEN_STRICT`).
9. **[H]** `tests/test_interp_riferimento.py:10` — `pytest.importorskip("rdkit")` at module level: no workflow installs rdkit, so the independent reference for the interpreter never runs. Fix: install rdkit in the nightly job and make the skip an error there.
10. **[W]** `tests/test_xlsx.py:46` — `pytest.importorskip("openpyxl")`: the only check that an independent reader opens the written workbook never runs in CI. Fix: as above.
11. **[W]** `tests/test_sw_cache_node.py:12` — `skipif(shutil.which("node") is None)`: the only unit test of the service worker disappears silently on a runner without node; `tests/*.py` hold 184 `skip`/`skipif`/`importorskip` sites in total. Fix: one strict environment variable in CI that turns "tool missing" skips into errors.
12. **[W]** 15 `#[ignore]` Rust tests (e.g. `crates/mzlab-core/src/scantable/tests_window.rs:37`, `crates/mzlab-feat/tests/groups.rs:441`, `crates/mzlab-lettori/tests/sqlite_columns.rs:295`, `crates/mzlab-chem/tests/validation.rs:201`, five in `crates/mzlab-finder-lr-wasm/tests/`); no workflow passes `--ignored`, so they run only by hand. Fix: give each a reason string and run them in the nightly job.
13. **[H]** Lab data: 40 Rust test files and 21 Python test files return early or skip without the private data; CI has none. Behaviours therefore **never tested in CI** (from `docs/agenti/test.md` and `docs/agenti/mutants.md`):
    - the whole teaching flow (`e2e_studenti`), `e2e_hr_msntree`, `prova_hr`;
    - the Orbitrap cut steps of `e2e_hr_base` and `e2e_hr_ppm`;
    - every `.dam` path: `e2e9`, `e2e10`, `e2e15`, `e2e_dam_incoerente`;
    - reader internals that survive mutation without real files: wiff (`wiff_member`, `scan_header`, `add_unindexed`, `header_tic`), Waters (chromatograms, cuts, functions, `.inf`, MRM, sides), Shimadzu metadata, `mzxml_index`, parts of netCDF.
    Fix: one small synthetic file per format from `tools/dati_sintetici.py` (header + two scans is enough to kill the listed mutants).

## 2. Hand mutation of 30 important functions

| id | file | mutation | result |
|---|---|---|---|
| R01 | `crates/mzlab-core/src/elements.rs` | `parse_formula`: group multiplier `n * k` → `n + k` | killed (integration tests) |
| R02 | `crates/mzlab-core/src/elements.rs` | `round_half_up`: `>= 5` → `> 5` | killed (integration tests) |
| R03 | `crates/mzlab-core/src/adduct_table.rs` | electron mass term: sign flipped | killed (integration tests) |
| R04 | `crates/mzlab-core/src/adduct_table.rs` | ppm: divided by the observed mass, not the calculated one | **survived** |
| R05 | `crates/mzlab-core/src/isoscore.rs` | `isotope_score`: best observed peak picked with `<` instead of `>` | **survived** |
| R06 | `crates/mzlab-core/src/composition.rs` | mDa tolerance `/ 1000` → `/ 100` | killed (integration tests) |
| R07 | `crates/mzlab-core/src/kendrick.rs` | defect `nkm - km` → `km - nkm` | killed (integration tests) |
| R08 | `crates/mzlab-core/src/baseline.rs` | `integrate`: crossing triangle `/ 2` → `/ 3` | killed (lib unit tests) |
| R10 | `crates/mzlab-core/src/intwin.rs` | `move_end`: last index `io - 1` → `io` | **survived** |
| R11 | `crates/mzlab-core/src/valley.rs` | local maximum `>` → `>=` on the left side | **survived** |
| R12 | `crates/mzlab-core/src/manint.rs` | open interval `k > lo && k < hi` → closed | killed (lib unit tests) |
| R13 | `crates/mzlab-picchi/src/peaks.rs` | area: clipping of negative points removed | not run |
| R14 | `crates/mzlab-core/src/mzmlarray.rs` | `decode_array`: 32-bit integers read as unsigned | killed (lib unit tests) |
| R15 | `crates/mzlab-core/src/specpeaks.rs` | peak test `>=` left → `>` | **survived** |
| R16 | `crates/mzlab-lib/src/parse.rs` | MSP: new-record rule loses the name test | **survived** |
| R17 | `crates/mzlab-lib/src/parse.rs` | MGF: a broken block is emitted instead of dropped | killed (crate tests) |
| R18 | `crates/mzlab-core/src/samples/csv.rs` | separator sniff: `;` counted inside quotes too | not run |
| R19 | `crates/mzlab-core/src/mzmlhead/reading.rs` | `header_polarity`: `pos |=` → `pos =` | not run |
| R22 | `crates/mzlab-core/src/ole.rs` | mini-stream read one byte short | not run |
| D1 | `crates/mzlab-desktop/src/update.rs` | `validate_web`: size `&&` hash → `||` | not run |
| D2 | `crates/mzlab-desktop/src/update.rs` | `fetch_one`: size `&&` hash → `||` | not run |
| D3 | `crates/mzlab-desktop/src/update.rs` | `diff`: native files no longer excluded | not run |
| D4 | `crates/mzlab-desktop/src/store.rs` | `clear_files`: nothing removed | not run |
| J1 | `mzlab/web/reset.js` | cache wipe: mode "all" `||` prefix → `&&` | **survived** |
| J2 | `mzlab/web/reset.js` | service worker unregistered in mode "none" instead of "all" | **survived** |
| J4 | `mzlab/web/sw.js` | version mark written although a fetch failed | killed (node/pytest) |
| J5 | `mzlab/web/sw.js` | `activate` deletes the big cache too | killed (node/pytest) |
| J6 | `mzlab/web/sw.js` | a failed response is put in the cache | killed (node/pytest) |
| J7 | `mzlab/web/rust-carica.js` | copy gate: `run < k` → `run <= k` | killed (node/pytest) |
| J8 | `mzlab/web/rust-carica.js` | copy gate: queue order `rank > r` → `rank >= r` | **survived** |

Run: 22 of 30 · killed: 13 · survived: 9 (R04, R05, R10, R11, R15, R16, J1, J2, J8) · not run: 8 (R13, R18, R19, R22, D1, D2, D3, D4).

Reading of the survivors:

- **[H]** J1, J2 (`mzlab/web/reset.js`): with the wipe in mode "all" no cache is deleted, and the service worker is never unregistered, and `tests/test_boot_reset.py` + `tests/test_migrazione_qqq.py` stay green. `tests_e2e/e2e_reset.py` may catch them but needs the built site, so it is nightly-only and was not run here. Fix: a node test with a fake `caches` and a fake `navigator.serviceWorker`.
- **[H]** R05 (`isoscore.rs`): matching the weakest instead of the strongest observed peak in an isotope window survives the lib tests and `isotopes_si`, `composition_parity`, `composition_ions`: no test has two observed peaks competing for one theoretical isotope.
- **[H]** R10 (`intwin.rs`, `move_end`): the clamp that keeps a moved start edge one point before the other edge can go (`io - 1` → `io`, a zero-width window becomes reachable) and the `mzlab-core` lib tests stay green; `crates/mzlab-cli/src/peak_window_cmd_tests.rs` was not run against it.
- **[H]** R11 (`valley.rs`): a flat apex counted as two local maxima survives the lib tests and `peak_labels`, `peaks_base`, `peaks_golden`: no fixture has two equal consecutive points at a top.
- **[W]** R15 (`specpeaks.rs`): with `>` on both sides a flat-topped spectrum peak is lost, and the lib tests and `specpeaks_parity` stay green: same missing fixture on the spectrum side.
- **[H]** R16 (`crates/mzlab-lib/src/parse.rs`, MSP): without the name test two records are merged when the first has no peak list; every `mzlab-lib` test stays green.
- **[W]** J8 (`mzlab/web/rust-carica.js`, copy gate): `rank > r` → `rank >= r` survives: `tests/carica_worker_node.mjs:243-260` never queues two jobs of equal rank, so their arrival order is not pinned.
- **[W]** R04 (`crates/mzlab-core/src/adduct_table.rs`, ppm): dividing by the observed instead of the calculated mass survives; every tested error is a few ppm, where the two differ by about 1e-5 ppm. Fix: one case with a large error.
- **[W]** R01 and R03 were killed only by the integration tests (`chemistry_parity`, `adduct_table`); `cargo test -p mzlab-core --lib` stayed green with a wrong group multiplier and a wrong electron sign. The fast unit level does not guard formula parsing.
- **[C]** J4, J5, J6 are three different service-worker bugs killed by the same line, `tests/test_sw_cache_node.py:15` (`assert r.returncode == 0` on one node script): the kill is real, the message does not say which rule broke.

## 3. Parity gaps

1. **[H]** Parity "by same function": `docs/agenti/parity.md` lists tests (`parity_msn`, `natregistry`, `parity_masscal`, the `channels` unit tests) where both halves call the same Rust function through two entry points. They prove the wiring, not the result: a wrong formula passes on both sides. Fix: add one independent expected value (hand-computed or from the Python reference) per function.
2. **[H]** The Flow site↔app byte comparisons are e2e scripts that need the built site, so they run only in the nightly job; a pull request that breaks byte parity merges green. Fix: one small synthetic Flow byte comparison in the pull-request smoke set.
3. **[W]** `tests_e2e/e2e_normative_screen.py` and `tests_e2e/e2e_flow_web_tabella.py` exercise the site route and check shapes and a few values, with no byte comparison against the CLI. Fix: compare the route output with `mzlab-cli` output for the same synthetic input.
4. **[W]** Python↔Rust goldens use tolerances, not bytes: `crates/mzlab-core/tests/golden.rs:18-20` (1e-4, 1e-6, 1e-9) and `crates/mzlab-core/tests/chemistry_parity.rs:1` (1e-9). Reasonable for floats, but 1e-4 on an area is wide enough to let R08-class integration errors through on small peaks. Fix: state per field why the tolerance is needed; use exact equality for integers and strings.
5. **[W]** `tests/test_golden.py:11-18` runs `tools/golden.py --controlla --skip-missing`: a golden whose input is missing is skipped, not failed, and the goldens are written by the same Python reference they are checked against (circular when Python and Rust were ported from one another). `MZLAB_GOLDEN_STRICT` exists only in `rust.yml` and `desktop.yml`. Fix: drop `--skip-missing` for goldens whose inputs are in the repo.

## 4. Coverage holes

1. **[W]** 69 of 365 JavaScript files under `mzlab/web/` (vendor, `elements.js`, examples excluded) are never named in any test, `# covers:` line or e2e script. Fix: list them in `docs/agenti/test.md` and add a load-and-call smoke per file.
2. **[H]** Error paths of the vendor readers (section 1, item 13): no malformed or truncated synthetic file exists for wiff, Waters, Shimadzu, mzXML index, netCDF.
3. **[W]** Updater and local store (`crates/mzlab-desktop/src/update.rs`, `crates/mzlab-desktop/src/store.rs`), the peak area in `crates/mzlab-picchi/src/peaks.rs`, the CSV separator sniff, the mzML header polarity and the OLE mini-stream: mutants prepared but not run (see table), so their coverage is unknown, not proven.

## 5. Flakiness and cost

1. **[C]** 895 fixed waits (`wait_for_timeout`/`sleep`) in 169 e2e scripts (`tools/attese_base.json`); the worst: `e2e24` 34, `e2e_studenti` 28, `e2e4` 22, `e2e25`, `e2e_banco_barra`, `e2e_perdite` 19 each. Fix: replace with `wait_for_function` on the state the step waits for, starting from these six.
2. **[H]** `cargo test -p mzlab-core --lib` failed once in 17 identical runs on the clean tree; the failing test name was not captured. A unit test that depends on timing or on shared state hides real failures behind "re-run". Fix: loop the lib tests 100 times with `--test-threads` 1 and 16 and pin the name.
3. **[C]** Known load-flaky e2e: `e2e_tour`, `e2e_tocco`, `e2e_hr_scanbin`; `tools/noti_rossi.txt` holds `e2e_primo_disegno` as a known red. Fix: each needs an owner and a date in that file.
4. **[W]** Of 642 e2e scripts a pull request runs the smoke lists (16 + 7) and the safe set in `tools/ci_e2e_verdi.json` (264); 75 need the built site, 22 the wasm build, 4 `.dam` files, 2 real data. More than half of the e2e suite is nightly or never.
5. **[C]** 7 `except Exception:` + `pass` pairs in `tests_e2e/` besides item 1.4; each one should name the exception it expects.

## 6. Fixtures and goldens read by no test

1. **[C]** `tests/data/assist/copioni/`: the ten scripts T10 to T19 are named by no test or tool. Fix: wire them into the assistant test or delete them.
2. **[C]** `tests/data/nomi_tp/` (4 files): no reference in tests or tools. Fix: as above.
3. **[W]** `tests/golden/`: the six `esempio_*.json` and `sintetico_Differenza.json`, `sintetico_MRM.json`, `sintetico_MS2.json` are reached only through `tools/golden.py --controlla --skip-missing` (section 3, item 5): if their input goes missing they are skipped silently.

## Top 15 tests to add (ranked)

1. `reset.js`, mode "all": every Cache Storage key is deleted; mode "big": only keys starting with the big-cache prefix; other modes: none (node, fake `caches`). Kills J1.
2. `reset.js`: mode "all" calls `unregister()` on every service-worker registration; every other mode calls it zero times. Kills J2.
3. `e2e_rotte_lazy`: with the module route aborted, the call returns status ≥ 400 **or** raises the request error — asserted outside the `try`; a 200 fails.
4. Remove the six `or True` (section 1, items 1-3 and 5-7) and assert: minimum of the subtracted trace < 0; chip text equals the stretch label; panel text present; `plotMeta` title; transport page requested; the row whose first cell contains "TIC".
5. One synthetic Flow run in the pull-request smoke set: the site `.wasm` output bytes equal the `mzlab-cli` output bytes (SHA-256 of both).
6. `isotope_score`: two observed peaks inside one isotope window, intensities 10 and 100 → the 100 one is matched and the score equals the single-peak case. Kills R05.
7. Integration window: a start edge moved onto or past the end edge stops one point before it (start index ≤ end index − 1), and the mirror case for the end edge. Kills R10.
8. Flat apex: the series 0, 5, 5, 0 gives exactly one local maximum in `valley.rs` and exactly one peak in `specpeaks.rs`, at the documented index. Kills R11 and R15.
9. MSP: a record with no peak list followed by a second name line gives two records, each with its own name. Kills R16.
10. Mass: ppm for observed 1001.0 against calculated 1000.0 is exactly 1000.0 (not 999.0), and a unit test inside `elements.rs` parses `(CH2)3` to C3 H6 and `C(CH3)3` to C4 H9, so `--lib` alone guards them. Kills R04; R01 at unit level.
11. Copy gate: two queued jobs of equal rank start in arrival order; a later job of lower rank starts before both. Kills J8.
12. Strict skips: with a CI variable set, `tools/verifica.py` reports a script that prints SKIP for a prerequisite its `NEEDS` entry declares (unit test of `judge`: one problem expected), and the nightly job installs rdkit and openpyxl so `tests/test_interp_riferimento.py` and `test_xlsx_openpyxl_reads_it` report "passed", not "skipped".
13. Synthetic vendor files (wiff, Waters, Shimadzu, mzXML, netCDF), two scans each: scan count, first retention time, TIC of scan 0, and a truncated copy that must return the reader's error, not a panic.
14. Independent expected values for the "same function" parity tests: one hand-computed number per function (MSn tree, natural registry, mass calibration, channels).
15. Updater: a file with the right size and a wrong SHA-256 is rejected, and a wrong size with the right SHA-256 is rejected, in both `validate_web` and `fetch_one`; native files never appear in the web diff; `clear_files` leaves no listed key (mutants D1-D4 not run: confirm first).

## Unfinished

- Eight mutants are marked "not run" in the table: no result, neither killed nor survived. The desktop ones need a full `mzlab-desktop` build; the others were cut by the time limit. Survivors are counted on the 22 that ran.
- Each Rust mutant ran the `mzlab-core` (or `mzlab-lib`) lib tests and the integration tests named for it, not the whole workspace: a survivor may still be killed by a test of another crate (CLI, wasm) or by an e2e script.
- JavaScript mutants were judged on node/pytest tests only: the site build in the scratch worktree failed at `tools/build_lettori_wasm.sh` (cause not investigated), so no e2e script was run against a mutant. J1 and J2 may be killed by `tests_e2e/e2e_reset.py`.
- The flaky `mzlab-core` unit test (section 5, item 2) is not identified.
- The parity audit is a sample (the files named in section 3), not every route; `# covers:` lines were not cross-checked one by one.
