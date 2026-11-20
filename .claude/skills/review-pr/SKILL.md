---
name: review-pr
description: Review checklist for an mzLab PR from the CLAUDE.md rules, wrapping the built-in code-review skill.
---
# Reviewing an mzLab PR

1. Run the built-in `code-review` skill on the PR or diff for correctness bugs.
2. Run `python3 tools/controlla.py` (limits) and `python3 tools/controlla_i18n.py`; also `python3 tools/check_english.py --elenco` for Italian left in touched files.
3. Check against `CLAUDE.md`:
   - **Parity**: every function in the site (`.wasm` + OPFS) and in the app (native + CLI) with a byte-for-byte parity test; a missing half needs a real technical limit stated in the PR body.
   - **CLI for agents**: a CLI route exists, `--json` has stable keys, distinct exit codes, data on stdout and progress/errors on stderr, `--dry-run` or check where sensible, listed in the agent catalogs; the step has its row in `docs/agenti/cli.md`.
   - **English only**: code, comments, docs, test names, messages, commits, PR text; user-facing text through `mzlab/web/lang/en/` keys, never bare literals.
   - **No real data, no surnames** from the private `nomi_vietati.txt`; only anonymous files in `mzlab/web/esempi/`.
   - **Logic in Rust**; JS is interface only; no bundler or heavy dependency.
   - **No skipped, disabled or quarantined test.**
   - **Comments** brief and on the why; no commented-out code; a 2-4 line module header.
   - **Generated files** (schemas, `mappa.md`, `tools-1.json`, ...) match their tool's output, not hand-edited.
   - **Wasm caps** (`.claude/rules/rust.md`): at most +10 % per new function, before/after in the `misura` field of the `tools/moduli/*.toml`; no cap raised by editing a workflow or build script (`docs/agenti/wasm-caps.md`).
   - Risky paths listed in the PR body: `Cargo.toml`/`Cargo.lock`, `.github/workflows/`, `.wasm`, the CSP, `crates/mzlab-desktop/`.
4. Report findings as file:line with the rule broken; do not push fixes to someone else's branch and never merge.
