## What changes
<!-- one or two lines: the area touched and why -->

## Before the push
- [ ] Area: I read the area file (docs/agenti/INDICE.md) and touched only what my prompt says
- [ ] `origin/main` merged only on a real conflict (`tools/fondi_main.py`, never rebase); `python3 tools/pre_pr.py` green
- [ ] Parity: the function exists in the site (`.wasm` + OPFS) and in the app (native + CLI) with a byte-for-byte parity test, or the real technical limit is stated below
- [ ] CLI route for agents: `--json` with stable keys, distinct exit codes, data on stdout and progress on stderr, listed in the agent catalogs
- [ ] English only (code, comments, docs, tests, messages, commit and PR text); user-facing text through `lang/` keys
- [ ] No real data and no forbidden surnames; synthetic data only
- [ ] Generated files regenerated with their tool (never by hand)
- [ ] Risks: this PR does NOT touch Cargo.toml/Cargo.lock, `.github/workflows/` (except `test.yml` and `stat-ci.yml`), `.wasm` files, the CSP, `crates/mzlab-desktop/` — or I listed them below

## Risks
<!-- empty if none -->
