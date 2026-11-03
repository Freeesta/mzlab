# CLAUDE.md - mzLab (repository Freeesta/mzlab)

Rules for every chat that changes this repository. Area details live in `docs/agenti/`: find the file with `grep -n '<word>' docs/agenti/INDICE.md`, read only that one. Find code with `python3 tools/indice.py`, then `grep -n '<name>' docs/agenti/mappa.md`.

## What it is
Free LC-MS analysis program, in the browser only (https://freeesta.github.io/mzlab/, GitHub Pages from `main`) plus a downloadable desktop/CLI app. Teaching tool (low-resolution triple-quad data, ESI+) and research tool (HR non-targeted screening, feature tables, reproducible reports). Files never leave the user's computer.

## Layout
- `crates/`: Rust engine (`mzlab-core`, `mzlab-feat`, `mzlab-cli`, `mzlab-desktop` (Tauri v2), `*-wasm`). This is where logic lives.
- `mzlab/web/`: plain HTML/JS interface (classic scripts in `index.html` order, ES modules for new files; no bundler). Texts only in `lang/it.js` and `lang/en.js`.
- `mzlab/*.py`, `tools/golden.py`, `tools/*_riferimento.py`: Python reference for parity tests only.
- `tools/`: dev tools. `tests/` pytest, `tests_e2e/` Playwright (`lib.py`). `docs/agenti/`: area files (current state only).

## Commands
- Before pushing: `python3 tools/pre_pr.py` (fast round; in cloud it skips browser tests and exits 0; `--completo` forces all).
- Full round: `python3 tools/verifica.py` (`--cambiati` = e2e of changed files only). Test levels and known reds: `docs/agenti/test.md`.
- Tests that need the built site: `python3 tools/build_site.py` first (writes `site/`, delete it afterwards).
- Size and doc limits: `python3 tools/controlla.py`. Code map: `python3 tools/indice.py`.
- Generated files (schemas, `mappa.md`, `strumenti-1.json`, ...): regenerate with their tool, never by hand.

## Rules
- **Parity**: every function (Flow step, reader, converter, export, model, engine route) exists both in the site (`.wasm` + OPFS) and in the app (native desktop + CLI), with a byte-for-byte parity test. A PR delivering one half is incomplete unless a real technical limit is stated in the PR body.
- **All logic in Rust.** Python is only for tests, `tools/` and reference; JS is interface only. New modules get a CLI (`--set INSTANCE.NAME=VALUE` per step parameter; checklist in `docs/agenti/cli.md`).
- **Wasm caps**: a `.wasm` may grow by up to +10 % per new function; record before/after in `docs/agenti/wasm-tetti.md`. Never raise a cap by editing a workflow.
- **No real data in this public repo.** Lab data live in the private repo `Freeesta/mzlab-dati` (found by `tools/dati_percorsi.py`; without it `tools/dati_sintetici.py` makes stand-ins). Only the anonymous files in `mzlab/web/esempi/` are allowed. Surnames in the private `nomi_vietati.txt` never appear here.
- **English only** for code, comments, commit messages and PRs; user-facing text in both languages via `lang/` keys (`tools/controlla_i18n.py` enforces it). Tabui (`TP_Mine/`) is English-only, no `lang/` keys, and never the bare word «formula».
- **Never push to `main`.** Work on `claude/<topic>` from `origin/main`, open a PR (ready for review), never merge it yourself. Merge `main` into the branch only if the PR conflicts (`tools/fondi_main.py`); never rebase or force-push.
- **Never skip, disable or quarantine a test** to get green. Fix the cause.
- **Teaching principles**: in the low-resolution world the program does NOT give answers (no «here are the TPs», no calibration lines or report tables); an m/z is a candidate, not an identification. In the HR world, libraries/formulas/analogues are scored candidates with evidence, never truth.
- **Accessibility** (WCAG 2.2 AA): colours only from CSS variables, `prefers-reduced-motion`, keyboard and `aria-label`, colour-blind-safe (Okabe-Ito) charts; every new chart ships with its data table (`TDATI`), targets of at least 24x24 px.
- **Light**: no bundler or heavy dependency; the site never runs Python.
- **Comments**: brief, on the why; 2-4 line header per module; no commented-out code.
- **Docs** (`docs/agenti/`) describe the current state only: rewrite the line you change (no «now», «was», PR numbers, dates). A doc line is at most 600 characters, an area file at most 25 KB.
- **Front end not tried in a browser is broken.** Every interface change keeps the guided tour (`tour.js`, `tests_e2e/e2e_tour.py`) valid.
- Do not read vendored or generated bulk: `mzlab/web/vendor/`, `mzlab/web/elements.js`, `mzlab/web/esempi/`, `.git/`, `site/` (grep is fine).
