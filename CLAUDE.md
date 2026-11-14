# CLAUDE.md - mzLab (Freeesta/mzlab)

Professional free LC-MS analysis program: in the browser only (https://freeesta.github.io/mzlab/, Pages from `main`) plus a desktop/CLI app. Low-resolution triple-quad (ESI+) and HR non-targeted screening, feature tables, reproducible reports. Files never leave the user's computer. The `qqq` lab site is the only teaching edition. Area rules load by path from `.claude/rules/` (rust, frontend, docs).

## Find things (read narrowly)
- Docs: `grep -n '<word>' docs/agenti/INDICE.md`, then read only that area file (line ranges, `Read` offset/limit).
- Code: `python3 tools/indice.py` first (it writes `docs/agenti/mappa.md`, which is gitignored and absent in a fresh clone), then `grep -n '<name>' docs/agenti/mappa.md`. Lookups: the `cercatore` agent.
- Never read `mzlab/web/vendor/`, `mzlab/web/elements.js`, `mzlab/web/esempi/`, `.git/`, `site/` (grep is fine).

## Layout
`crates/` Rust engine (all logic: `mzlab-core`, `mzlab-feat`, `mzlab-cli`, `mzlab-desktop`, `*-wasm`) · `mzlab/web/` plain HTML/JS interface · `mzlab/*.py`, `tools/golden.py`, `tools/*_reference.py` Python reference for parity tests only · `tools/` dev tools · `tests/` pytest · `tests_e2e/` Playwright · `docs/agenti/` current-state docs.

## Commands
- Before pushing: `python3 tools/pre_pr.py` (fast; in cloud skips browser tests; `--completo` forces all). Changed e2e only: `python3 tools/verifica.py --cambiati`; full round once: `python3 tools/verifica.py`. Levels and known reds: `docs/agenti/test.md`.
- Tests needing the site: `python3 tools/build_site.py` first, delete `site/` afterwards.
- Limits: `python3 tools/controlla.py`; i18n: `python3 tools/controlla_i18n.py`.
- Generated files (schemas, `mappa.md`, `strumenti-1.json`, ...): regenerate with their tool, never by hand.

## Rules
- **Parity**: every function (Flow step, reader, converter, export, model, engine route) exists in the site (`.wasm` + OPFS) and in the app (native + CLI) with a byte-for-byte parity test; one half alone is incomplete unless a real technical limit is stated in the PR body.
- **All logic in Rust.** Python only for tests, `tools/`, reference; JS is interface only; the site never runs Python.
- **No real data in this public repo.** Lab data live in private `Freeesta/mzlab-dati` (`tools/dati_percorsi.py`; else `tools/dati_sintetici.py`). Only the anonymous `mzlab/web/esempi/` files are allowed. Surnames in the private `nomi_vietati.txt` never appear here.
- **English only** for code, comments, commits, PRs. User-facing text in both languages via `lang/` keys.
- **Git**: never push to `main`; work on `claude/<topic>` from `origin/main`, open the PR ready for review, never merge it yourself; merge `main` in only on a real conflict (`tools/fondi_main.py`); never rebase or force-push.
- **Never skip, disable or quarantine a test.** Fix the cause.
- **The user decides**: show the evidence (scores, errors, parameters) and log every automatic choice; no disclaimers, no blocked results.
- **Comments**: brief, on the why; 2-4 line header per module; no commented-out code.
- **Light**: no bundler or heavy dependency.

## Chats
One chat, one task, one PR; after the merge stop (no scheduled check-ins, no polling). Report in one line: PR number, head SHA, CI status, what remains.
