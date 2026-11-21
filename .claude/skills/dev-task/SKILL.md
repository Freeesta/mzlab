---
name: dev-task
description: How an mzLab DEV chat starts and finishes one task, from reading CLAUDE.md to the ready-for-review PR and the one-line report. Read at the start of every DEV chat.
---
# One DEV chat, one task, one PR

**Start**
1. Read `CLAUDE.md`. Find the docs with `grep -n '<word>' docs/agenti/INDICE.md`, then read only that area file (offset/limit).
2. Find code with `python3 tools/indice.py` (writes `docs/agenti/mappa.md`), then `grep -n '<name>' docs/agenti/mappa.md`, or ask the `finder` agent. Never read `mzlab/web/vendor/`, `mzlab/web/elements.js`, `mzlab/web/esempi/`, `.git/`, `site/`.
3. `git fetch origin && git switch -c claude/<topic> origin/main`. Never work on or push to `main`.

**Work**
- Parity: every function exists in the site (`.wasm` + OPFS) and in the app (native + CLI) with a byte-for-byte parity test, or the PR body states the real technical limit.
- CLI for agents: every function has a CLI route (`--json` with stable keys, distinct exit codes, data on stdout and progress on stderr, `--dry-run` or check where sensible, listed in the agent catalogs; `docs/agenti/cli-agenti.md`).
- English only, no real data (use `tools/dati_sintetici.py`), user-facing text through `lang/` keys, generated files regenerated with their tool, never skip a test.

**Finish**
1. `python3 tools/pre_pr.py` (`--full` forces the browser tests). Changed e2e only: `python3 tools/verifica.py --changed`.
2. Merge `origin/main` only on a real conflict: `python3 tools/fondi_main.py`. Never rebase or force-push.
3. Push `claude/<topic>`, open the PR ready for review (not draft) filling `.github/pull_request_template.md`. Never merge it yourself; after opening it follow the `steward` skill.
4. Report in one line: PR number, head SHA, CI status, what remains.
