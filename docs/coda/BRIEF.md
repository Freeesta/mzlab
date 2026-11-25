# Shared brief of every queue prompt

Read `CLAUDE.md` and `.claude/skills/dev-task/SKILL.md` first. This file holds the rules that every roadmap task shares, without numbers that drift: for a live number run the tool named next to the rule.

## What you deliver
- One task, one PR on `claude/<topic>` from `origin/main`, ready for review, never merged by you. Report in one line: PR number, head SHA, CI status, what remains.
- Read before you build: `grep -n '<word>' docs/agenti/INDICE.md`, then only that area file. Code: `python3 tools/indice.py`, then grep `docs/agenti/mappa.md`. Verify against `origin/main` what already exists before you write code; a prompt can be older than main. If the task is done, blocked or obsolete, the PR says so and still removes its row (closing step below).

## Rules that never bend
- **Parity**: every function exists in the site (`.wasm` + OPFS), in the app and in the CLI, and a test compares the three byte for byte. The wasm leg of that test must FAIL, not skip, in CI when its tools are missing (skip only on a local machine; pattern: `tests/test_flowtab_union_wasm.py`). One half alone is incomplete unless the PR body states a real technical limit.
- **CLI for agents**: `--json` with stable keys, distinct exit codes pinned in a test, data on stdout and progress or errors on stderr, a schema, `--dry-run` or a check where sensible, listed in the agent catalogs (`docs/agenti/cli-agenti.md`, `docs/agenti/cli-catalog.md`). No CLI route means incomplete.
- **The user decides**: show the evidence (scores, errors, parameters) and log every automatic choice in the output. No disclaimers, no blocked results. Thresholds are `--set` parameters with their defaults printed.
- **Text**: English only, everywhere in the repo. User-facing text goes through `lang/` keys (`mzlab/web/lang/en/`), errors as `err.*` keys with a context line; never a bare literal.
- **Data**: synthetic only (`tools/dati_sintetici.py`, `MzmlWriter` as in `crates/mzlab-feat/tests/lr_fullscan.rs`). No real compound, sample or file names and no private numbers in this public repo; use generic words (parent compound, dark control, internal reference). If you need a private fact, ask the coordinator for counts or format facts only.
- **All logic in Rust**; JS is interface only; Python only for tests, `tools/` and references.
- **Cite mzLab** where output is meant for a report (`crates/mzlab-cli/src/citation.rs`).

- **How would an agent or user find this?** Every task ends by asking it. Put the entry point where they will look (error and output hints, the task table of `docs/agenti/for-agents.md`, `related.rs` task words and `see_also`, a help example) and test it (pattern: `crates/mzlab-cli/src/catalog/find_tests.rs`; `commands --find WORDS --json`). If the thing you were asked for already exists on main, do not close the task as done: it becomes a discoverability task (why was it not reached; fix the cause).

## Guards are never raised
Run `python3 tools/controlla.py` for the live numbers and fix the cause when one fails. The tool enforces them; they cover the size of the compact `commands --json` and of `flow steps --compact`, the size of every doc, the INDICE rows (one per area file, short, sorted), the length of subcommand names, and the size and line limits of code.

## `.wasm` size
A module cap changes only by the size the CI measures, and only inside the per-PR growth rule of `docs/agenti/wasm-caps.md`. Put module, raw bytes before, after and % in the PR body. Growth over that rule needs the owner: new site code goes into a module with room or into a new lazy module. The `rotte` module is nearly full: do not grow it.

## Process
- Never skip, disable or quarantine a test; fix the cause. Regenerate generated files (schemas, `mappa.md`, `tools-1.json`, help snapshots) with their tool, never by hand.
- Before pushing: `python3 tools/pre_pr.py`. Changed e2e only: `python3 tools/verifica.py --changed`.
- Merge `main` in only on a real conflict (`python3 tools/fondi_main.py`); never rebase or force-push.

## Closing step (part of the same PR)
The task came from `docs/coda/CODA.md`. In your PR: delete your row and `docs/coda/prompt/<id>.md`; delete your id (and `chat:<your id>`, for a chat started with an inline prompt) from the `needs first` cell of other rows; for each concrete follow-up named in your PR body add one row and one prompt file (start it with the line `Read docs/coda/BRIEF.md.`, with the sections of the other prompts, inside the size cap that `controlla_coda.py` checks). Run `python3 tools/controlla_coda.py`. If your task is blocked or obsolete, say so in the PR body and still remove the row. Follow-ups that change existing output bytes without an opt-in carry `needs owner decision: <what>` in the `needs first` cell.
