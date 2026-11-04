# PERF-04: why `hrp` and `hr15` never open in the site (diagnose, then fix)

You are a chat of mzLab (repository Freeesta/mzlab). Read CLAUDE.md, `docs/studi/OTTIMIZZAZIONE_PRESTAZIONI.md` §1-4, `docs/agenti/visore-veloce.md` (cases, «Non misurati»), `docs/agenti/memoria.md`, `docs/agenti/caricamento-mappa.md`, `docs/agenti/mappa-fluida.md` (D1, D8). Branch `claude/perf-hrp-hr15` from `origin/main`, after wave 1 (PERF-01..03) is in `main`. English for code, commits and PR.

**Fact.** `tools/bench_visore.py` has two cases that never open in the site:
- `hrp`: Orbitrap profile, 400 scans × 50 000 points, ~320 MB mzML. Files open, but the TIC and spectrum panels do not appear within 25 min.
- `hr15`: 15 `hrc` files of ~150 MB. They do not open within 27 min.

Natively the same `hrp` file (`bench_mappa.write_file(..., 'hrp', 400, 50000, 4)`) prepares in 2.2 s and builds its `.mzidx` in 1.7 s (240 MB, one pass) on 4 vCPU. So the engine's compute is not the cause. Suspects, in order: the 192 MB budget and spill path (`error.memoryBudget`, OPFS spill); a route the engine does not serve (`err.route.unavailable`, `window.MZLAB_LR_MISS`) for single-precision profile sections, where `profile_level` needs `prime_steps` from the mzML (`profspec.rs:55-70`); a worker that dies and restarts (`error.engineFallback`); OPFS quota of the headless profile; a JS loop over 20 M points on the main thread.

**Do.**
1. Build the site (`python3 tools/build_site.py`, wasm as in `.github/workflows/pages.yml`). Run `python3 tools/bench_visore.py hrp` and then `hr15` with a per-phase trace. Log the worker messages, `MZLAB_LR_MISS`, console errors, `performance.memory`, the engine's memory, and the timestamps of each phase of `caricamento-mappa.md` (table, `.mzidx`, `lrOpen`, `lr_entry`, first TIC, first spectrum). Cap each case at 10 min.
2. Name the blocking phase with evidence: file:line, the log line, the time, the memory. If the cause is a cap (memory budget, `MAX_TOTAL`, wasm memory) **do not raise it**. Write the measured need in the PR for Federico and stop at the diagnosis.
3. If the cause is a bug or an avoidable cost (a missing route, a lost `prime_steps`, a quadratic loop, an unneeded copy), fix it in the smallest zone. Both halves of the engine (site `.wasm` and desktop native) get the same fix. Add a parity test if a route changes.
4. Re-run both cases and fill the two «not measured» columns of the table in `visore-veloce.md` (G1-G6, M).

**Zone.** `tools/bench_visore.py` (tracing only), `docs/agenti/visore-veloce.md` (you own it in wave 2), and ONLY the files the diagnosis names. List them in the first comment of the PR before you change them. Do not touch `impronta*.js` or `rust-nativo.js` (PERF-05), `Cargo.toml` profiles, the build scripts, `docs/schemi/steps-cli-1.json`, `F.liveJobs`. Rows for `misure-velocita.md` / `ottimizzazioni-tentate.md` go in the PR body; PERF-05 copies them.

**Identity test.** Whatever you change must keep the output of every existing route byte-identical on the files where it works today. Show this with the existing parity tests: `cargo test --release -p mzlab-core`, `mzidx_parity.rs`, `golden.rs`, `e2e_mzidx`, `e2e_hr_xic`, `e2e_xic_spettro_tipi`. Also add a test that would have failed before your fix: native or e2e, on a reduced synthetic `hrp` (e.g. 60 × 50 000) so that it runs in CI time.

**Rules.** No cap raised, no new dependency, no `unsafe`, deterministic results. `python3 tools/pre_pr.py` before pushing. If the diagnosis takes the whole session, deliver it as a measure-only PR with the evidence and a next prompt.

**PR.** Title `Open large HR profile files and 15-file HR series in the site` (or `Diagnose why large HR profile files do not open in the site` for a measure-only PR). Body starts with `VISIBLE: yes - large Orbitrap profile files open in the browser` (or `VISIBLE: no`). Ready for review. Send the Coordinator: PR number, 40-char SHA at green CI, the blocking phase in one line, and any cap Federico must decide on.
