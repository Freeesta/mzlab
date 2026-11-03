# N03: one verb for the golden variables, `MZLAB_APP_DIR`, `verifica.py --mostra-ok`

You are a chat of mzLab (repository Freeesta/mzlab). Read CLAUDE.md, `docs/agenti/test.md` and `docs/studi/NOMI_TERMINALE.md` §2.5–2.6, §4. Branch `claude/env-names` from `origin/main`. English only.

**Goal.** «Rewrite the expected file» is always `MZLAB_UPDATE_<WHAT>`; the desktop app folder no longer looks like `MZLAB_DATI`.

**Do.** Exactly the `env` and `tools` rows of study §4, except `MZLAB_UPDATE_FIGURA` (N02). Readers check the new name first, then the old one (`var_os(new).or_else(|| var_os(old))`; Python `os.environ.get(new) or os.environ.get(old)`); the test's own comment and failure message name the new variable. `MZLAB_DATA` → `MZLAB_APP_DIR` in `crates/mzlab-desktop/examples/serve.rs` and the two `e2e_shim*.py` that set it. `tools/verifica.py`: `--mostra-ok` with `--tutto` as an argparse alias (`add_argument("--mostra-ok", "--tutto", dest=…)`), help text in Italian as the file is.

**Do not touch.** `MZLAB_DATI`, the build variables (`MZLAB_COMMIT`, `MZLAB_RUSTC`, `MZLAB_TARGET`, `MZLAB_CARGO_LOCK_SHA256`), `.github/workflows/` unless one sets an old name (then set the new one), `msn_figura_tests.rs`, `pre_pr.py`.

**Tests.** For each variable one check that the old name still triggers the rewrite path (run the test with the old name in a temporary copy, or a unit test of the small reader); `tests/test_verifica*.py` (if any) or a new pytest that `--tutto` and `--mostra-ok` parse to the same namespace. Golden files unchanged: `git diff --stat origin/main -- tests/golden crates/*/tests/data` empty. `python3 tools/pre_pr.py`.

**Docs.** Rewrite the lines naming these variables (`rete.md`, `mzfeat-prove.md`, `ci-veloce.md`, `rust-rotte.md`, `memoria.md`, `rust-desktop.md`, `test.md`).

**PR.** Title «Tests: one UPDATE verb for golden variables». Body starts with `VISIBLE: no`, old → new table, old names still work.
