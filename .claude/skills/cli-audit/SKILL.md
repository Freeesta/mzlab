---
name: cli-audit
description: Audit mzlab-cli as an AI agent would use it: run the five documented chains on synthetic data and report gaps as a table.
---
# CLI audit

Act as an external agent; use only the CLI and its docs (`docs/agenti/cli-agenti.md`, `docs/agenti/for-agents.md`).

1. Install per `cli-agenti.md`: from source `cargo build --profile cli -p mzlab-cli` (binary in `target/cli/`), or the release binary for Linux.
2. Data: `python3 tools/dati_sintetici.py OUT_DIR` (synthetic mzML; no real data). For the other writers see `tools/dati_sintetici_*.py`.
3. Learn the CLI: `mzlab-cli commands --json` (catalog `commands/1`), `mzlab-cli flow steps --compact`, `mzlab-cli --version` (citation line).
4. Run the five chains of `for-agents.md` ("Six chains (tested)", chains 1-5), as implemented in `crates/mzlab-cli/src/workflow_tests.rs`: unknown folder to CSV, reproducible run, trace to peaks, statistics, vendor folder triage.
5. For every call check: exit code against the table in `cli-agenti.md` (0, 1 data, 2 usage, 3 plan/schema, 4 input missing, 5 selector, 6 output); data only on stdout and progress/errors on stderr; under `--json` stable keys and one `error/1` line on a non-zero exit; `--dry-run` or check writes nothing; `--help` exit 0.
6. Check every step of `mzlab-cli flow steps` has a row in `docs/agenti/cli.md` and every command in the `commands --json` catalog is documented.
7. Do not modify the repo. Report gaps as a table: command | expected | actual (with exit code and the stream involved). Say what you could not run.
